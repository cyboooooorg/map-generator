use super::*;
use crate::constants::{EARTH_MEAN_RADIUS, STANDARD_GRAVITY};

/// Source: total mean mass of Earth's atmosphere, 5.1480e18 kg
/// (Trenberth & Smith 2005, J. Climate 18, 864-875,
/// doi:10.1175/JCLI-3299.1, abstract, read through Crossref on
/// 2026-11-23).
const TRENBERTH_SMITH_MASS: Kilogram = Kilogram(5.1480e18);

/// Source: global mean total surface pressure, 985.50 hPa (Trenberth &
/// Smith 2005, abstract), in bar.
const TRENBERTH_SMITH_PRESSURE: Bar = Bar(0.985_50);

/// Definition: relative tolerance of the column weight against the mean
/// surface pressure, 1 %: one gravity and one radius stand for the
/// topography and the gravity varying with latitude and height that the
/// paper integrates over.
const COLUMN_TOLERANCE: f64 = 0.01;

/// Definition: relative tolerance of a law checked against its own
/// formula (round-off only).
const ROUND_OFF: f64 = 1e-12;

/// Source: Ganymede's seafloor in the model, 1.1 GPa, and the share of
/// its air it keeps, 98 % or more: the targets of
/// [`SEAFLOOR_HALF_PRESSURE`] (`docs/science/planet-atmospheres.md`,
/// "Under a deep ocean"; study 21, part 5).
const GANYMEDE_SEAFLOOR: Bar = Bar(1.1e4);
/// Source: see [`GANYMEDE_SEAFLOOR`].
const ICY_MOON_KEPT: f64 = 0.98;

/// Definition: a dry column above [`DRY_COLUMN`], 20 bar.
const THICK_DRY: Bar = Bar(20.0);

#[test]
fn a_column_of_earths_air_weighs_its_pressure() {
    let p = column_pressure(TRENBERTH_SMITH_MASS, STANDARD_GRAVITY, EARTH_MEAN_RADIUS);
    let expected = TRENBERTH_SMITH_PRESSURE.0;
    assert!(
        (p.0 / expected - 1.0).abs() < COLUMN_TOLERANCE,
        "{p:?} against {expected} bar"
    );
}

#[test]
fn half_is_released_under_the_half_pressure() {
    let half = released_under_ocean(SEAFLOOR_HALF_PRESSURE).get();
    assert!((half - 0.5).abs() < ROUND_OFF, "{half}");
    // No ocean lets all through; a deeper floor lets less.
    assert_eq!(released_under_ocean(Bar(0.0)).get(), 1.0);
    let deep = released_under_ocean(Bar(SEAFLOOR_HALF_PRESSURE.0 * 2.0)).get();
    let by_hand = 1.0 / (1.0 + 2f64.powf(SEAFLOOR_STEEPNESS));
    assert!((deep / by_hand - 1.0).abs() < ROUND_OFF, "{deep}");
    assert!(deep < half);
}

#[test]
fn an_icy_moon_keeps_its_air_under_its_ocean() {
    let kept = released_under_ocean(GANYMEDE_SEAFLOOR).get();
    assert!(kept >= ICY_MOON_KEPT, "{kept}");
}

#[test]
fn wash_out_shares() {
    // Legacy `wash_out` test (atmosphere.rs at legacy-final): a dry
    // oxygen-free bar keeps a tenth, a thick dry air all, a wet or
    // oxidising air the wet share.
    assert_eq!(
        so2_kept(false, Bar(0.0), Bar(1.0)).get(),
        1.0 / DRY_COLUMN.0
    );
    assert_eq!(so2_kept(false, Bar(0.0), THICK_DRY).get(), 1.0);
    assert_eq!(so2_kept(true, Bar(0.0), Bar(1.0)).get(), WET_SO2_SHARE);
    assert_eq!(so2_kept(false, OXIDISING_O2, Bar(1.0)).get(), WET_SO2_SHARE);
    assert_eq!(so2_kept(true, Bar(0.0), THICK_DRY).get(), WET_SO2_SHARE);
}

/// Definition: a mass of NH3 to decompose, 1e18 kg, a test input.
const AMMONIA: Kilogram = Kilogram(1e18);

#[test]
fn decomposing_ammonia_keeps_its_nitrogen() {
    // `2 NH3 -> N2 + 3 H2`: the N2 made and the NH3 left hold the
    // nitrogen of the NH3 given, and the reaction progress is at most 1
    // (Glein 2015 via Erkaev et al. 2020, eq. 42).
    const { assert!(AMMONIA_DECOMPOSED > 0.0 && AMMONIA_DECOMPOSED <= 1.0) };
    let d = decomposed(AMMONIA);
    let back = d.n2.0 / N2_PER_NH3 + d.nh3.0;
    assert!((back / AMMONIA.0 - 1.0).abs() < ROUND_OFF, "{d:?}");
    assert!((d.nh3.0 / AMMONIA.0 - (1.0 - AMMONIA_DECOMPOSED)).abs() < ROUND_OFF);
}
