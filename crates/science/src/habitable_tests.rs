use super::*;
use crate::constants::EARTH_MASS;

/// Source: the runaway greenhouse flux of a Sun-like star in Kopparapu et
/// al. 2014 (ApJL 787, L29, doi:10.1088/2041-8205/787/2/L29), Table 1,
/// column `S_eff,sun`, for 1 M_earth.
const KOPPARAPU_SOLAR_RUNAWAY: f64 = 1.107;

/// Source: the fit's lower limit, 2 600 K, Kopparapu et al. 2014, as
/// stated in `docs/science/habitable-zone.md`.
const FIT_MIN: Kelvin = Kelvin(2_600.0);

/// Source: the fit's upper limit, 7 200 K, same page.
const FIT_MAX: Kelvin = Kelvin(7_200.0);

/// Source: Earth's bulk water drawn at Marty 2012's 2700 ppm of the mass
/// (Earth Planet. Sci. Lett. 313-314, 56-66; arXiv:1405.6336 author
/// copy, p. 18).
const MARTY_BULK_WATER: f64 = 2.7e-3;

/// Source: the volume of Earth's ocean, 1.332e9 km³ (Charette & Smith
/// 2010, Oceanography 23(2), 112, doi:10.5670/oceanog.2010.51, Table 1),
/// in m³.
const CHARETTE_SMITH_OCEAN_VOLUME: f64 = 1.332e18;

/// Source: a usual density of sea water, 1027 kg/m³ (the one the ocean
/// unit's doc comment reads Charette & Smith's volume at).
const SEA_WATER_DENSITY: f64 = 1_027.0;

/// Definition: a test tolerance, relative, 3 %: it covers the two-digit
/// rounding of Cowan & Abbot's 2.3e-4 (±0.05e-4, 2.2 %), the share's
/// numerator, against an ocean mass computed from another source.
const TWO_DIGIT_ROUNDING: f64 = 0.03;

#[test]
fn the_sun_runs_away_at_1_107() {
    assert_eq!(runaway_flux(Kelvin(5_780.0)).0, KOPPARAPU_SOLAR_RUNAWAY);
}

/// Source: the maximum greenhouse flux of a Sun-like star in Kopparapu et
/// al. 2014, Table 1, column `S_eff,sun` of the maximum greenhouse, for
/// 1 M_earth.
const KOPPARAPU_SOLAR_OUTER: f64 = 0.356;

/// Source: the Sun's effective temperature in IAU 2015 Resolution B3,
/// 5772 K.
const IAU_SUN_TEMPERATURE: Kelvin = Kelvin(5_772.0);

/// Source: the outer edge round the Sun, 1.676 AU, of study 25 (Wiki,
/// 25-climate-solver-audit) from Kopparapu et al. 2014; the edge is
/// `1 / sqrt(S)` for a solar luminosity.
const STUDY_OUTER_EDGE_AU: f64 = 1.676;

/// Definition: the tolerance on that edge, 0.002 AU, the rounding of the
/// study's three-digit figure and of the Sun's temperature.
const OUTER_EDGE_TOLERANCE_AU: f64 = 0.002;

/// Source: the flux of a 3300 K star, 0.2411, the published coefficients
/// of Kopparapu et al. 2014, Table 1 (maximum greenhouse), evaluated by
/// hand at `T = 3300 - 5780 K`.
const KOPPARAPU_OUTER_AT_3300: f64 = 0.2411;

/// Definition: the tolerance on that hand evaluation, 5e-4, its four
/// digits.
const HAND_EVALUATION_TOLERANCE: f64 = 5e-4;

#[test]
fn the_outer_edge_is_kopparapus() {
    assert!((maximum_greenhouse_flux(Kelvin(5_780.0)).0 - KOPPARAPU_SOLAR_OUTER).abs() < 1e-12);
    let edge = 1.0 / maximum_greenhouse_flux(IAU_SUN_TEMPERATURE).0.sqrt();
    assert!(
        (edge - STUDY_OUTER_EDGE_AU).abs() < OUTER_EDGE_TOLERANCE_AU,
        "{edge}"
    );
    let cool = maximum_greenhouse_flux(Kelvin(3_300.0)).0;
    assert!(
        (cool - KOPPARAPU_OUTER_AT_3300).abs() < HAND_EVALUATION_TOLERANCE,
        "{cool}"
    );

    // Definition: the fit's range in 100 K steps, 2600 to 7200 K.
    let mut previous = 0.0;
    for step in 0..=46 {
        let teff = Kelvin(FIT_MIN.0 + 100.0 * f64::from(step));
        let outer = maximum_greenhouse_flux(teff).0;
        assert!(outer > previous, "not rising at {teff:?}");
        assert!(
            outer < runaway_flux(teff).0,
            "not below the runaway at {teff:?}"
        );
        previous = outer;
    }

    assert_eq!(
        maximum_greenhouse_flux(Kelvin(2_000.0)),
        maximum_greenhouse_flux(FIT_MIN)
    );
    assert_eq!(
        maximum_greenhouse_flux(Kelvin(9_000.0)),
        maximum_greenhouse_flux(FIT_MAX)
    );
}

#[test]
fn the_fit_is_clamped_to_its_range() {
    assert_eq!(runaway_flux(Kelvin(2_000.0)), runaway_flux(FIT_MIN));
    assert_eq!(runaway_flux(Kelvin(9_000.0)), runaway_flux(FIT_MAX));
}

#[test]
fn the_ocean_is_the_volume_of_earths_at_a_sea_water_density() {
    // Charette & Smith 2010: 1.332e9 km3; at 1027 kg/m3 (a usual value of
    // sea water) it is 1.37e21 kg, which 1.4e21 rounds up.
    let kg = 1.332e9 * 1.0e9 * 1_027.0;
    assert!((OCEAN_MASS.0 / kg - 1.0).abs() < 0.03);
}

#[test]
fn an_earth_drawn_at_martys_bulk_water_holds_earths_ocean() {
    // The share's numerator is Cowan & Abbot's ocean fraction; the ocean
    // expected here is Charette & Smith's volume at a sea-water density.
    let surface = SURFACE_WATER_SHARE * MARTY_BULK_WATER * EARTH_MASS.0;
    let ocean = CHARETTE_SMITH_OCEAN_VOLUME * SEA_WATER_DENSITY;
    assert!(
        (surface / ocean - 1.0).abs() < TWO_DIGIT_ROUNDING,
        "{surface} {ocean}"
    );
}
