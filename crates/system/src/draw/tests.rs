use std::collections::BTreeMap;

use super::*;
use crate::inputs::{BodyRequest, INPUTS_SCHEMA, SystemRequest};
use science::units::{Dex, SolarMass};
use timeline::Trace;

fn refuse(_: &Trial, _: &Trace) -> Result<TrialOutcome, SystemError> {
    unreachable!("no trial is classified yet")
}

fn system_inputs(seed: u64) -> Inputs {
    Inputs {
        schema: INPUTS_SCHEMA,
        seed: Seed::new(seed),
        present: None,
        request: Request::System(SystemRequest {
            host_mass: None,
            metallicity: None,
            archetypes: BTreeMap::new(),
        }),
    }
}

#[test]
fn a_drawn_system_carries_its_star() {
    let inputs = Inputs {
        present: Some(Gyr(4.57)),
        request: Request::System(SystemRequest {
            host_mass: Some(SolarMass(1.0)),
            metallicity: Some(Dex(0.0)),
            archetypes: BTreeMap::new(),
        }),
        ..system_inputs(7)
    };
    let drawn = draw_with(&inputs, &mut Trace::new(), refuse).unwrap();
    let expected = star::star(
        Some(SolarMass(1.0)),
        Some(Dex(0.0)),
        Some(Gyr(4.57)),
        Seed::new(7),
        &mut Trace::new(),
    )
    .unwrap();
    assert_eq!(drawn.star.mass.value(), expected.state.mass.value());
    assert_eq!(drawn.star.radius.value(), expected.state.radius.value());
    assert_eq!(
        drawn.star.luminosity.value(),
        expected.state.luminosity.value()
    );
    assert_eq!(drawn.age.value(), expected.age.value());
    assert!(drawn.placements.is_empty());
    assert_eq!(drawn.seed, Seed::new(7));
}

#[test]
fn the_system_number_is_in_range() {
    for seed in 0..1000 {
        let drawn = draw_with(&system_inputs(seed), &mut Trace::new(), refuse).unwrap();
        let number = drawn.designation.system.get();
        assert!((10_000..=99_999).contains(&number), "{number}");
    }
}

#[test]
fn a_body_request_is_refused() {
    let inputs = Inputs {
        request: Request::Body(BodyRequest {
            host_mass: None,
            metallicity: None,
            archetype: None,
        }),
        ..system_inputs(1)
    };
    assert_eq!(
        draw_with(&inputs, &mut Trace::new(), refuse).unwrap_err(),
        SystemError::BodyRequestNotDrawn
    );
}
