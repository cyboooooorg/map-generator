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
/// (Earth Planet. Sci. Lett. 313-314, 56, p. 18).
const MARTY_BULK_WATER: f64 = 2.7e-3;

/// Definition: the rounding of the ocean unit, Charette & Smith 2010's
/// 1.37e21 kg rounded to 1.4e21 kg, 2.2 %, taken as 3 %.
const OCEAN_ROUNDING: f64 = 0.03;

#[test]
fn the_sun_runs_away_at_1_107() {
    assert_eq!(runaway_flux(Kelvin(5_780.0)).0, KOPPARAPU_SOLAR_RUNAWAY);
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
fn an_earth_drawn_at_martys_bulk_water_gets_one_ocean() {
    let surface = SURFACE_WATER_SHARE * MARTY_BULK_WATER * EARTH_MASS.0;
    let oceans = surface / OCEAN_MASS.0;
    assert!((oceans - 1.0).abs() < OCEAN_ROUNDING, "{oceans}");
}
