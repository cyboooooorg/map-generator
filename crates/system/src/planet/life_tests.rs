//! Life: its draw, its stage on the settled planet, and the climate
//! solved again on the air it oxygenated. Ported from
//! `legacy-final:crates/mapgen/src/planet/life.rs` (`mod tests`), less
//! `life_draw_is_the_last_draw`, which pinned legacy's random stream.
//!
//! Every wet body found keeps a frozen branch (bistable, about a minute
//! a solve at opt-level 0), so the second solve is tier 3: here on a
//! small water world, and on the reference Earth in
//! `climate::heavy_tests::the_reference_earth_lives_and_reaches_its_climate`.

use super::climate::Rotation;
use super::climate::fixture::rotation;
use super::codes;
use super::draws::{LABELS, life_draw, snowball_draw};
use super::life::{Living, living};
use super::settled::{Settled, settled_with};
use super::settled_tests::{SMALL_MASS, nodes_of};
use super::tests::{
    EARTH, MARS, ReferenceBody, VENUS, ancestry, derived_from, reference_trial, water_world_trial,
};
use science::life::O2_PER_N2;
use science::units::{Degree, Gyr};
use std::collections::BTreeSet;
use timeline::{Biosphere, Code, Gas, LiquidKind, Provenance, Seed, Trace, Traced};

/// Definition: the seed of the tests.
const SEED: u64 = 1;

/// Definition: a life draw under any chance of life, 0 (legacy
/// `earth_has_oxygenic_life`).
const LIVING_DRAW: f64 = 0.0;

/// Definition: a life draw over the chance of life, 0.99 (legacy
/// `a_habitable_world_can_be_lifeless`).
const LIFELESS_DRAW: f64 = 0.99;

/// Definition: the age of the tests, 4.6 Gyr (legacy
/// `earth_has_oxygenic_life`), past `science::life::MODERN_OXYGEN_AFTER_GYR`.
const AGE: Gyr = Gyr(4.6);

/// Definition: an obliquity for the bodies of these tests, 0 degrees: it
/// weights only the light on ice caps, which none of them has.
const OBLIQUITY: Degree = Degree(0.0);

/// Definition: a small water world, 0.2 Earth masses (`SMALL_MASS`) with
/// Earth's core share and 1e-3 of its mass in water at 1 AU, a test
/// input: wet and regulated. No wet body that is not bistable was found
/// (an Earth from 0.93 to 1 AU, water worlds of 0.1 to 5 Earth masses
/// from 0.8 to 1.5 AU: each either loses its water or keeps a frozen
/// branch), so the test is tier 3.
const SMALL_WATER_WORLD: (f64, f64, f64) = (SMALL_MASS, 1.0e-3, 1.0);

/// The settled `body`, its rotation, the snowball draw it was solved
/// with and its trace.
fn settled_of(body: &ReferenceBody) -> (Settled, Rotation, f64, Trace) {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, body);
    let rot = rotation(&mut trace, OBLIQUITY, false);
    let seed = Seed::new(SEED).derive(LABELS.snowball);
    let settled = settled_with(&t, &d, &rot, seed, &mut trace).unwrap();
    (settled, rot, snowball_draw(seed), trace)
}

/// An age leaf of `trace`.
fn age(trace: &mut Trace, age: Gyr) -> Traced<Gyr> {
    trace
        .leaf(Code::of("test.age"), age, Provenance::Input)
        .unwrap()
}

/// The life of `settled` (with its rotation, snowball draw and trace)
/// with the life draw `draw`, `AGE` old.
fn living_with(
    (settled, rot, snowball, mut trace): (Settled, Rotation, f64, Trace),
    draw: f64,
) -> (Settled, Living, Trace) {
    let a = age(&mut trace, AGE);
    let life = living(settled.clone(), &a, draw, &rot, snowball, &mut trace).unwrap();
    (settled, life, trace)
}

