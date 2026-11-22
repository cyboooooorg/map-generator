use super::*;
use crate::relief::spectrum::Analogue;
use crate::units::{Degree, KilogramPerCubicMetre, Metre, Radian};

/// Airy step of a crust: its continental column over its oceanic one.
fn step(crust: &Crust) -> Metre {
    floating_height(crust, crust.continental_thickness)
}

/// Ported from `legacy-final:crates/mapgen/src/generation.rs:3352`; the
/// legacy mapping from noise to thickness is not ported, so only the Airy
/// part and the two crusts remain.
#[test]
fn the_crust_floats_by_its_thickness() {
    // Earth: (41.1 − 7.1 km)(1 − 2830/3380) = 5.53 km (Christensen &
    // Mooney 1995; White et al. 1992; PREM's LID).
    let earth = step(&EARTH_CRUST);
    assert!(earth.0 > 3.5e3 && earth.0 < 6.0e3, "{earth:?}");
    assert!((earth.0 - 5.53e3).abs() < 5.0, "{earth:?}");
    // Oceanic crust is the datum.
    assert_eq!(
        floating_height(&EARTH_CRUST, EARTH_CRUST.oceanic_thickness),
        Metre(0.0)
    );
    // Mars: highlands over lowlands, (58 − 32 km)(1 − 2900/3500) = 4.46 km,
    // the order of the dichotomy (Neumann et al. 2004).
    let mars = step(&MARS_CRUST);
    assert!((3.5e3..6.0e3).contains(&mars.0), "{mars:?}");
    assert!((mars.0 - 4.457e3).abs() < 5.0, "{mars:?}");
    // Thinner than the oceanic crust, the column sinks below the datum.
    assert!(floating_height(&EARTH_CRUST, Metre(5.0e3)).0 < 0.0);
}

#[test]
fn crusts_keep_their_cited_values() {
    assert_eq!(EARTH_CRUST.continental_thickness, Metre(41.1e3));
    assert_eq!(EARTH_CRUST.oceanic_thickness, Metre(7.1e3));
    assert_eq!(EARTH_CRUST.crust_density, KilogramPerCubicMetre(2830.0));
    assert_eq!(EARTH_CRUST.mantle_density, KilogramPerCubicMetre(3380.0));
    assert_eq!(MARS_CRUST.continental_thickness, Metre(58e3));
    assert_eq!(MARS_CRUST.oceanic_thickness, Metre(32e3));
    assert_eq!(MARS_CRUST.crust_density, KilogramPerCubicMetre(2900.0));
    assert_eq!(MARS_CRUST.mantle_density, KilogramPerCubicMetre(3500.0));
    assert_eq!(crust(Analogue::Earth), EARTH_CRUST);
    assert_eq!(crust(Analogue::Mars), MARS_CRUST);
}

#[test]
fn continental_shares_are_the_measured_ones() {
    // Cogley 1984; 1 − 0.42, Marinova, Aharonson & Asphaug 2008.
    assert_eq!(continental_share(Analogue::Earth).get(), 0.41);
    assert_eq!(continental_share(Analogue::Mars).get(), 0.58);
}

#[test]
fn the_continental_slope_is_measured_on_etopo() {
    // 2.4° on ETOPO 2022 (the relief study).
    assert!((CONTINENTAL_SLOPE.0 - Radian::from(Degree(2.4)).0).abs() < 1e-12);
}
