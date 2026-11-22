//! The bodies and the Earth case the radiation tests share, each defined
//! once.

use crate::constants::GRAVITATIONAL_CONSTANT;
use crate::units::MetrePerSecondSquared;

/// Source: Venus's surface gravity, 8.87 m/s² (NASA Venus Fact Sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html, "Surface
/// gravity (mean)", read 2026-11-22).
pub(super) const VENUS_GRAVITY: MetrePerSecondSquared = MetrePerSecondSquared(8.87);

/// Source: Titan's surface gravity, `G M / R²` from its mass, 1345.5e20
/// kg, and mean radius, 2575 km (NASA Saturnian Satellite Fact Sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/saturniansatfact.html, read
/// 2026-11-22): 1.354 m/s² (the legacy took 1.352).
pub(super) const TITAN_GRAVITY: MetrePerSecondSquared =
    MetrePerSecondSquared(GRAVITATIONAL_CONSTANT * 1345.5e20 / (2575e3 * 2575e3));

/// Source: the surface relative humidity of Manabe & Wetherald 1967
/// (J. Atmos. Sci. 24, 241, doi:10.1175/1520-0469(1967)024<0241:TEOTAW>2.0.CO;2,
/// eq. 2), 77 %, the legacy `RELATIVE_HUMIDITY` of a moist surface. The
/// paper is not open; the value is confirmed indirectly by climlab's
/// `ManabeWaterVapor` default (`relative_humidity=0.77`, "following
/// Manabe 1967") and by arXiv:2006.03098 ("surface 77 %"), as the audit
/// (Wiki study 19) records.
pub(super) const MANABE_HUMIDITY: f64 = 0.77;

/// Definition: Earth's surface temperature of the test cases, K, its mean
/// surface temperature rounded (the U.S. Standard Atmosphere 1976 takes
/// 288.15 K at sea level).
pub(super) const EARTH_SURFACE_K: f64 = 288.0;
