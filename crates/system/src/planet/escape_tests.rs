//! Tier 1: the energy-limited escape of an envelope, ported from
//! `legacy-final:crates/mapgen/src/planet/escape.rs` (its tests) on the
//! fixtures of `planet/tests.rs`.

use super::codes;
use super::draws::Drawn;
use super::enveloped::Body;
use super::escape::{Escaped, escaped};
use super::interior::interior;
use super::tests::{
    EARTH_CORE_MASS_FRACTION, EARTH_WATER_FRACTION, derived_from, jupiter_at, trial,
};
use crate::Trial;
use science::escape::MIN_ENVELOPE;
use science::giants::GAS_GIANT_MIN_MASS;
use science::units::{EarthMass, JoulePerSquareMetre};
use timeline::{Code, Provenance, StellarSources, Trace};

/// Source: the mini-Neptune of the legacy tests, 5 M_E with 1 % of its
/// mass in hydrogen and helium, the smallest envelope of Owen & Wu 2017's
/// sample (`X_min = 0.01`, section 3.3).
const MINI_NEPTUNE: (f64, f64) = (5.0, 0.01);

/// Definition: the close orbit of the legacy tests, 0.05 au, 400 times
/// Earth's flux and XUV fluence around the trial's Sun-like source.
const CLOSE_AU: f64 = 0.05;

/// Definition: the temperate orbit of the legacy tests, 0.5 au.
const TEMPERATE_AU: f64 = 0.5;

/// Source: the radius valley of Fulton et al. 2017 (AJ 154, 109), centred
/// near 1.75 R_E: a stripped core lies below it, the legacy test's 1.8 R_E.
const VALLEY_RADIUS: f64 = 1.8;

/// Definition: a numerical tolerance on a mass computed two ways, relative.
const ROUNDING: f64 = 1e-12;

/// A dry planet of `mass` M_E at `au` with the envelope fraction
/// `envelope` (with Earth's core and water fractions), and its formed body.
fn formed(trace: &mut Trace, mass: f64, au: f64, envelope: f64) -> (Trial, Drawn, Body) {
    let t = trial(trace, 1, mass, au, 0.0);
    let drawn = Drawn::fixed(
        &t,
        trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
        None,
        envelope,
        None,
        None,
    );
    let body = interior(&t, &drawn, trace).unwrap();
    (t, drawn, body)
}

/// `t` with its source's XUV fluence replaced by `fluence`, J/m².
fn with_fluence(trace: &mut Trace, t: &Trial, fluence: f64) -> Trial {
    let mut source = t.sources.primary().clone();
    source.xuv.fluence = trace
        .leaf(
            Code::of("test.xuv-fluence"),
            JoulePerSquareMetre(fluence),
            Provenance::Input,
        )
        .unwrap();
    Trial {
        sources: StellarSources::new(vec![source]).unwrap(),
        ..t.clone()
    }
}

/// The envelope mass of an escaped planet over the one it formed with.
fn kept_share(before: (&Trial, &Drawn), after: &Escaped) -> f64 {
    let formed = before.0.mass.value().0 * before.1.envelope_fraction.value().get();
    after.trial.mass.value().0 * after.drawn.envelope_fraction.value().get() / formed
}

#[test]
fn a_close_mini_neptune_is_stripped_to_its_core() {
    let mut trace = Trace::new();
    let (mass, envelope) = MINI_NEPTUNE;
    let (t, drawn, body) = formed(&mut trace, mass, CLOSE_AU, envelope);
    let left = escaped(&t, &drawn, body, &mut trace).unwrap();
    assert_eq!(left.drawn.envelope_fraction.value().get(), 0.0);
    let radius = left.body.bulk().radius.value().0;
    assert!(radius < VALLEY_RADIUS, "{radius}");
}

// Review Focus 1: the stripped planet is a rocky body of `M - f M`, its
// bulk water kept, and the trace says the loss stripped it.
#[test]
fn a_stripped_mini_neptune_is_rebuilt_rocky() {
    let mut trace = Trace::new();
    let (mass, envelope) = MINI_NEPTUNE;
    let (t, drawn, body) = formed(&mut trace, mass, CLOSE_AU, envelope);
    let left = escaped(&t, &drawn, body, &mut trace).unwrap();
    assert!(matches!(left.body, Body::Rocky(_)), "{:?}", left.body);
    let core = mass * (1.0 - envelope);
    let kept = left.trial.mass.value().0;
    assert!(
        (kept / core - 1.0).abs() < ROUNDING,
        "{kept} against {core}"
    );
    assert_eq!(left.body.bulk().mass, left.trial.mass);
    assert_eq!(left.drawn.water_fraction, drawn.water_fraction);
    let lost = left.lost.expect("an envelope was lost");
    let fraction = left.drawn.envelope_fraction.node();
    assert_eq!(
        trace.node(fraction).unwrap().code,
        Code::of(codes::ENVELOPE_KEPT_FRACTION)
    );
    assert!(derived_from(&trace, fraction).contains(&lost.node()));
    let mass_node = left.trial.mass.node();
    assert_eq!(
        trace.node(mass_node).unwrap().code,
        Code::of(codes::MASS_KEPT)
    );
    let from = derived_from(&trace, mass_node);
    assert!(from.contains(&t.mass.node()) && from.contains(&lost.node()));
    assert_eq!(
        trace.node(lost.node()).unwrap().code,
        Code::of(codes::ENVELOPE_LOST)
    );
    let from = derived_from(&trace, lost.node());
    assert!(from.contains(&t.sources.primary().xuv.fluence.node()));
}

