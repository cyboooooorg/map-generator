//! Tier 1: the chain behind `class_of` and `model`, on cheap trials: the
//! same trial gives the same answer on both paths, an eccentric placement
//! is modelled on its own orbit, a giant has no surface, an asked
//! obliquity and an asked composition reach both paths, the state holds
//! what the chain recorded. Tier 3: every state is finite on extreme
//! inputs, an asked earth-like has a surface at 1 au.

use super::bare::{Ground, formed};
use super::chain::{Chained, Spun, chain_with, state_of};
use super::climate::fixture::rotation;
use super::draws::{Drawn, draw};
use super::tests::{HOT_ORBIT_AU, VENUS, jupiter_at, neptune_at, reference_trial, trial};
use super::{InteriorError, class_of, class_read, model};
use crate::archetype::{self, ArchetypeId};
use crate::sources::{light, sources};
use crate::spin;
use crate::star::star;
use crate::{Placement, SystemError, Trial, TrialOutcome};
use science::units::{AstronomicalUnit, Day, Dex, EarthMass, Fraction, Gyr, SolarMass};
use timeline::{
    Biosphere, Code, Designation, Orbit, Origin, PlanetClass, PlanetState, Provenance, Seed,
    SystemNumber, Trace,
};

/// Definition: the seed of the hot rocky trial, chosen once.
const HOT_SEED: u64 = 11;

/// Definition: the seed of the cold rocky trial, chosen once.
const COLD_SEED: u64 = 16;

/// Definition: the seed of the giant trial, chosen once.
const GIANT_SEED: u64 = 13;

/// Definition: the orbit of the cold rocky trial, 5 au (beyond the ice
/// line of the trial's Sun-like source, 2.7 au).
const COLD_AU: f64 = 5.0;

/// Definition: the mass of the hot rocky trial, 1 Earth mass.
const ROCKY_MASS: f64 = 1.0;

/// Definition: the mass of the cold rocky trial, 0.3 Earth masses (an
/// ice world that settles in about a second at opt-level 0).
const COLD_MASS: f64 = 0.3;

/// Definition: the giant trial, 318 Earth masses at 5.2 au (Jupiter's).
const GIANT: (f64, f64) = (318.0, 5.2);

/// Definition: the eccentricity of the eccentric placement, 0.3.
const ECCENTRICITY: f64 = 0.3;

/// Definition: Neptune's distance, 30 au, for `neptune_at`.
const NEPTUNE_AU: f64 = 30.0;

/// Definition: any snowball and life draw for the tests that pass them,
/// 0.5 (the planets they run on are not bistable and have no water).
const HALF: f64 = 0.5;

/// Definition: the first survey number, 10 000, for the tests'
/// designations.
const SYSTEM: u32 = 10_000;

/// The placement of `trial` as the draw builds it from the outcome of its
/// class (`draw_with`): the spin drawn from the trial seed with the
/// outcome's class and radius, and the trial's archetype.
fn placed(trial: &Trial, outcome: &TrialOutcome, trace: &mut Trace) -> Placement {
    let archetype = trial.archetype.as_ref().and_then(archetype::placement);
    let spin = spin::spin(
        outcome.class,
        &trial.mass,
        outcome.radius,
        archetype,
        trial.seed,
        trace,
    )
    .unwrap();
    Placement {
        designation: Designation::planet(SystemNumber::new(SYSTEM).unwrap(), 0).unwrap(),
        seed: trial.seed.derive("body"),
        spin,
        trial: trial.clone(),
    }
}

/// The class, radius and density of `state`.
fn outcome_of(state: &PlanetState) -> TrialOutcome {
    TrialOutcome {
        class: *state.class.value(),
        radius: *state.bulk.radius.value(),
        mean_density: *state.bulk.mean_density.value(),
    }
}

/// A trial fixture at a distance, au.
type Fixture = fn(&mut Trace, f64) -> (Trial, Drawn);

