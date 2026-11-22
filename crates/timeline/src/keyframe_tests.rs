use super::*;
use crate::fixtures::{earth_like_fixture, frac, interior, leaf};
use crate::{Code, Gas, NodeValueOf, PlanetClass, RadialSample, RadialSamples, Renewal, StarPhase};
use science::units::{
    Dex, Kelvin, KilogramPerCubicMetre, Metre, Pascal, SolarLuminosity, SolarMass, SolarRadius,
};

fn star(trace: &mut Trace) -> StarState {
    StarState {
        phase: StarPhase::MainSequence,
        mass: leaf(trace, "star.mass", SolarMass(1.0)),
        radius: leaf(trace, "star.radius", SolarRadius(1.0)),
        luminosity: leaf(trace, "star.luminosity", SolarLuminosity(1.0)),
        effective_temperature: leaf(trace, "star.teff", Kelvin(5772.0)),
        metallicity: leaf(trace, "star.metallicity", Dex(0.0)),
        interior: None,
    }
}

fn planet_keyframe(date: f64) -> Keyframe {
    let mut trace = Trace::new();
    let state = earth_like_fixture(&mut trace, date);
    Keyframe::new(Gyr(date), BodyState::Planet(state), &trace).unwrap()
}

fn star_keyframe(date: f64) -> Keyframe {
    let mut trace = Trace::new();
    let state = star(&mut trace);
    Keyframe::new(Gyr(date), BodyState::Star(state), &trace).unwrap()
}

fn timeline(keyframes: Vec<Keyframe>) -> Result<Timeline, ContractError> {
    Timeline::new(
        "MGS 12345 b".parse().unwrap(),
        Seed::new(7),
        EngineVersion(1),
        Gyr(4.57),
        keyframes,
    )
}

#[test]
fn a_planet_keyframe_explains_every_field() {
    let mut trace = Trace::new();
    let state = earth_like_fixture(&mut trace, 4.57);
    // Every node the fixture recorded is held by a field of the state.
    let recorded = trace.len();
    let k = Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace).unwrap();
    k.validate().unwrap();
    let mut ids = Vec::new();
    let mut state = k.state().clone();
    state.for_each_node(&mut |id| ids.push(*id));
    assert_eq!(k.causes().nodes.len(), recorded);
    assert!(ids.iter().all(|id| k.causes().node(*id).is_some()));
}

#[test]
fn a_keyframe_round_trips_through_json() {
    for k in [planet_keyframe(4.57), star_keyframe(0.0)] {
        let json = serde_json::to_string(&k).unwrap();
        let back: Keyframe = serde_json::from_str(&json).unwrap();
        assert_eq!(back, k);
        back.validate().unwrap();
    }
}

#[test]
fn a_corrupted_keyframe_is_reported() {
    let k = planet_keyframe(4.57);
    let mut json: serde_json::Value = serde_json::to_value(&k).unwrap();
    *json.pointer_mut("/state/planet/age/node").unwrap() = serde_json::json!(9999);
    let back: Keyframe = serde_json::from_value(json).unwrap();
    assert_eq!(
        back.validate(),
        Err(ContractError::UnknownNode { node: 9999 })
    );
}

#[test]
fn layers_out_of_order_are_refused() {
    let mut trace = Trace::new();
    let mut state = earth_like_fixture(&mut trace, 4.57);
    state.interior = interior(&mut trace, [6.371e6, 3.48e6]);
    assert_eq!(
        Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace),
        Err(ContractError::LayersNotOrdered)
    );
}

#[test]
fn an_eccentricity_of_one_is_refused() {
    let mut trace = Trace::new();
    let mut state = earth_like_fixture(&mut trace, 4.57);
    state.orbit.eccentricity = leaf(&mut trace, "orbit.e", frac(1.0));
    assert_eq!(
        Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace),
        Err(ContractError::EccentricityNotBelowOne)
    );
}

#[test]
fn a_timeline_needs_ordered_keyframes_of_one_body_kind() {
    assert_eq!(timeline(vec![]), Err(ContractError::NoKeyframe));
    assert_eq!(
        timeline(vec![planet_keyframe(4.57), planet_keyframe(4.0)]),
        Err(ContractError::KeyframesNotOrdered)
    );
    assert_eq!(
        timeline(vec![star_keyframe(0.0), planet_keyframe(4.57)]),
        Err(ContractError::MixedBodyKinds)
    );
    assert!(timeline(vec![planet_keyframe(1.0), planet_keyframe(4.57)]).is_ok());
}

#[test]
fn contract_enums_report_their_serialised_name() {
    assert_eq!(
        PlanetClass::HotJupiter.node_value(),
        crate::NodeValue::Code(Code::of("hot-jupiter"))
    );
    assert_eq!(
        serde_json::to_string(&Renewal::HeatPipe).unwrap(),
        "\"heat-pipe\""
    );
    assert_eq!(serde_json::to_string(&Gas::Co2).unwrap(), "\"co2\"");
}

