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
    // Definition: at xi = π/2, sin xi − xi cos xi = 1, so m(R/2) = M/π.
    let half = polytrope_mass_within(m, JUPITER_RADIUS, Metre(JUPITER_RADIUS.0 / 2.0));
    assert!((half.0 / (m.0 / PI) - 1.0).abs() < 1e-12);
}

/// Definition: the points of the Lane-Emden checks, `xi` in `(0, π)`.
const XI: [f64; 3] = [0.5, 1.5, 2.5];

/// Definition: the pressure of the index-one polytrope of `mass` and
/// `radius` at `xi`, written from Hubbard 2013 (ApJ 768, 43) Eqs. 61-62
/// and the Lane-Emden solution `theta = sin xi / xi`:
/// `P = K rho_c² theta²`, `K = 2 G R² / π`, `rho_c = (π² / 3) rho_mean`.
fn lane_emden_pressure(mass: Kilogram, radius: Metre, xi: f64) -> Pascal {
    let r = radius.0;
    let k = 2.0 * GRAVITATIONAL_CONSTANT * r * r / PI;
    let mean = mass.0 / (4.0 / 3.0 * PI * r.powi(3));
    let central = PI * PI / 3.0 * mean;
    let theta = xi.sin() / xi;
    Pascal(k * central * central * theta * theta)
}

#[test]
fn the_radius_of_a_pressure_follows_lane_emden() {
    let m = jupiter_mass();
    for xi in XI {
        let p = lane_emden_pressure(m, JUPITER_RADIUS, xi);
        let r = polytrope_radius_at(m, JUPITER_RADIUS, p).unwrap();
        let expected = xi * JUPITER_RADIUS.0 / PI;
        assert!((r.0 / expected - 1.0).abs() < 1e-9, "xi {xi}: {}", r.0);
    }
}

#[test]
fn the_mass_within_the_radius_of_a_pressure_closes_the_round_trip() {
    // The radius read for a pressure, then the mass inside it: the
    // closed form m(xi) = M (sin xi − xi cos xi) / π at that xi.
    let m = jupiter_mass();
    for xi in XI {
        let p = lane_emden_pressure(m, JUPITER_RADIUS, xi);
        let r = polytrope_radius_at(m, JUPITER_RADIUS, p).unwrap();
        let inside = polytrope_mass_within(m, JUPITER_RADIUS, r);
        let expected = m.0 * (xi.sin() - xi * xi.cos()) / PI;
        assert!((inside.0 / expected - 1.0).abs() < 1e-9, "xi {xi}");
    }
}

#[test]
fn a_pressure_at_or_below_zero_is_the_surface_and_a_non_finite_one_none() {
    let m = jupiter_mass();
    for p in [0.0, -1.0] {
        assert_eq!(
            polytrope_radius_at(m, JUPITER_RADIUS, Pascal(p)),
            Some(JUPITER_RADIUS)
        );
    }
    for p in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(polytrope_radius_at(m, JUPITER_RADIUS, Pascal(p)), None);
    }
}