/// K1, same trial, same answer: `class_read` on the circular trial of
/// `mass` at `au` from `seed`, then `model` on the placement the draw
/// would build from it, give the same class, radius and density, and, for
/// a planet with a surface (`rocky`), the same obliquity bit for bit, the
/// model's climate reading the placement's node itself; a giant reads
/// none.
fn the_same_trial_gives_the_same_answer(seed: u64, mass: f64, au: f64, rocky: bool) {
    let mut trace = Trace::new();
    let t = trial(&mut trace, seed, mass, au, 0.0);
    let read = class_read(&t, &trace).unwrap();
    let placement = placed(&t, &read.outcome, &mut trace);
    let state = model(&placement, &mut trace).unwrap();
    // Equal, not close: the same draws on the same trial.
    assert_eq!(outcome_of(&state), read.outcome);
    assert_eq!(state.surface.is_some(), rocky, "{:?}", read.outcome);
    if rocky {
        let obliquity = read.obliquity.expect("a surface");
        assert_eq!(
            obliquity.0.to_bits(),
            placement.spin.obliquity.value().0.to_bits()
        );
        let surface = state.surface.as_ref().unwrap();
        let temperature = surface.climate.surface_temperature.node();
        let reads = super::tests::ancestry(&trace, temperature);
        assert!(reads.contains(&placement.spin.obliquity.node()));
    } else {
        assert!(spin::is_giant(read.outcome.class), "{:?}", read.outcome);
        assert_eq!(read.obliquity, None);
    }
}

#[test]
fn the_same_hot_trial_gives_the_same_answer() {
    the_same_trial_gives_the_same_answer(HOT_SEED, ROCKY_MASS, HOT_ORBIT_AU, true);
}

#[test]
fn the_same_cold_trial_gives_the_same_answer() {
    the_same_trial_gives_the_same_answer(COLD_SEED, COLD_MASS, COLD_AU, true);
}

#[test]
fn the_same_giant_trial_gives_the_same_answer() {
    the_same_trial_gives_the_same_answer(GIANT_SEED, GIANT.0, GIANT.1, false);
}

/// A trial of `mass` at `au` and eccentricity `e` around a drawn Sun, its
/// sources at that eccentricity (`draw/place.rs`, `sources`), its period
/// the circular one's (Kepler, not read here).
fn sun_trial(trace: &mut Trace, seed: u64, mass: f64, au: f64, e: f64) -> Trial {
    let sun = star(
        Some(SolarMass(1.0)),
        Some(Dex(0.0)),
        Some(Gyr(4.5673)),
        Seed::new(seed),
        trace,
    )
    .unwrap();
    let light = light(&sun).unwrap();
    let input = |trace: &mut Trace, code: &'static str, value| {
        trace
            .leaf(Code::of(code), value, Provenance::Input)
            .unwrap()
    };
    let a = input(trace, "test.a", AstronomicalUnit(au));
    let e = trace
        .leaf(
            Code::of("test.e"),
            Fraction::new(e).unwrap(),
            Provenance::Input,
        )
        .unwrap();
    let sources = sources(&sun, &light, &a, &e, trace).unwrap();
    Trial {
        seed: Seed::new(seed),
        mass: trace
            .leaf(Code::of("test.mass"), EarthMass(mass), Provenance::Input)
            .unwrap(),
        orbit: Orbit {
            semi_major_axis: a,
            eccentricity: e,
            period: trace
                .leaf(
                    Code::of("test.period"),
                    Day(365.25 * au.powf(1.5)),
                    Provenance::Input,
                )
                .unwrap(),
            perihelion_longitude: trace
                .leaf(
                    Code::of("test.varpi"),
                    science::units::Degree(0.0),
                    Provenance::Input,
                )
                .unwrap(),
        },
        sources,
        age: sun.age,
        metallicity: sun.state.metallicity,
        ice_line: trace
            .leaf(
                Code::of("test.ice-line"),
                AstronomicalUnit(2.7),
                Provenance::Input,
            )
            .unwrap(),
        archetype: None,
    }
}

#[test]
fn an_eccentric_placement_is_modelled_on_its_own_orbit() {
    let mut trace = Trace::new();
    let circular = sun_trial(&mut trace, COLD_SEED, COLD_MASS, COLD_AU, 0.0);
    let eccentric = sun_trial(&mut trace, COLD_SEED, COLD_MASS, COLD_AU, ECCENTRICITY);
    let outcome = class_of(&circular, &trace).unwrap();
    let placement = placed(&eccentric, &outcome, &mut trace);
    let state = model(&placement, &mut trace).unwrap();
    state.validate().unwrap();
    assert!(state.surface.is_some(), "{:?}", state.class.value());
    assert_eq!(state.orbit, eccentric.orbit);
    assert_eq!(
        state.orbit.eccentricity.value().get(),
        ECCENTRICITY,
        "the eccentric orbit"
    );
    let flux = |t: &Trial| t.sources.primary().flux.value().0;
    assert_eq!(state.sources, eccentric.sources);
    assert!(
        state.sources.primary().flux.value().0 > flux(&circular),
        "the mean flux of an eccentric orbit is higher"
    );
}