fn sample_at(radius: f64, density: f64) -> RadialSample {
    RadialSample {
        radius: Metre(radius),
        density: KilogramPerCubicMetre(density),
        pressure: Pascal(1.0e9),
        temperature: Kelvin(3000.0),
        transport: None,
        fusion_share: None,
    }
}

fn with_samples(samples: Vec<RadialSample>) -> Result<Keyframe, ContractError> {
    let mut trace = Trace::new();
    let mut state = earth_like_fixture(&mut trace, 4.57);
    state.interior.samples = leaf(&mut trace, "interior.samples", RadialSamples(samples));
    Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace)
}

#[test]
fn a_nan_keyframe_date_is_refused() {
    let mut trace = Trace::new();
    let state = earth_like_fixture(&mut trace, 4.57);
    assert_eq!(
        Keyframe::new(Gyr(f64::NAN), BodyState::Planet(state), &trace),
        Err(ContractError::NotFinite {
            code: "keyframe.date".into()
        })
    );
}

#[test]
fn a_nan_sample_density_is_refused() {
    assert_eq!(
        with_samples(vec![sample_at(1.0e6, f64::NAN)]),
        Err(ContractError::NotFinite {
            code: "interior.samples".into()
        })
    );
}

#[test]
fn reversed_samples_are_refused_even_with_ordered_layers() {
    assert_eq!(
        with_samples(vec![sample_at(6.0e6, 5000.0), sample_at(3.0e6, 5000.0)]),
        Err(ContractError::LayersNotOrdered)
    );
}

#[test]
fn equal_sample_radii_are_accepted() {
    // Pinned: a discontinuity is two samples at one radius.
    assert!(with_samples(vec![sample_at(3.0e6, 9000.0), sample_at(3.0e6, 5000.0)]).is_ok());
}

#[test]
fn an_eccentricity_of_0_999_is_accepted() {
    let mut trace = Trace::new();
    let mut state = earth_like_fixture(&mut trace, 4.57);
    state.orbit.eccentricity = leaf(&mut trace, "orbit.e", frac(0.999));
    assert!(Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace).is_ok());
}

#[test]
fn a_planet_whose_age_is_not_the_date_is_refused() {
    let mut trace = Trace::new();
    let state = earth_like_fixture(&mut trace, 4.0);
    assert_eq!(
        Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace),
        Err(ContractError::AgeMismatch)
    );
    // And after deserialisation.
    let mut json = serde_json::to_value(planet_keyframe(4.57)).unwrap();
    *json.pointer_mut("/date").unwrap() = serde_json::json!(1.0);
    let back: Keyframe = serde_json::from_value(json).unwrap();
    assert_eq!(back.validate(), Err(ContractError::AgeMismatch));
}

#[test]
fn a_present_outside_the_keyframes_is_refused() {
    let new = |present: f64| {
        Timeline::new(
            "MGS 12345 b".parse().unwrap(),
            Seed::new(7),
            EngineVersion(1),
            Gyr(present),
            vec![planet_keyframe(1.0), planet_keyframe(4.57)],
        )
    };
    assert!(new(1.0).is_ok() && new(3.0).is_ok() && new(4.57).is_ok());
    assert_eq!(new(0.5), Err(ContractError::PresentOutsideKeyframes));
    assert_eq!(new(5.0), Err(ContractError::PresentOutsideKeyframes));
}

#[test]
fn a_designation_must_match_the_kind_of_its_keyframes() {
    let make = |body: &str, keyframes| {
        Timeline::new(
            body.parse().unwrap(),
            Seed::new(7),
            EngineVersion(1),
            Gyr(4.57),
            keyframes,
        )
    };
    assert_eq!(
        make("MGS 12345 A", vec![planet_keyframe(4.57)]),
        Err(ContractError::BodyKindMismatch)
    );
    assert_eq!(
        make("MGS 12345 b", vec![star_keyframe(4.57)]),
        Err(ContractError::BodyKindMismatch)
    );
    assert!(make("MGS 12345 A", vec![star_keyframe(4.57)]).is_ok());
}

#[test]
fn a_planet_rank_of_650_or_more_is_refused_by_the_timeline() {
    let body = Designation {
        system: crate::SystemNumber::new(12_345).unwrap(),
        body: crate::BodyPath::Planet { rank: 650 },
    };
    assert_eq!(
        Timeline::new(
            body,
            Seed::new(7),
            EngineVersion(1),
            Gyr(4.57),
            vec![planet_keyframe(4.57)]
        ),
        Err(ContractError::InvalidDesignation("rank 650".into()))
    );
}

#[test]
fn contract_errors_display_as_codes_with_parameters() {
    assert_eq!(
        ContractError::InvalidDesignation("MGS 1 b".into()).to_string(),
        "invalid_designation(MGS 1 b)"
    );
    assert_eq!(
        ContractError::NotFinite {
            code: "keyframe.date".into()
        }
        .to_string(),
        "not_finite(code=keyframe.date)"
    );
    assert_eq!(
        ContractError::TooManySources(4).to_string(),
        "too_many_sources(4)"
    );
    assert_eq!(ContractError::AgeMismatch.to_string(), "age_mismatch");
}
