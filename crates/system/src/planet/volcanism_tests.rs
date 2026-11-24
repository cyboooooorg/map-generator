//! Tier 1: the volcanic intensity recorded in the bare chain.

use super::bare::bare_with;
use super::codes;
use super::tests::{EARTH, MARS, derived_from, reference_trial};
use super::volcanism::volcanism;
use science::outgassing::{LID_SHARE, MOBILE_SHARE};
use science::relief::volcanoes::{heat_flux, volcanic_intensity};
use science::units::{Metre, Watt};
use timeline::{Code, Heat, Provenance, TectonicRegime, Trace};

/// Definition: the tolerance of a value recomputed from the same inputs
/// by the same law, 1e-12 (a test choice).
const SAME: f64 = 1e-12;

/// The intensity the formula gives for a bare reference body, and the
/// regime and node the chain recorded.
fn recorded(body: &super::tests::ReferenceBody) -> (f64, f64, TectonicRegime) {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, body);
    let b = bare_with(&t, &d, &mut trace).unwrap();
    let s = b.surface.as_ref().unwrap();
    let radius = Metre::from(*b.body.bulk().radius.value());
    let power =
        Watt(b.heat.radiogenic.value().0 + b.heat.tidal.value().0 + b.heat.secular.value().0);
    let mut intensity = volcanic_intensity(heat_flux(power, radius)).get();
    if *s.regime.value() == TectonicRegime::Stagnant {
        intensity *= LID_SHARE / MOBILE_SHARE;
    }
    let expected = vec![
        b.heat.radiogenic.node(),
        b.heat.tidal.node(),
        b.heat.secular.node(),
        b.body.bulk().radius.node(),
        s.regime.node(),
    ];
    // The node derives from exactly the five nodes, in order.
    assert_eq!(derived_from(&trace, s.volcanism.node()), expected);
    assert_eq!(
        trace.node(s.volcanism.node()).unwrap().code,
        Code::of(codes::SURFACE_VOLCANISM)
    );
    (s.volcanism.value().get(), intensity, *s.regime.value())
}

#[test]
fn volcanism_reads_the_regime() {
    let (earth, earth_formula, earth_regime) = recorded(&EARTH);
    let (mars, mars_formula, mars_regime) = recorded(&MARS);
    // The precondition: the two regimes the test is about.
    assert_eq!(earth_regime, TectonicRegime::Plates);
    assert_eq!(mars_regime, TectonicRegime::Stagnant);
    assert!((earth - earth_formula).abs() < SAME);
    assert!((mars - mars_formula).abs() < SAME);
    assert!(mars < earth);
}

#[test]
fn a_heat_pipe_reads_the_law() {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, &EARTH);
    let b = bare_with(&t, &d, &mut trace).unwrap();
    let radius = Metre::from(*b.body.bulk().radius.value());
    let regime = trace
        .leaf(
            Code::of(codes::REGIME),
            TectonicRegime::HeatPipe,
            Provenance::Input,
        )
        .unwrap();
    // Definition: the surface flux of a heat pipe, 1.5 W/m2 (above the
    // regime's 1 W/m2, below the law's 2.5 W/m2) and 3 W/m2 (above both).
    for (flux, saturated) in [(1.5, false), (3.0, true)] {
        let power = Watt(flux * 4.0 * std::f64::consts::PI * radius.0 * radius.0);
        let mut leaf = |code, w| trace.leaf(Code::of(code), w, Provenance::Input).unwrap();
        let heat = Heat {
            radiogenic: leaf("test.radiogenic", power),
            tidal: leaf("test.tidal", Watt(0.0)),
            secular: leaf("test.secular", Watt(0.0)),
        };
        let v = volcanism(&b.body, &heat, &regime, &mut trace).unwrap();
        let law = volcanic_intensity(science::units::WattPerSquareMetre(flux)).get();
        assert!((v.value().get() - law).abs() < SAME);
        assert_eq!(saturated, (v.value().get() - 1.0).abs() < SAME);
        assert!(saturated || v.value().get() < 1.0);
    }
}