#[test]
fn a_giant_is_modelled_without_a_surface() {
    let giants: [Fixture; 2] = [jupiter_at, |trace, _| neptune_at(trace, NEPTUNE_AU)];
    for at in giants {
        let mut trace = Trace::new();
        let (t, drawn) = at(&mut trace, GIANT.1);
        let chained = chain_with(&t, &drawn, Spun::Trial, HALF, HALF, &mut trace).unwrap();
        assert_eq!(chained.rotation, None, "no spin call without a surface");
        let class = *chained.class.value();
        let radius = *chained.living.settled.bare.body.bulk().radius.value();
        let shells: Vec<_> = chained
            .living
            .settled
            .bare
            .body
            .layers()
            .iter()
            .map(|shell| {
                [
                    shell.outer_radius.node(),
                    shell.mass.node(),
                    shell.material.node(),
                    shell.state.node(),
                ]
            })
            .collect();
        let outcome = TrialOutcome {
            class,
            radius,
            mean_density: *chained.living.settled.bare.body.bulk().mean_density.value(),
        };
        let placement = placed(&t, &outcome, &mut trace);
        let state = state_of(&placement, chained, &mut trace).unwrap();
        assert!(state.surface.is_none(), "{class:?}");
        state.validate().unwrap();
        let layers: Vec<_> = state
            .interior
            .layers
            .iter()
            .map(|layer| {
                assert_eq!(layer.transport, None);
                [
                    layer.outer_radius.node(),
                    layer.mass.node(),
                    layer.material.node(),
                    layer.state.node(),
                ]
            })
            .collect();
        assert_eq!(layers, shells, "the body's own nodes");
        assert!(state.interior.samples.value().0.is_empty());
        assert_eq!(
            trace.node(state.interior.samples.node()).unwrap().origin,
            Origin::Leaf(Provenance::Imposed)
        );
    }
}

#[test]
fn an_asked_obliquity_reaches_both_paths() {
    let earth_like = ArchetypeId(Code::of("earth-like"));
    let asked = archetype::placement(&earth_like)
        .and_then(|p| p.obliquity)
        .expect("earth-like asks an obliquity");
    let mut trace = Trace::new();
    let t = Trial {
        archetype: Some(earth_like),
        ..trial(&mut trace, HOT_SEED, ROCKY_MASS, HOT_ORBIT_AU, 0.0)
    };
    let drawn = draw(t.seed, &t, &mut trace).unwrap();
    let outcome = TrialOutcome {
        class: PlanetClass::Rocky,
        radius: science::units::EarthRadius(1.0),
        mean_density: science::units::KilogramPerCubicMetre(5514.0),
    };
    let placement = placed(&t, &outcome, &mut trace);
    for spun in [Spun::Trial, Spun::Placed(&placement.spin)] {
        let chained = chain_with(&t, &drawn, spun, HALF, HALF, &mut trace).unwrap();
        let obliquity = chained.rotation.expect("a surface").obliquity;
        assert_eq!(*obliquity.value(), asked, "{spun:?}");
        assert_eq!(
            trace.node(obliquity.node()).unwrap().origin,
            Origin::Leaf(Provenance::Input),
            "{spun:?}"
        );
    }
}

/// Definition: the earth-like trial, 1 Earth mass at 1 au (the
/// archetype's mass and its one Earth flux round the trial's Sun-like
/// source).
const EARTH_LIKE: (f64, f64) = (1.0, 1.0);

/// The earth-like trial of `mass` Earth masses at `au`, and the same trial
/// unasked.
fn earth_like_at(trace: &mut Trace, mass: f64, au: f64) -> (Trial, Trial) {
    let free = trial(trace, HOT_SEED, mass, au, 0.0);
    let t = Trial {
        archetype: Some(ArchetypeId(Code::of("earth-like"))),
        ..free.clone()
    };
    (t, free)
}

