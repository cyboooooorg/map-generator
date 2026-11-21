//! The draw is a pure function of its inputs: the same `Inputs` give the
//! same system, in one process and across runs (`golden_digests`).

use std::collections::BTreeMap;

use science::units::{Dex, EarthRadius, KilogramPerCubicMetre, SolarMass};
use system::{
    ArchetypeId, Drawn, INPUTS_SCHEMA, Inputs, Request, SystemError, SystemRequest, Trial,
    TrialOutcome, draw_with,
};
use timeline::{Code, PlanetClass, Seed, Trace, fnv1a64};

/// Source: the mean density of the Earth, 5 514 kg/m³ (NASA Earth fact
/// sheet), to turn a test density into Earth radii.
const EARTH_DENSITY: f64 = 5514.0;

/// A stand-in for the planet model: below 2 M⊕ rocky, below 10 a
/// super-Earth, below 30 a mini-Neptune, else a gas giant (a hot Jupiter
/// inside 0.1 au); small planets of radius M^0.27, giants of density
/// 1 300 kg/m³.
fn model(trial: &Trial, _: &Trace) -> Result<TrialOutcome, SystemError> {
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

fn inputs(seed: u64, host: Option<f64>, archetypes: &[(&str, u32)]) -> Inputs {
    Inputs {
        schema: INPUTS_SCHEMA,
        seed: Seed::new(seed),
        present: None,
        request: Request::System(SystemRequest {
            host_mass: host.map(SolarMass),
            metallicity: Some(Dex(0.0)),
            archetypes: archetypes
                .iter()
                .map(|&(name, count)| (ArchetypeId(Code::new(name.to_owned()).unwrap()), count))
                .collect::<BTreeMap<_, _>>(),
        }),
    }
}

fn real(value: f64) -> String {
    format!("{value:.8e}")
}

/// A canonical text of every value of a drawn system, hashed.
fn digest(drawn: &Drawn) -> u64 {
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

fn draw(inputs: &Inputs) -> Drawn {
    draw_with(inputs, &mut Trace::new(), model).unwrap()
}

/// Eight fixed requests: three free seeds, one per forced host class, an
/// `earth-like` and two `hot-neptune`.
fn goldens() -> Vec<Inputs> {
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

#[test]
fn the_same_inputs_give_the_same_digest() {
    for request in goldens() {
        assert_eq!(digest(&draw(&request)), digest(&draw(&request)));
    }
}

/// The digests of [`goldens`]. Ignored until pinned: CI runs on
/// `ubuntu-latest` and the libm results may differ from macOS by an ulp,
/// enough to flip a threshold. Read the Linux values from the CI log of
/// this test run with `--ignored --nocapture`, paste them in `EXPECTED`
/// and drop the `#[ignore]`.
#[test]
#[ignore = "digests not yet pinned from the Linux CI run"]
fn golden_digests() {
    // macOS values, to replace with the Linux CI ones.
    const EXPECTED: [u64; 8] = [
        13710157267424814265,
        16494581978402232220,
        12784802251799961512,
        6748074216206917634,
        8644050250174416647,
        8845664612222415756,
        12078261093261751040,
        13410322849457797753,
    ];
    let digests: Vec<u64> = goldens()
        .iter()
        .map(|request| digest(&draw(request)))
        .collect();
    println!("golden digests: {digests:?}");
    assert_eq!(digests, EXPECTED);
}
