//! The draw is a pure function of its inputs: the same `Inputs` give the
//! same system in one process. Across runs and platforms, the digests of
//! the reference systems are pinned in tier 2
//! (`crates/regression/tests/determinism.rs`).

use std::collections::BTreeMap;

use science::units::{Dex, SolarMass};
use system::{ArchetypeId, Drawn, INPUTS_SCHEMA, Inputs, Request, SystemRequest, draw_with};
use timeline::{Code, Seed, Trace, fnv1a64};

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
    draw_with(inputs, &mut Trace::new(), system::stand_in::classify).unwrap()
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
