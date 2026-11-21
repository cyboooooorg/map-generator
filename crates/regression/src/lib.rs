//! Tier-2 regression tests (see CLAUDE.md, Tests): shared reference
//! systems generated once per run on a coarse grid, then statistics,
//! contracts and determinism asserted on them.
//!
//! Test-only crate: it may depend on every engine, which a domain crate
//! may not, even as a dev-dependency.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use science::units::{Dex, EarthRadius, KilogramPerCubicMetre, SolarMass};
use system::{
    ArchetypeId, Drawn, INPUTS_SCHEMA, Inputs, Request, SystemError, SystemRequest, Trial,
    TrialOutcome, draw_with,
};
use timeline::{Code, PlanetClass, Seed, Trace, fnv1a64};

/// Seeds of the shared reference systems; filled in with slice 1, PR 2.
pub const REFERENCE_SEEDS: &[u64] = &[];

/// Source: the mean density of the Earth, 5 514 kg/m³ (NASA Earth fact
/// sheet), to turn a stand-in density into Earth radii.
const EARTH_DENSITY: f64 = 5514.0;

/// A stand-in for the planet model until mapgen1's lands: below 2 M⊕
/// rocky, below 10 a super-Earth, below 30 a mini-Neptune, else a gas
/// giant (a hot Jupiter inside 0.1 au); small planets of radius M^0.27,
/// giants of density 1 300 kg/m³.
///
/// # Errors
///
/// Never: the signature is the one `draw_with` takes.
pub fn stand_in_model(trial: &Trial, _: &Trace) -> Result<TrialOutcome, SystemError> {
    let mass = trial.mass.value().0;
    let a = trial.orbit.semi_major_axis.value().0;
    let class = match mass {
        m if m < 2.0 => PlanetClass::Rocky,
        m if m < 10.0 => PlanetClass::SuperEarth,
        m if m < 30.0 => PlanetClass::MiniNeptune,
        _ if a < 0.1 => PlanetClass::HotJupiter,
        _ => PlanetClass::GasGiant,
    };
    let (radius, density) = if mass < 30.0 {
        (mass.powf(0.27), 5500.0)
    } else {
        ((mass * EARTH_DENSITY / 1300.0).cbrt(), 1300.0)
    };
    Ok(TrialOutcome {
        class,
        radius: EarthRadius(radius),
        mean_density: KilogramPerCubicMetre(density),
    })
}

fn inputs(seed: u64, host: Option<f64>, archetypes: &[(&'static str, u32)]) -> Inputs {
    Inputs {
        schema: INPUTS_SCHEMA,
        seed: Seed::new(seed),
        present: None,
        request: Request::System(SystemRequest {
            host_mass: host.map(SolarMass),
            metallicity: Some(Dex(0.0)),
            archetypes: archetypes
                .iter()
                .map(|&(name, count)| (ArchetypeId(Code::of(name)), count))
                .collect::<BTreeMap<_, _>>(),
        }),
    }
}

/// The reference system requests: three free seeds, one per forced host
/// mass (0.3, 1 and 2 M☉), an `earth-like` and two `hot-neptune`.
#[must_use]
pub fn reference_requests() -> Vec<Inputs> {
    vec![
        inputs(1, None, &[]),
        inputs(2, None, &[]),
        inputs(3, None, &[]),
        inputs(4, Some(0.3), &[]),
        inputs(5, Some(1.0), &[]),
        inputs(6, Some(2.0), &[]),
        inputs(7, None, &[("earth-like", 1)]),
        inputs(8, None, &[("hot-neptune", 2)]),
    ]
}

/// The systems of [`reference_requests`], drawn once per run with
/// [`stand_in_model`].
///
/// # Panics
///
/// When a reference request fails to draw.
pub fn reference_systems() -> &'static [Drawn] {
    static SYSTEMS: OnceLock<Vec<Drawn>> = OnceLock::new();
    SYSTEMS.get_or_init(|| {
        reference_requests()
            .iter()
            .map(|request| draw_with(request, &mut Trace::new(), stand_in_model).unwrap())
            .collect()
    })
}

fn real(value: f64) -> String {
    format!("{value:.8e}")
}

/// A canonical text of every value of a drawn system, hashed: its
/// designation, family, star and age, then per planet its mass, orbit,
/// spin and fluxes, each real written with 9 significant digits.
#[must_use]
pub fn digest(drawn: &Drawn) -> u64 {
    let star = &drawn.star;
    let mut text = format!(
        "{} {} {} {} {} {} {} {} {}",
        drawn.designation,
        drawn.family.code(),
        real(star.mass.value().0),
        real(star.radius.value().0),
        real(star.luminosity.value().0),
        real(star.effective_temperature.value().0),
        real(star.metallicity.value().0),
        real(drawn.age.value().0),
        drawn.placements.len(),
    );
    for placement in &drawn.placements {
        let orbit = &placement.trial.orbit;
        let source = placement.trial.sources.primary();
        text += &format!(
            "\n{} {} {} {} {} {} {} {} {} {}",
            placement.designation,
            real(placement.trial.mass.value().0),
            real(orbit.semi_major_axis.value().0),
            real(orbit.eccentricity.value().get()),
            real(orbit.period.value().0),
            real(orbit.perihelion_longitude.value().0),
            real(placement.spin.rotation_period.value().0),
            real(placement.spin.obliquity.value().0),
            real(source.flux.value().0),
            real(source.xuv.flux.value().0),
        );
    }
    fnv1a64(text.as_bytes())
}
