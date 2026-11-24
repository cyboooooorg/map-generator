//! Sourced physical constants.

use crate::units::{
    Gyr, Kelvin, Kilogram, Metre, MetrePerSecond, MetrePerSecondSquared, Second, Watt,
    WattPerSquareMetre,
};

/// Definition: IAU 2012 Resolution B2, 149 597 870 700 m exactly.
pub const ASTRONOMICAL_UNIT: Metre = Metre(149_597_870_700.0);

/// Definition: standard acceleration of gravity, 9.806 65 m/s2 exactly (CGPM
/// 1901, 3rd General Conference on Weights and Measures).
pub const STANDARD_GRAVITY: MetrePerSecondSquared = MetrePerSecondSquared(9.806_65);

/// Source: Mars's mean surface gravity, 3.73 m/s² (NASA Mars Fact Sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html, "Surface gravity
/// (mean)"). The sheet's other two values are surface accelerations,
/// rotation included: 3.69 at the equator, 3.73 at the pole.
pub const MARS_SURFACE_GRAVITY: MetrePerSecondSquared = MetrePerSecondSquared(3.73);

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

/// Definition: IAU 2015 Resolution B3 nominal Jupiter mass parameter GM, in
/// m^3/s^2 (Prša et al. 2016, AJ 152, 41). Divided by
/// [`EARTH_MASS_PARAMETER`] it gives Jupiter's mass in Earth masses
/// (317.83), with no dependence on G.
pub const JUPITER_MASS_PARAMETER: f64 = 1.266_865_3e17;

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

/// Definition: a centimetre per Julian year in metres per second, 0.01 m
/// over [`JULIAN_YEAR_DAYS`] days of [`DAY`].
pub const CENTIMETRE_PER_YEAR: MetrePerSecond = MetrePerSecond(0.01 / (JULIAN_YEAR_DAYS * DAY.0));

/// Definition: years in a gigayear (SI prefix).
pub const YEARS_PER_GYR: f64 = 1e9;

/// Source: Boltzmann constant, 1.380 649e-23 J/K, exact since the SI
/// redefinition of 2019 (CODATA 2018, Tiesinga et al. 2021, Rev. Mod.
/// Phys. 93, 025010; NIST CODATA value `k`).
pub const BOLTZMANN: f64 = 1.380_649e-23;

/// Source: Avogadro constant, 6.022 140 76e23 /mol, exact since the SI
/// redefinition of 2019 (CODATA 2018, Tiesinga et al. 2021, Rev. Mod.
/// Phys. 93, 025010; NIST CODATA value `N_A`).
pub const AVOGADRO: f64 = 6.022_140_76e23;

/// Source: Loschmidt constant `n0 = p0 / (k T0)` at 273.15 K and
/// 101.325 kPa, 2.686 780 111e25 m^-3, exact (CODATA 2018, Tiesinga et al.
/// 2021, Rev. Mod. Phys. 93, 025010; NIST CODATA value `n_0`, read on
/// physics.nist.gov/cuu/Constants/ as `docs/science/radiative-transfer.md`
/// cites it).
pub const LOSCHMIDT: f64 = 2.686_780_111e25;

/// Definition: speed of light in vacuum, 299 792 458 m/s exactly (SI,
/// 17th CGPM 1983).
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;

/// Definition: Planck constant, 6.626 070 15e-34 J s exactly since the SI
/// redefinition of 2019 (CODATA 2022; NIST CODATA value `h`).
pub const PLANCK: f64 = 6.626_070_15e-34;

/// Definition: molar gas constant `R = k N_A`, J/(mol K), exact
/// (CODATA 2018 prints 8.314 462 618... J/(mol K)); the product of
/// [`BOLTZMANN`] and [`AVOGADRO`] to its last digit.
pub const MOLAR_GAS_CONSTANT: f64 = 8.314_462_618_153_24;

/// Source: second radiation constant `c2 = h c / k`, 1.438 776 877e-2 m K
/// (exact, CODATA 2018, Tiesinga et al. 2021, Rev. Mod. Phys. 93, 025010;
/// NIST CODATA value `c_2`), here in cm K for wavenumbers in cm^-1, to
/// its printed digits.
pub const SECOND_RADIATION_CONSTANT_CM_K: f64 = 1.438_776_877;

/// Definition: pascals in a bar (exact, SI-accepted unit).
pub const PASCALS_PER_BAR: f64 = 1e5;

/// Definition: degrees in a half turn, so that a degree is pi/180 rad.
pub const DEGREES_PER_HALF_TURN: f64 = 180.0;

/// Source: IUGG mean radius R1 = (2a + b) / 3 of the GRS80 ellipsoid,
/// 6 371 008.7714 m (Moritz 2000, "Geodetic Reference System 1980",
/// J. Geodesy 74, 128-162; value read on the Wikipedia page of GRS80, which
/// quotes it and cites that paper). Earth's volumetric mean radius is
/// 6 371 000.79 m, 8 m smaller; no law here is sensitive to that.
pub const EARTH_MEAN_RADIUS: Metre = Metre(6_371_008.771_4);

