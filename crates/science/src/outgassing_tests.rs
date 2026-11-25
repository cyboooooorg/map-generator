use super::*;
use crate::constants::{EARTH_MEAN_RADIUS, STANDARD_GRAVITY};
use crate::heat::{EARTH_AGE, EARTH_SILICATE_MASS};
use crate::units::Gyr;

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

/// Source: the branches of K-40 to Ar-40 (DDEP evaluation of 2025, X.
/// Mougeot, ICRM 2025 poster, HAL cea-05087440): electron capture to the
/// 1461 keV level of Ar-40, 10.34 %, to its ground state, 0.098 %, and
/// beta+, 0.00103 %, in percent.
const DDEP_ARGON_BRANCHES: [f64; 3] = [10.34, 0.098, 0.00103];

/// Source: the Ar-40 made in Earth's silicate part over 4.5 Gyr, 3.9 ±
/// 0.9e18 mol, from 280 ± 60 ppm of potassium (Marty 2012, EPSL 313-314,
/// 56, arXiv:1405.6336, p. 15), as its 1σ ends.
const MARTY_ARGON_MADE: (f64, f64) = (3.0e18, 4.8e18);

/// Source: the Ar-40 the law makes on Earth, 4.053e18 mol: study 29's
/// own computation of the same formula from DDEP 2025's decay data
/// (`docs/science/planet-atmospheres.md`, "Argon"), to its four digits.
const STUDY_ARGON_MADE: f64 = 4.053e18;

/// Definition: the tolerance on [`STUDY_ARGON_MADE`], half its last digit
/// and the rounding of the inputs the study printed, 0.5 %.
const STUDY_TOLERANCE: f64 = 5e-3;

/// Source: Earth's argon as a weight, 0.0127 bar: 9340 ppm of the dry
/// air by volume (NASA Earth fact sheet) is 1.288 % of its 5.1352e18 kg
/// (Trenberth & Smith 2005) at 39.95 against 28.97 g/mol, 6.61e16 kg on
/// a sphere of 6371 km at 9.80665 m/s² (study 29, derived).
const EARTH_ARGON: Bar = Bar(0.0127);

/// Definition: the test's tolerance round [`EARTH_ARGON`], 10 %, study
/// 29's: the share degassed is fitted on Earth's air (Marty's mass
/// balance), so the test checks the chain, as the N2 test does.
const EARTH_ARGON_TOLERANCE: f64 = 0.1;

#[test]
fn argon_40_takes_ddeps_branches() {
    let sum: f64 = DDEP_ARGON_BRANCHES.iter().sum::<f64>() / 100.0;
    assert!((ARGON_40_BRANCH / sum - 1.0).abs() < 1e-4, "{sum}");
}

#[test]
fn earths_rock_made_martys_argon() {
    let made = radiogenic_argon(EARTH_SILICATE_MASS, 1.0, EARTH_AGE).0;
    assert!(
        (MARTY_ARGON_MADE.0..=MARTY_ARGON_MADE.1).contains(&made),
        "{made}"
    );
    assert!(
        (made / STUDY_ARGON_MADE - 1.0).abs() < STUDY_TOLERANCE,
        "{made}"
    );
    // Twice the rock, twice the abundance: four times the argon; none at
    // formation.
    let more = radiogenic_argon(EARTH_SILICATE_MASS * 2.0, 2.0, EARTH_AGE).0;
    assert!((more / made - 4.0).abs() < ROUND_OFF, "{more}");
    assert_eq!(radiogenic_argon(EARTH_SILICATE_MASS, 1.0, Gyr(0.0)).0, 0.0);
}

#[test]
fn earths_air_weighs_its_argon() {
    let made = radiogenic_argon(EARTH_SILICATE_MASS, 1.0, EARTH_AGE);
    let air = argon_mass(made) * argon_degassed_share(false).get();
    let p = column_pressure(air, STANDARD_GRAVITY, EARTH_MEAN_RADIUS).0;
    assert!(
        (p / EARTH_ARGON.0 - 1.0).abs() < EARTH_ARGON_TOLERANCE,
        "{p}"
    );
}

#[test]
fn a_lid_degasses_lid_share_over_mobile_share_of_the_plates_argon() {
    // Marty 2012's 0.41 under plates, scaled for a lid by
    // `LID_SHARE / MOBILE_SHARE`.
    assert_eq!(
        argon_degassed_share(false).get(),
        ARGON_DEGASSED_UNDER_PLATES
    );
    let lid = argon_degassed_share(true).get();
    let ratio = ARGON_DEGASSED_UNDER_PLATES * LID_SHARE / MOBILE_SHARE;
    assert!((lid / ratio - 1.0).abs() < ROUND_OFF, "{lid}");
    /// Source: study 29's lid share, 0.0863, to its last printed digit
    /// (`docs/science/planet-atmospheres.md`, "Argon", the table of the
    /// share degassed).
    const STUDY_LID_SHARE: f64 = 0.0863;
    assert!((lid - STUDY_LID_SHARE).abs() < 5e-5, "{lid}");
}