/// Asserts that `drawn` holds the composition earth-like imposes, as
/// input leaves.
fn assert_asked(trace: &Trace, drawn: &Drawn) {
    let asked = archetype::composition(&ArchetypeId(Code::of("earth-like")))
        .expect("earth-like asks a composition");
    for (leaf, value) in [
        (drawn.core_mass_fraction, asked.core_mass_fraction),
        (drawn.water_fraction, asked.water_fraction),
        (drawn.envelope_fraction, asked.envelope_fraction),
    ] {
        assert_eq!(Some(leaf.value().get()), value, "{leaf:?}");
        assert_eq!(
            trace.node(leaf.node()).unwrap().origin,
            Origin::Leaf(Provenance::Input),
            "{leaf:?}"
        );
    }
    let migrated = drawn.migrated.expect("inside the ice line");
    assert!(!*migrated.value());
    assert_eq!(
        trace.node(migrated.node()).unwrap().origin,
        Origin::Leaf(Provenance::Input)
    );
}

/// `class_of`'s path and `model`'s on the asked trial `t`: the same
/// composition, so a surface on both (a body without an envelope has
/// one, by the class's table) and the same class.
fn assert_both_paths(trace: &mut Trace, t: &Trial) {
    let read = class_read(t, trace).unwrap();
    assert!(read.obliquity.is_some(), "{read:?}");
    let placement = placed(t, &read.outcome, trace);
    let state = model(&placement, trace).unwrap();
    assert!(state.surface.is_some(), "{:?}", state.class);
    assert_eq!(state.envelope_fraction.value().get(), 0.0);
    assert_eq!(
        trace.node(state.envelope_fraction.node()).unwrap().origin,
        Origin::Leaf(Provenance::Input)
    );
    assert_eq!(*state.class.value(), read.outcome.class);
}

#[test]
fn an_asked_composition_reaches_both_paths() {
    // At 1 M_E and 1 au: the draws give way to the archetype's inputs,
    // and the ground keeps a surface.
    let (mass, au) = EARTH_LIKE;
    let mut trace = Trace::new();
    let (t, free) = earth_like_at(&mut trace, mass, au);
    // Unasked, the envelope is drawn: Lammer's grid at 1 M_E starts
    // above 0.
    let unasked = draw(free.seed, &free, &mut trace).unwrap();
    assert!(unasked.envelope_fraction.value().get() > 0.0);
    let drawn = draw(t.seed, &t, &mut trace).unwrap();
    assert_asked(&trace, &drawn);
    assert!(matches!(
        formed(&t, &drawn, &mut trace).unwrap(),
        Ground::Formed(_)
    ));
    // Both paths, on the hot orbit that settles in seconds (a 1 au Earth
    // is a bistable solve: tier 3, below).
    let mut trace = Trace::new();
    let (t, _) = earth_like_at(&mut trace, mass, HOT_ORBIT_AU);
    assert_both_paths(&mut trace, &t);
}

#[test]
#[ignore = "heavy: a bistable Earth solved on both paths, 160 s at opt-level 0"]
fn an_asked_earth_like_has_a_surface_at_one_au() {
    let (mass, au) = EARTH_LIKE;
    let mut trace = Trace::new();
    let (t, _) = earth_like_at(&mut trace, mass, au);
    assert_both_paths(&mut trace, &t);
}

#[test]
fn an_interior_error_is_a_system_error() {
    let error = SystemError::from(InteriorError::NoEnvelope);
    assert!(matches!(error, SystemError::Planet(_)), "{error:?}");
    let message = error.to_string();
    assert!(message.starts_with("system.planet: interior-"), "{message}");
}

#[test]
fn the_planet_state_holds_the_chain() {
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &VENUS);
    let rot = rotation(&mut trace, science::units::Degree(0.0), false);
    let spin = timeline::Spin {
        rotation_period: trace
            .leaf(Code::of("test.rotation"), Day(243.0), Provenance::Input)
            .unwrap(),
        obliquity: rot.obliquity,
    };
    let chained: Chained =
        chain_with(&t, &drawn, Spun::Placed(&spin), HALF, HALF, &mut trace).unwrap();
    let settled_water = chained
        .living
        .settled
        .bare
        .surface
        .as_ref()
        .unwrap()
        .water
        .clone();
    let climate = chained.living.settled.climate.clone().unwrap();
    let placement = Placement {
        designation: Designation::planet(SystemNumber::new(SYSTEM).unwrap(), 1).unwrap(),
        seed: t.seed,
        spin,
        trial: t,
    };
    let mut state = state_of(&placement, chained, &mut trace).unwrap();
    state.validate().unwrap();
    assert_eq!(*state.class.value(), PlanetClass::RunawayGreenhouse);
    assert!(state.targets.is_empty());
    let surface = state.surface.as_ref().expect("Venus has a surface");
    assert_eq!(surface.water, settled_water);
    assert_eq!(surface.liquid, None);
    assert_eq!(*surface.life.value(), Biosphere::Lifeless);
    assert!(surface.terrain.units.is_empty());
    assert_eq!(surface.atmosphere.as_ref(), Some(&climate.atmosphere));
    assert_eq!(surface.climate, climate.global);
    trace.freeze(&mut state).unwrap();
}

