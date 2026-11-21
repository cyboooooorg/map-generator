//! Sourced physical constants.

use crate::units::{Kilogram, Metre, Second, Watt, WattPerSquareMetre};

/// Definition: IAU 2012 Resolution B2, 149 597 870 700 m exactly.
pub const ASTRONOMICAL_UNIT: Metre = Metre(149_597_870_700.0);

/// Definition: IAU 2015 Resolution B3 nominal solar radius (Prša et al.
/// 2016, AJ 152, 41).
pub const SOLAR_RADIUS: Metre = Metre(6.957e8);

/// Definition: IAU 2015 Resolution B3 nominal solar luminosity (Prša et al.
/// 2016, AJ 152, 41).
pub const SOLAR_LUMINOSITY: Watt = Watt(3.828e26);

/// Definition: IAU 2015 Resolution B3 nominal total solar irradiance at
/// 1 au (Prša et al. 2016, AJ 152, 41). One Earth flux.
pub const SOLAR_IRRADIANCE: WattPerSquareMetre = WattPerSquareMetre(1361.0);

/// Definition: IAU 2015 Resolution B3 nominal solar mass parameter GM, in
/// m^3/s^2 (Prša et al. 2016, AJ 152, 41).
pub const SOLAR_MASS_PARAMETER: f64 = 1.327_124_4e20;

/// Definition: IAU 2015 Resolution B3 nominal Earth equatorial radius
/// (Prša et al. 2016, AJ 152, 41). The mean radius (6371 km) is not this.
pub const EARTH_EQUATORIAL_RADIUS: Metre = Metre(6.378_1e6);

/// Definition: IAU 2015 Resolution B3 nominal Earth mass parameter GM, in
/// m^3/s^2 (Prša et al. 2016, AJ 152, 41).
pub const EARTH_MASS_PARAMETER: f64 = 3.986_004e14;

/// Source: CODATA 2018 Newtonian constant of gravitation, in
/// m^3 kg^-1 s^-2 (Tiesinga et al. 2021, Rev. Mod. Phys. 93, 025010).
pub const GRAVITATIONAL_CONSTANT: f64 = 6.674_30e-11;

/// Source: CODATA 2018 Stefan-Boltzmann constant, in W m^-2 K^-4
/// (Tiesinga et al. 2021, Rev. Mod. Phys. 93, 025010).
pub const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;

/// Definition: the nominal solar mass parameter divided by
/// [`GRAVITATIONAL_CONSTANT`].
pub const SOLAR_MASS: Kilogram = Kilogram(SOLAR_MASS_PARAMETER / GRAVITATIONAL_CONSTANT);

/// Definition: the nominal Earth mass parameter divided by
/// [`GRAVITATIONAL_CONSTANT`].
pub const EARTH_MASS: Kilogram = Kilogram(EARTH_MASS_PARAMETER / GRAVITATIONAL_CONSTANT);

/// Definition: the day of 86 400 SI seconds (IAU and SI).
pub const DAY: Second = Second(86_400.0);

/// Definition: days in a Julian year (IAU).
pub const JULIAN_YEAR_DAYS: f64 = 365.25;

/// Definition: years in a gigayear (SI prefix).
pub const YEARS_PER_GYR: f64 = 1e9;

/// Definition: pascals in a bar (exact, SI-accepted unit).
pub const PASCALS_PER_BAR: f64 = 1e5;

/// Definition: degrees in a half turn, so that a degree is pi/180 rad.
pub const DEGREES_PER_HALF_TURN: f64 = 180.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masses_from_mass_parameters_match_the_published_masses() {
        // Published: Sun 1.98847e30 kg (CODATA/IAU), Earth 5.9722e24 kg.
        assert!((SOLAR_MASS.0 / 1.988_47e30 - 1.0).abs() < 1e-4);
        assert!((EARTH_MASS.0 / 5.972_2e24 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn one_earth_flux_is_the_solar_luminosity_spread_over_one_au() {
        let l = SOLAR_LUMINOSITY.0;
        let a = ASTRONOMICAL_UNIT.0;
        let s = l / (4.0 * std::f64::consts::PI * a * a);
        assert!((s / SOLAR_IRRADIANCE.0 - 1.0).abs() < 1e-3);
    }
}
