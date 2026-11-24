//! Tier 1: the planet chained from its draw to its bare air.

use super::bare::{Bare, bare, bare_with};
use super::codes;
use super::draws::Drawn;
use super::tests::{
    EARTH, MARS, ReferenceBody, WET_VENUS, ancestry, derived_from, reference_trial, trial,
    with_fluence,
};
use crate::Trial;
use science::escape::MIN_ENVELOPE;
use science::units::Kilogram;
use timeline::{Code, Seed, Trace};

/// The bare planet of a trial of `mass` Earth masses at `au`, drawn from
/// the seed `seed`.
fn drawn(seed: u64, mass: f64, au: f64) -> (Bare, Trace) {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, mass, au, 0.0);
    let b = bare(&t, Seed::new(seed), &mut trace).unwrap();
    (b, trace)
}

/// The bare planet of the reference `body`, its composition set by hand.
fn reference(body: &ReferenceBody) -> (Trial, Bare, Trace) {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, body);
    let b = bare_with(&t, &d, &mut trace).unwrap();
    (t, b, trace)
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
    // Definition: 10 M_E at 1 au with 20 % of envelope, set by hand, of
    // which the escape at 1 au leaves well above `MIN_ENVELOPE` (a test
    // choice).
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 10.0, 1.0, 0.0);
    let d = Drawn::fixed(
        &t,
        &mut trace,
        EARTH.core_mass_fraction,
        EARTH.water_fraction,
        0.0,
        Some(false),
        0.2,
        None,
        None,
    );
    let b = bare_with(&t, &d, &mut trace).unwrap();
    assert!(b.drawn.envelope_fraction.value().get() >= MIN_ENVELOPE);
    assert!(b.surface.is_none());
}

#[test]
fn a_runaway_planet_is_lighter() {
    // As `venus_ran_away_and_dried`: the reference Venus holds no water
    // and would lose none; given Earth's ocean it loses it to the steam.
    let (t, b, trace) = reference(&WET_VENUS);
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
    for body in [EARTH, WET_VENUS] {
        let (t, b, trace) = reference(&body);
        let s = b.surface.as_ref().unwrap();
        let mut nodes = vec![
            ("regime", s.regime.node()),
            ("core.state", s.core.state.node()),
            ("core.field", s.core.field.node()),
            ("water.inventory", s.water.inventory.node()),
            ("air.pressure", s.atmosphere.surface_pressure.node()),
            ("air.frozen-co2", s.frozen_co2.node()),
            ("air.outgassed-co2", s.outgassed_co2.node()),
            ("air.reducing", s.reducing.node()),
        ];
        // `water.boiled` stays an imposed leaf (false) until `settled`
        // derives it from the boil loop's verdict.
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

/// Definition: a Mars at 0.7 au that migrated in with 10 % of its mass
/// in water, the least of a water world's draw (Marboeuf et al. 2014), a
/// small runaway planet whose cosmic shoreline takes part of the vapour
/// its steam loss left (a test choice). A water world, all of whose water
/// is at its surface: the steam takes all of a dry Mars's surface share,
/// and all of a water world's 5 %.
const SMALL_WET_RUNAWAY: ReferenceBody = ReferenceBody {
    orbit: 0.7,
    water_fraction: 0.1,
    ..MARS
};

#[test]
fn the_shoreline_lightens_a_runaway_planet_again() {
    let body = SMALL_WET_RUNAWAY;
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, body.mass, body.orbit, body.eccentricity);
    let d = Drawn::fixed(
        &t,
        &mut trace,
        body.core_mass_fraction,
        body.water_fraction,
        0.0,
        Some(true),
        0.0,
        None,
        None,
    );
    let b = bare_with(&t, &d, &mut trace).unwrap();
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
    let dry = Kilogram::from(*t.mass.value()).0 * (1.0 - body.water_fraction);
    let mass = Kilogram::from(*b.trial.mass.value()).0;
    assert!(
        ((dry + s.water.inventory.value().0) / mass - 1.0).abs() < 1e-12,
        "{mass}"
    );
}

// The escape drops an envelope under its floor even without XUV light
// (`escape_tests::a_wisp_of_envelope_is_dropped_without_xuv`): the chain
// then goes on to the surface.
#[test]
fn a_wisp_of_envelope_leaves_a_surface() {
    let mut trace = Trace::new();
    // Definition: 5 M_E at 0.5 au with 0.05 % of envelope, half of
    // `MIN_ENVELOPE`, under no XUV light (a test choice).
    let t = trial(&mut trace, 1, 5.0, 0.5, 0.0);
    let dark = with_fluence(&mut trace, &t, 0.0);
    let d = Drawn::fixed(
        &dark,
        &mut trace,
        EARTH.core_mass_fraction,
        EARTH.water_fraction,
        0.0,
        Some(false),
        5e-4,
        None,
        None,
    );
    let b = bare_with(&dark, &d, &mut trace).unwrap();
    assert!(b.surface.is_some());
}