/// The path of the first number of `value` that is not finite. serde_json
/// writes a non-finite `f64` as `null`, so a `null` counts as one, except
/// for the fields that are an `Option` of the contract. Ported from
/// `legacy-final:crates/mapgen/src/planet/classify.rs` (`not_finite`,
/// `first_bad`, l.1171-1190), without legacy's sign test: the rewrite's
/// states hold signed values (a metallicity, a perihelion longitude).
fn first_bad(value: &serde_json::Value, path: &str, key: &str) -> Option<String> {
    /// Definition: the `Option` fields of `PlanetState` and its parts.
    const OPTIONAL: [&str; 5] = [
        "surface",
        "atmosphere",
        "liquid",
        "transport",
        "fusion_share",
    ];
    match value {
        serde_json::Value::Number(n) => {
            let x = n.as_f64().unwrap();
            (!x.is_finite()).then(|| format!("{path}: {x}"))
        }
        serde_json::Value::Null if !OPTIONAL.contains(&key) => Some(format!("{path}: null")),
        serde_json::Value::Array(a) => a
            .iter()
            .enumerate()
            .find_map(|(i, v)| first_bad(v, &format!("{path}/{i}"), key)),
        serde_json::Value::Object(o) => o
            .iter()
            .find_map(|(k, v)| first_bad(v, &format!("{path}/{k}"), k)),
        _ => None,
    }
}

#[test]
fn the_finiteness_check_sees_null() {
    let json = serde_json::json!({"surface": null, "a": [1.0, {"b": null}]});
    assert_eq!(first_bad(&json, "", ""), Some("/a/1/b: null".to_owned()));
    let fine = serde_json::json!({"surface": null, "a": [1.0, -2.0]});
    assert_eq!(first_bad(&fine, "", ""), None);
}

/// Definition: the extreme masses of legacy's test, Earth masses.
const EXTREME_MASSES: [f64; 2] = [0.01, 20.0];

/// Definition: the extreme fluxes of legacy's test, Earth fluxes.
const EXTREME_FLUXES: [f64; 2] = [1e-3, 1e4];

/// Definition: the extreme ages of legacy's test, Gyr.
const EXTREME_AGES: [f64; 2] = [0.5, 13.0];

#[test]
#[ignore = "heavy: the whole chain on eight extreme trials (Review Focus 4), 2.0 s at opt-level 0, 0.3 s in release"]
fn every_planet_state_is_finite_on_extreme_inputs() {
    for mass in EXTREME_MASSES {
        for flux in EXTREME_FLUXES {
            for age in EXTREME_AGES {
                let what = format!("{mass} M_E, {flux} S_E, {age} Gyr");
                let mut trace = Trace::new();
                let mut t = trial(&mut trace, HOT_SEED, mass, flux.sqrt().recip(), 0.0);
                t.age = trace
                    .leaf(Code::of("test.age"), Gyr(age), Provenance::Input)
                    .unwrap();
                let outcome = match class_of(&t, &trace) {
                    Ok(outcome) => outcome,
                    Err(SystemError::Planet(error)) => {
                        println!("{what}: {error}");
                        continue;
                    }
                    Err(error) => panic!("{what}: {error}"),
                };
                let placement = placed(&t, &outcome, &mut trace);
                let state = model(&placement, &mut trace).unwrap_or_else(|e| panic!("{what}: {e}"));
                let json = serde_json::to_value(&state).unwrap();
                assert_eq!(first_bad(&json, "", ""), None, "{what}");
                println!("{what}: {:?}", state.class.value());
            }
        }
    }
}