/// Definition: IAU 2015 Resolution B3 nominal solar effective temperature,
/// 5772 K (Prša et al. 2016, AJ 152, 41).
pub const SOLAR_EFFECTIVE_TEMPERATURE: Kelvin = Kelvin(5772.0);

/// Source: age of the Universe, 13.8 Gyr (Planck 2018 results VI, A&A 641,
/// A6: 13.787 +/- 0.020 Gyr, TT,TE,EE+lowE+lensing+BAO).
pub const AGE_OF_UNIVERSE: Gyr = Gyr(13.8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masses_from_mass_parameters_match_the_published_masses() {
        // Published: Sun 1.98847e30 kg (CODATA/IAU), Earth 5.9722e24 kg.
        assert!((SOLAR_MASS.0 / 1.988_47e30 - 1.0).abs() < 1e-4);
        assert!((EARTH_MASS.0 / 5.972_2e24 - 1.0).abs() < 1e-4);
    }

    /// Definition: exact SI 2019 value of the Planck constant, J s.
    const PLANCK: f64 = 6.626_070_15e-34;
    /// Definition: exact SI value of the speed of light, m/s.
    const LIGHT_SPEED: f64 = 299_792_458.0;

    fn close(value: f64, published: f64, tolerance: f64) {
        assert!(
            (value / published - 1.0).abs() < tolerance,
            "{value} against {published}"
        );
    }

    #[test]
    fn every_constant_matches_its_published_value() {
        let rows: [(&str, f64, f64); 8] = [
            // Source: IAU 2012 Resolution B2, exact.
            ("ASTRONOMICAL_UNIT", ASTRONOMICAL_UNIT.0, 149_597_870_700.0),
            // Source: IAU 2015 Resolution B3, nominal solar radius.
            ("SOLAR_RADIUS", SOLAR_RADIUS.0, 6.957e8),
            // Source: IAU 2015 Resolution B3, nominal solar luminosity.
            ("SOLAR_LUMINOSITY", SOLAR_LUMINOSITY.0, 3.828e26),
            // Source: IAU 2015 Resolution B3, nominal irradiance.
            ("SOLAR_IRRADIANCE", SOLAR_IRRADIANCE.0, 1361.0),
            // Source: IAU 2015 Resolution B3, nominal GM_sun.
            ("SOLAR_MASS_PARAMETER", SOLAR_MASS_PARAMETER, 1.327_124_4e20),
            // Source: IAU 2015 Resolution B3, nominal equatorial radius.
            (
                "EARTH_EQUATORIAL_RADIUS",
                EARTH_EQUATORIAL_RADIUS.0,
                6.378_1e6,
            ),
            // Source: IAU 2015 Resolution B3, nominal GM_earth.
            ("EARTH_MASS_PARAMETER", EARTH_MASS_PARAMETER, 3.986_004e14),
            // Source: CODATA 2018, G.
            (
                "GRAVITATIONAL_CONSTANT",
                GRAVITATIONAL_CONSTANT,
                6.674_30e-11,
            ),
        ];
        for (name, value, published) in rows {
            assert_eq!(value, published, "{name}");
        }
    }

    #[test]
    fn the_stefan_boltzmann_constant_follows_from_k_h_and_c() {
        // sigma = 2 pi^5 k^4 / (15 h^3 c^2), computed from the exact SI
        // 2019 values, independently of the stored number.
        let sigma = 2.0 * std::f64::consts::PI.powi(5) * BOLTZMANN.powi(4)
            / (15.0 * PLANCK.powi(3) * LIGHT_SPEED.powi(2));
        close(STEFAN_BOLTZMANN, sigma, 1e-9);
    }

    #[test]
    fn the_gas_and_radiation_constants_follow_from_k_h_and_c() {
        close(MOLAR_GAS_CONSTANT, BOLTZMANN * AVOGADRO, 1e-15);
        // h c / k, in m K, converted to cm K.
        let c2_cm_k = PLANCK * LIGHT_SPEED / BOLTZMANN * 100.0;
        close(SECOND_RADIATION_CONSTANT_CM_K, c2_cm_k, 1e-9);
    }

    /// A consistency check between the stored irradiance and luminosity,
    /// not a published value: the flux of the nominal Sun at 1 au.
    #[test]
    fn one_earth_flux_is_the_solar_luminosity_spread_over_one_au() {
        let l = SOLAR_LUMINOSITY.0;
        let a = ASTRONOMICAL_UNIT.0;
        let s = l / (4.0 * std::f64::consts::PI * a * a);
        assert!((s / SOLAR_IRRADIANCE.0 - 1.0).abs() < 1e-3);
    }
}
