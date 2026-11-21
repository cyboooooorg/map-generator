use super::*;
use crate::constants::JUPITER_MASS_PARAMETER;

/// Source: Jupiter's equatorial radius at 1 bar, 71 492 km (NASA Jupiter
/// Fact Sheet; study `giants-and-envelopes.md` §1.6).
const JUPITER_RADIUS: Metre = Metre(71_492e3);

/// Source: Debras & Chabrier 2019 (arXiv:1901.05697), the inhomogeneous
/// region of 0.1 to 2 Mbar, from about 0.93 of Jupiter's radius down by 10
/// to 15 % of it: 0.80 to 0.93 R (study §5.3).
const DEBRAS_CHABRIER_REGION: (f64, f64) = (0.80, 0.93);

fn jupiter_mass() -> Kilogram {
    Kilogram(JUPITER_MASS_PARAMETER / GRAVITATIONAL_CONSTANT)
}

#[test]
fn jupiter_1_mbar_lies_in_debras_and_chabrier_region() {
    let r = polytrope_radius_at(jupiter_mass(), JUPITER_RADIUS, METALLIC_PRESSURE).unwrap();
    let share = r.0 / JUPITER_RADIUS.0;
    let (low, high) = DEBRAS_CHABRIER_REGION;
    assert!(share > low && share < high, "1 Mbar at {share} R");
}

#[test]
fn a_pressure_above_the_centre_is_never_reached() {
    // Definition: a pressure far above any planet's centre, 1e20 Pa.
    let r = polytrope_radius_at(jupiter_mass(), JUPITER_RADIUS, Pascal(1e20));
    assert_eq!(r, None);
}

#[test]
fn the_polytrope_holds_the_whole_mass() {
    // Definition: m(π) = M, m(0) = 0, by the integral of the solution.
    let m = jupiter_mass();
    let whole = polytrope_mass_within(m, JUPITER_RADIUS, JUPITER_RADIUS);
    assert!((whole.0 / m.0 - 1.0).abs() < 1e-12);
    assert_eq!(polytrope_mass_within(m, JUPITER_RADIUS, Metre(0.0)).0, 0.0);
}
