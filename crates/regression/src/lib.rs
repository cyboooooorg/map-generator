//! Tier-2 regression tests (see CLAUDE.md, Tests): shared reference
//! systems generated once per run on a coarse grid, then statistics,
//! contracts and determinism asserted on them.
//!
//! Test-only crate: it may depend on every engine, which a domain crate
//! may not, even as a dev-dependency.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use science::units::{Dex, SolarMass};
use system::planet::class_of;
use system::{ArchetypeId, Drawn, INPUTS_SCHEMA, Inputs, Request, SystemRequest, draw_with};
use timeline::{Code, Seed, Trace, fnv1a64};

/// Seeds of the shared reference systems; filled in with slice 1, PR 2.
pub const REFERENCE_SEEDS: &[u64] = &[];

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
/// the planet model's [`class_of`].
///
/// # Panics
///
/// When a reference request fails to draw.
pub fn reference_systems() -> &'static [Drawn] {
    static SYSTEMS: OnceLock<Vec<Drawn>> = OnceLock::new();
    SYSTEMS.get_or_init(|| {
        reference_requests()
            .iter()
            .map(|request| draw_with(request, &mut Trace::new(), class_of).unwrap())
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
