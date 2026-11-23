//! Tier 1: the planet chained from its draw to its bare air.

use std::collections::BTreeSet;

use super::bare::{Bare, bare, bare_with};
use super::codes;
use super::tests::{EARTH, MARS, ReferenceBody, VENUS, derived_from, reference_trial, trial};
use science::escape::MIN_ENVELOPE;
use science::units::Kilogram;
use timeline::{Code, NodeId, Origin, Seed, Trace};

/// The bare planet of a trial of `mass` Earth masses at `au`, drawn from
/// the seed `seed`.
fn drawn(seed: u64, mass: f64, au: f64) -> (Bare, Trace) {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, mass, au, 0.0);
    let b = bare(&t, Seed::new(seed), &mut trace).unwrap();
    (b, trace)
}

/// The bare planet of the reference `body`, its composition set by hand.
fn reference(body: &ReferenceBody) -> (crate::Trial, Bare, Trace) {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, body);
    let b = bare_with(&t, &d, &mut trace).unwrap();
    (t, b, trace)
}

/// Every node `node` derives from, itself included, walking the
/// `Origin::Derived` links back to the leaves.
fn ancestry(trace: &Trace, node: NodeId) -> BTreeSet<NodeId> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![node];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if let Origin::Derived { from } = &trace.node(id).unwrap().origin {
            stack.extend(from.iter().copied());
        }
    }
    seen
}

#[test]
fn the_same_seed_gives_the_same_bare_planet() {
    // Definition: Earth's mass at 1 au, seed 7, a draw whose envelope the
    // escape strips (a test choice).
    let (a, trace_a) = drawn(7, 1.0, 1.0);
    let (b, trace_b) = drawn(7, 1.0, 1.0);
    assert_eq!(a, b);
    // The whole chain ran: this draw left a surface.
    assert!(a.surface.is_some());
    assert_eq!(trace_a.len(), trace_b.len());
}

#[test]
fn an_enveloped_planet_has_no_surface() {
    // Definition: 10 M_E at 1 au, drawn as an ice giant (from 9 % of
    // envelope, `envelope::ICE_GIANT_ENVELOPE_MIN`), of which the escape at
    // 1 au leaves well above `MIN_ENVELOPE`.
    let (b, _) = drawn(3, 10.0, 1.0);
    assert!(b.drawn.envelope_fraction.value().get() >= MIN_ENVELOPE);
    assert!(b.surface.is_none());
}

#[test]
fn a_runaway_planet_is_lighter() {
    // As `venus_ran_away_and_dried`: the reference Venus holds no water
    // and would lose none; given Earth's ocean it loses it to the steam.
    let wet = ReferenceBody {
        water_fraction: EARTH.water_fraction,
        ..VENUS
    };
    let (t, b, trace) = reference(&wet);
    let surface = b.surface.as_ref().unwrap();
    assert!(*surface.water.runaway.value());
    assert!(b.trial.mass.value().0 < t.mass.value().0);
    assert_eq!(
        trace.node(b.trial.mass.node()).unwrap().code,
        Code::of(codes::MASS_AFTER_STEAM)
    );
    let steam = ancestry(&trace, b.trial.mass.node())
        .into_iter()
        .any(|id| trace.node(id).unwrap().code == Code::of(codes::WATER_STEAM_LOST));
    assert!(
        steam,
        "the mass after the steam derives from the steam lost"
    );
    assert!(ancestry(&trace, b.trial.mass.node()).contains(&t.mass.node()));
}

#[test]
fn every_surface_value_traces_back_to_the_trial() {
    let wet = ReferenceBody {
        water_fraction: EARTH.water_fraction,
        ..VENUS
    };
    for body in [EARTH, wet] {
        let (t, b, trace) = reference(&body);
        let s = b.surface.as_ref().unwrap();
        let mut nodes = vec![
            ("regime", s.regime.node()),
            ("core.state", s.core.state.node()),
            ("core.field", s.core.field.node()),
            ("water.inventory", s.water.inventory.node()),
            ("air.pressure", s.atmosphere.surface_pressure.node()),
        ];
        // `water.boiled` stays an imposed leaf (false) until the climate's
        // boiling loop derives it (#659).
        nodes.extend(
            s.atmosphere
                .partial
                .values()
                .map(|p| ("air.partial", p.node())),
        );
        for (name, node) in nodes {
            // A fresh leaf would panic here: every value is derived.
            derived_from(&trace, node);
            assert!(
                ancestry(&trace, node).contains(&t.mass.node()),
                "{name} of {body:?} does not reach the trial's mass"
            );
        }
        // The runaway reads the light alone, not the mass.
        let runaway = s.water.runaway.node();
        derived_from(&trace, runaway);
        let light = ancestry(&trace, runaway);
        let source = t.sources.primary();
        assert!(light.contains(&source.flux.node()));
        assert!(light.contains(&source.effective_temperature.node()));
    }
}

/// Definition: a Mars at 0.7 au holding 5 % of its mass in water, a small
/// runaway planet whose cosmic shoreline takes part of the vapour its
/// steam loss left (a test choice).
const SMALL_WET_RUNAWAY: ReferenceBody = ReferenceBody {
    orbit: 0.7,
    water_fraction: 0.05,
    ..MARS
};

#[test]
fn the_shoreline_lightens_a_runaway_planet_again() {
    let (t, b, trace) = reference(&SMALL_WET_RUNAWAY);
    let s = b.surface.as_ref().unwrap();
    assert!(*s.water.runaway.value());
    // The water the air kept is the shoreline's share of the inventory.
    let inventory = derived_from(&trace, s.water.inventory.node());
    assert_eq!(
        trace.node(inventory[1]).unwrap().code,
        Code::of(codes::AIR_SHORELINE_KEPT)
    );
    // The planet weighs that water: its dry mass and the vapour kept.
    assert!(derived_from(&trace, b.trial.mass.node()).contains(&s.water.inventory.node()));
    let dry = Kilogram::from(*t.mass.value()).0 * (1.0 - SMALL_WET_RUNAWAY.water_fraction);
    let mass = Kilogram::from(*b.trial.mass.value()).0;
    assert!(
        ((dry + s.water.inventory.value().0) / mass - 1.0).abs() < 1e-12,
        "{mass}"
    );
}
