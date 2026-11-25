//! The bodies and the Earth case the radiation tests share, each defined
//! once.

use crate::constants::GRAVITATIONAL_CONSTANT;
use crate::radiation::gases::PartialPressures;
use crate::units::MetrePerSecondSquared;

/// Source: Venus's surface gravity, 8.87 m/s² (NASA Venus Fact Sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html, "Surface
/// gravity (mean)", read 2026-11-22).
pub(crate) const VENUS_GRAVITY: MetrePerSecondSquared = MetrePerSecondSquared(8.87);

/// Source: Titan's surface gravity, `G M / R²` from its mass, 1345.5e20
/// kg, and mean radius, 2575 km (NASA Saturnian Satellite Fact Sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/saturniansatfact.html, read
/// 2026-11-22): 1.354 m/s² (the legacy took 1.352).
pub(crate) const TITAN_GRAVITY: MetrePerSecondSquared =
    MetrePerSecondSquared(GRAVITATIONAL_CONSTANT * 1345.5e20 / (2575e3 * 2575e3));

/// Definition: Earth's surface temperature of the test cases, K, its mean
/// surface temperature rounded (the U.S. Standard Atmosphere 1976 takes
/// 288.15 K at sea level).
pub(crate) const EARTH_SURFACE_K: f64 = 288.0;

/// Definition: Earth's dry air of the test columns, the balance's
/// [`crate::balance::EARTH_DRY_AIR`]: the air of the moist plateau and of
/// the shortwave paths.
pub(crate) const EARTH_AIR: PartialPressures = crate::balance::EARTH_DRY_AIR;

/// Source: Titan's surface pressure, 1.467 bar (1467 hPa; Fulchignoni et
/// al. 2005, *Nature* 438, 785, doi:10.1038/nature04314, as the audit,
/// Wiki study 19, confirms).
pub(crate) const TITAN_PRESSURE_BAR: f64 = 1.467;

/// Source: Titan's surface temperature, 93.65 K (+-0.25 K; Fulchignoni et
/// al. 2005, *Nature* 438, 785, doi:10.1038/nature04314, as the audit
/// confirms).
pub(crate) const TITAN_SURFACE_K: f64 = 93.65;

/// Source: the methane share of Titan's air near its surface, 5.65 % (+-
/// 0.18 %; Niemann et al. 2010, *J. Geophys. Res. Planets* 115, E12006,
/// doi:10.1029/2010JE003659, as the audit confirms).
pub(crate) const TITAN_METHANE_SHARE: f64 = 0.0565;

/// Source: Titan's effective temperature, 82 K (McKay, Pollack & Courtin
/// 1991, *Science* 253, 1118, doi:10.1126/science.11538492, as the audit
/// quotes it).
pub(crate) const TITAN_EFFECTIVE_K: f64 = 82.0;

/// Definition: Titan's air at its surface: [`TITAN_METHANE_SHARE`] of methane, the
/// rest nitrogen, under [`TITAN_PRESSURE_BAR`].
pub(crate) const TITAN_SURFACE_AIR: PartialPressures = PartialPressures {
    n2: TITAN_PRESSURE_BAR * (1.0 - TITAN_METHANE_SHARE),
    co2: 0.0,
    h2o: 0.0,
    ch4: TITAN_PRESSURE_BAR * TITAN_METHANE_SHARE,
    nh3: 0.0,
    so2: 0.0,
    o2: 0.0,
};

/// Source: Venus's surface pressure, 92 bar (NASA Venus Fact Sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html, "Surface
/// pressure", read 2026-11-23).
pub(crate) const VENUS_PRESSURE_BAR: f64 = 92.0;

/// Source: Venus's mean surface temperature, 737 K (NASA Venus Fact
/// Sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html,
/// "Average temperature", read 2026-11-23).
pub(crate) const VENUS_SURFACE_K: f64 = 737.0;

/// Source: the SO₂ of Venus's near-surface air, 150 ppm by volume (NASA
/// Venus Fact Sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html,
/// "Surface composition", read 2026-11-23), times [`VENUS_PRESSURE_BAR`]:
/// 1.38e-2 bar.
pub(crate) const VENUS_SO2_BAR: f64 = VENUS_PRESSURE_BAR * 150e-6;

/// Source: the global mean methane of Earth's air, 1936.37 ppb (August
/// 2026; NOAA Global Monitoring Laboratory, gml.noaa.gov/ccgg/trends_ch4/,
/// "Global CH4 Monthly Means", page last updated 2026-11-19, read
/// 2026-11-23), as a mixing ratio of 1.93637e-6 (the tests' 1.9 ppm).
pub(crate) const EARTH_METHANE_MIXING_RATIO: f64 = 1936.37e-9;

/// Source: Earth's clear-sky surface albedo, the reflected over the
/// incoming shortwave at the surface under clear skies of CERES EBAF
/// Ed4.0, 29.8 / 243.8 W/m² = 0.122 (Kato et al. 2018, *J. Climate* 31,
/// 4501, doi:10.1175/JCLI-D-17-0208.1, Table 5), the ground the shortwave
/// tests put under Earth's air.
pub(crate) const EARTH_GROUND_ALBEDO: f64 = 0.122;

/// Definition: the sunlight on Earth averaged over the globe, W/m², a
/// quarter of `SOLAR_IRRADIANCE`: 340.25.
pub(crate) const EARTH_INCOMING_W_M2: f64 = crate::constants::SOLAR_IRRADIANCE.0 / 4.0;

/// Source: Earth's mean surface pressure, 1.014 bar (NASA Earth fact
/// sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html, as
/// `system`'s settled tests take it).
pub(crate) const EARTH_PRESSURE_BAR: f64 = 1.014;

/// Source: the shares of N₂ and O₂ in Earth's dry air by volume, 78.08 %
/// and 20.95 % (NASA Earth fact sheet, "78.08 % N₂, 20.95 % O₂, 9340 ppm
/// Ar").
pub(crate) const EARTH_N2_O2_SHARES: (f64, f64) = (0.7808, 0.2095);

/// Definition: Earth's measured air as a test input, without its CO₂ and
/// water (the tests set them) and without its argon, which the column has
/// no field for: [`EARTH_N2_O2_SHARES`] of [`EARTH_PRESSURE_BAR`].
/// [`EARTH_AIR`] is legacy's rounded 0.78 and 0.21 bar, not this air.
pub(crate) const EARTH_MEASURED_AIR: PartialPressures = PartialPressures {
    n2: EARTH_PRESSURE_BAR * EARTH_N2_O2_SHARES.0,
    co2: 0.0,
    h2o: 0.0,
    ch4: 0.0,
    nh3: 0.0,
    so2: 0.0,
    o2: EARTH_PRESSURE_BAR * EARTH_N2_O2_SHARES.1,
};