// Review Focus 4: a giant's envelope is a deep well; its kept fraction is
// `1 - Mz / M'`, from the drawn heavy mass.
#[test]
fn a_close_giant_keeps_its_envelope() {
    let mut trace = Trace::new();
    let (t, drawn) = jupiter_at(&mut trace, CLOSE_AU);
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let left = escaped(&t, &drawn, body, &mut trace).unwrap();
    let kept = kept_share((&t, &drawn), &left);
    assert!(kept >= 0.99, "{kept}");
    assert!(matches!(left.body, Body::Giant(_)), "{:?}", left.body);
    let heavy = drawn.heavy_mass.unwrap();
    let fraction = left.drawn.envelope_fraction;
    let expected = 1.0 - heavy.value().0 / left.trial.mass.value().0;
    assert!((fraction.value().get() / expected - 1.0).abs() < ROUNDING);
    let from = derived_from(&trace, fraction.node());
    assert!(from.contains(&heavy.node()) && from.contains(&left.trial.mass.node()));
}

#[test]
fn a_temperate_mini_neptune_keeps_half() {
    let mut trace = Trace::new();
    let (mass, envelope) = MINI_NEPTUNE;
    let (t, drawn, body) = formed(&mut trace, mass, TEMPERATE_AU, envelope);
    let left = escaped(&t, &drawn, body, &mut trace).unwrap();
    let kept = kept_share((&t, &drawn), &left);
    assert!(kept >= 0.5, "{kept}");
}

#[test]
fn a_rocky_planet_is_untouched() {
    let mut trace = Trace::new();
    // Earth at 1 au, and at 0.05 au: 400 times the fluence.
    for au in [1.0, CLOSE_AU] {
        let (t, drawn, body) = formed(&mut trace, 1.0, au, 0.0);
        let nodes = trace.len();
        let left = escaped(&t, &drawn, body.clone(), &mut trace).unwrap();
        assert_eq!(left.body, body);
        assert_eq!(left.trial, t);
        assert_eq!(left.drawn, drawn);
        assert_eq!(left.lost, None);
        assert_eq!(trace.len(), nodes, "{au} au");
    }
}

#[test]
fn the_loss_never_exceeds_the_envelope() {
    let mut trace = Trace::new();
    // Legacy's cases: a thin, the reference, a thick envelope and a giant;
    // the fluence from none to 1e6 times Earth's at 0.05 au.
    for (mass, envelope) in [(2.0, 0.005), (5.0, 0.01), (10.0, 0.2), (300.0, 0.9)] {
        let t = trial(&mut trace, 1, mass, CLOSE_AU, 0.0);
        let heavy = (mass >= GAS_GIANT_MIN_MASS.0).then_some(EarthMass(mass * (1.0 - envelope)));
        let drawn = Drawn::fixed(
            &t,
            &mut trace,
            EARTH_CORE_MASS_FRACTION,
            EARTH_WATER_FRACTION,
            0.0,
            None,
            envelope,
            heavy,
            None,
        );
        let body = interior(&t, &drawn, &mut trace).unwrap();
        let earth = science::xuv::EARTH_XUV_FLUENCE.0;
        let core = mass * (1.0 - envelope);
        for times in [0.0, 1.0, 1.0e2, 1.0e4, 1.0e6] {
            let exposed = with_fluence(&mut trace, &t, times * earth);
            let left = escaped(&exposed, &drawn, body.clone(), &mut trace).unwrap();
            let fraction = left.drawn.envelope_fraction.value().get();
            assert!(
                (0.0..=envelope).contains(&fraction),
                "{mass} {times}: {fraction}"
            );
            let kept = left.trial.mass.value().0;
            assert!(kept >= core * (1.0 - ROUNDING), "{mass} {times}: {kept}");
            assert!(kept <= mass * (1.0 + ROUNDING), "{mass} {times}: {kept}");
            assert!(fraction == 0.0 || fraction >= MIN_ENVELOPE, "{fraction}");
            assert_eq!(left.lost.is_none(), times == 0.0, "{mass} {times}");
        }
    }
}
