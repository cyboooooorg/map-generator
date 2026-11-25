use super::*;
use crate::constants::{EARTH_MEAN_RADIUS, STANDARD_GRAVITY};
use crate::outgassing::{column_mass, column_pressure};
use crate::radiation::gases::PartialPressures;
use crate::surface::titan_like_air;
use crate::volatiles::methane_triple_pressure;

/// Definition: the tolerance of an identity of the law, 1e-12.
const IDENTITY: f64 = 1e-12;

/// Source: Cowan & Abbot 2014 § 2.4: twice the radius holds four times
/// the area, twice the gravity half the depth, 2 × 3.7 = 7.4 oceans.
const TWICE_AND_TWICE: f64 = 7.4;

/// Definition: a methane share for a Titan-like air, 1e-3 bar.
const SOME_METHANE: f64 = 1e-3;

/// Definition: a hair, a relative step of 1e-9 either side of a line.
const HAIR: f64 = 1e-9;

/// Definition: Earth's surface pressure for the inversion, 1 bar.
const ONE_BAR: Bar = Bar(1.0);

#[test]
fn an_earth_holds_three_point_seven_oceans() {
    let earth = basin_capacity_oceans(EarthRadius(1.0), STANDARD_GRAVITY);
    assert!((earth - BASIN_OCEANS).abs() < IDENTITY, "{earth}");
    let big = basin_capacity_oceans(
        EarthRadius(2.0),
        MetrePerSecondSquared(2.0 * STANDARD_GRAVITY.0),
    );
    assert!((big - TWICE_AND_TWICE).abs() < IDENTITY, "{big}");
}

#[test]
fn titan_like_air_reads_the_triple_point_pressure() {
    let triple = methane_triple_pressure().0;
    let air = |total: f64| PartialPressures {
        n2: total - SOME_METHANE,
        ch4: SOME_METHANE,
        ..PartialPressures::default()
    };
    assert!(titan_like_air(&air(triple * (1.0 + HAIR))));
    assert!(!titan_like_air(&air(triple * (1.0 - HAIR))));
}

#[test]
fn column_mass_inverts_column_pressure() {
    let radius = EARTH_MEAN_RADIUS;
    let mass = column_mass(ONE_BAR, STANDARD_GRAVITY, radius);
    let back = column_pressure(mass, STANDARD_GRAVITY, radius);
    assert!((back.0 - ONE_BAR.0).abs() < IDENTITY, "{back:?}");
}