/// The life of the settled `body` with the life draw `draw`, `AGE` old.
fn living_of(body: &ReferenceBody, draw: f64) -> (Settled, Living, Trace) {
    living_with(settled_of(body), draw)
}

/// [`SMALL_WATER_WORLD`] settled, as [`settled_of`].
fn small_water_world() -> (Settled, Rotation, f64, Trace) {
    let (mass, water, au) = SMALL_WATER_WORLD;
    let mut trace = Trace::new();
    let (t, d) = water_world_trial(&mut trace, mass, EARTH.core_mass_fraction, water, au);
    let rot = rotation(&mut trace, OBLIQUITY, false);
    let seed = Seed::new(SEED).derive(LABELS.snowball);
    let settled = settled_with(&t, &d, &rot, seed, &mut trace).unwrap();
    (settled, rot, snowball_draw(seed), trace)
}

/// The codes of every node `node` derives from, itself included.
fn causes(trace: &Trace, node: timeline::NodeId) -> BTreeSet<String> {
    ancestry(trace, node)
        .into_iter()
        .map(|id| trace.node(id).unwrap().code.as_str().to_owned())
        .collect()
}

#[test]
fn life_draws_from_its_own_sub_seed() {
    let seed = Seed::new(SEED);
    let draw = life_draw(seed.derive("planet/life"));
    assert!(draw > 0.0 && draw < 1.0, "{draw}");
    assert_ne!(draw, snowball_draw(seed.derive(LABELS.snowball)));
    assert_eq!(draw, life_draw(seed.derive(LABELS.life)));
}

#[test]
fn venus_and_mars_are_lifeless() {
    for body in [VENUS, MARS] {
        for draw in [LIVING_DRAW, LIFELESS_DRAW] {
            let (settled, life, trace) = living_of(&body, draw);
            let node = life.life.expect("a body with a surface");
            assert_eq!(*node.value(), Biosphere::Lifeless, "{draw}");
            let parents = derived_from(&trace, node.node());
            let drawn = nodes_of(&trace, codes::LIFE_DRAW);
            assert_eq!(drawn.len(), 1);
            assert!(parents.contains(&drawn[0].0), "{parents:?}");
            let age = trace.node(parents[1]).unwrap().code.as_str().to_owned();
            assert_eq!(age, "test.age");
            assert_eq!(life.settled, settled, "{draw}");
        }
    }
}

#[test]
#[ignore = "heavy: a bistable small water world settled twice and solved again once, 165 s at opt-level 0, 5.6 s in release"]
fn the_second_solve_is_what_the_class_reads() {
    let (settled, life, trace) = living_with(small_water_world(), LIVING_DRAW);
    let first = settled.climate.as_ref().unwrap();
    assert_eq!(
        first.liquid.map(|l| *l.value()),
        Some(LiquidKind::Water),
        "{first:?}"
    );
    assert_eq!(*life.life.unwrap().value(), Biosphere::Modern);
    let air = &life.settled.bare.surface.as_ref().unwrap().atmosphere;
    let n2 = air.partial[&Gas::N2];
    let o2 = air.partial[&Gas::O2];
    assert_eq!(o2.value().0, O2_PER_N2 * n2.value().0);
    assert!(derived_from(&trace, o2.node()).contains(&n2.node()));
    assert_eq!(air.partial[&Gas::Ch4].value().0, 0.0);
    let second = life.settled.climate.as_ref().unwrap();
    let t = second.global.surface_temperature;
    assert_ne!(t.node(), first.global.surface_temperature.node());
    assert!(causes(&trace, t.node()).contains(codes::LIFE_AIR_O2));
    assert_eq!(life.settled.hides_surface, settled.hides_surface);
    assert_eq!(nodes_of(&trace, codes::CLIMATE_BOILS).len(), 1);

    // A lifeless draw keeps the first climate.
    let (settled, lifeless, _) = living_with(small_water_world(), LIFELESS_DRAW);
    assert_eq!(*lifeless.life.unwrap().value(), Biosphere::Lifeless);
    assert_eq!(lifeless.settled, settled);
}
