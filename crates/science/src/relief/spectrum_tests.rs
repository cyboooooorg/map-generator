use super::*;
use crate::constants::STANDARD_GRAVITY;
use crate::units::{Metre, MetrePerSecondSquared};

#[test]
fn persistence_follows_the_hurst_exponent() {
    assert!((persistence(0.5) - 0.5_f64.sqrt()).abs() < 1e-12);
    assert!((persistence(1.0) - 0.5).abs() < 1e-12);
}

#[test]
fn octaves_reach_two_cells() {
    // floor(log2(40 075 km / 30 km)) + 1 = 10 + 1.
    assert_eq!(octaves(Metre(40_075e3), Metre(15e3)), 11);
    assert_eq!(octaves(Metre(1e3), Metre(15e3)), 1);
}

#[test]
fn analogues_keep_their_measured_hurst() {
    assert_eq!(hurst(Analogue::Earth), 0.479);
    assert_eq!(hurst(Analogue::Mars), 0.53);
}

#[test]
fn amplitudes_are_the_measured_haar_fluctuations_at_100_km() {
    assert_eq!(REFERENCE_WAVELENGTH, Metre(100e3));
    assert_eq!(amplitude(Analogue::Earth), Metre(219.0));
    assert_eq!(amplitude(Analogue::Mars), Metre(233.0));
}

/// Ported from `legacy-final:crates/mapgen/src/generation.rs:3491`.
#[test]
fn relief_stands_as_one_over_gravity_up_to_a_cap() {
    assert_eq!(relief_scale(STANDARD_GRAVITY), 1.0);
    assert_eq!(relief_scale(MetrePerSecondSquared(3.71)), MAX_RELIEF_SCALE);
    assert_eq!(MAX_RELIEF_SCALE, 2.49);
    assert!((relief_scale(MetrePerSecondSquared(19.6133)) - 0.5).abs() < 1e-12);
}
