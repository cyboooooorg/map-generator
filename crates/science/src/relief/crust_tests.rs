use super::*;
use crate::units::{Degree, Metre, Radian};

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
fn constants_match_their_sources() {
    // The slope is written in radians from ETOPO 2022's 2.4° (the relief
    // study): the hand conversion agrees with the units crate's.
    assert!((CONTINENTAL_SLOPE.0 - Radian::from(Degree(2.4)).0).abs() < 1e-12);
}
