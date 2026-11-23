//! The runaway greenhouse limit of a star's light, the unit of an ocean,
//! and the share of a rocky planet's bulk water that is at its surface.
//!
//! **Runaway greenhouse** (Kopparapu et al. 2014, ApJL 787, L29,
//! doi:10.1088/2041-8205/787/2/L29, Table 1, first-column fit for a
//! 1 M_earth planet): the flux at which a planet cannot keep liquid water
//! is `S = 1.107 + a T + b T² + c T³ + d T⁴`, `T = Teff - 5780 K`. The
//! paper's mass correction is not applied. See
//! `docs/science/habitable-zone.md`.
//!
//! **Surface water** ([`SURFACE_WATER_SHARE`]): a planet whose water is
//! bound in its mantle keeps that share of it at its surface, Earth's
//! ocean over Earth's bulk water, the same for every planet (study
//! [22](https://github.com/cyboooooorg/map-generator/wiki/22-surface-water);
//! see `docs/science/planet-atmospheres.md`, "Surface water").
//!
//! Ported from the tag `legacy-final` (`star.rs` `habitable_fluxes`,
//! `planet/regime.rs`), with the sources checked in study
//! [21](https://github.com/cyboooooorg/map-generator/wiki/21-escape-outgassing-audit).

use crate::units::{EarthFlux, Kelvin, Kilogram};

/// Source: the solar-type runaway greenhouse flux, in units of Earth's,
/// Kopparapu et al. 2014, Table 1, `S_eff,sun` of the runaway greenhouse.
const SOLAR_FLUX: f64 = 1.107;

/// Source: the temperature of the fit's origin, K, Kopparapu et al. 2014,
/// eq. 2 (`T_star = Teff - 5780 K`).
const FIT_ORIGIN: f64 = 5_780.0;

/// Source: coefficient `a` of the runaway greenhouse fit, Kopparapu et
/// al. 2014, Table 1.
const COEFFICIENT_A: f64 = 1.332e-4;

/// Source: coefficient `b` of the runaway greenhouse fit, Kopparapu et
/// al. 2014, Table 1.
const COEFFICIENT_B: f64 = 1.58e-8;

/// Source: coefficient `c` of the runaway greenhouse fit, Kopparapu et
/// al. 2014, Table 1.
const COEFFICIENT_C: f64 = -8.308e-12;

/// Source: coefficient `d` of the runaway greenhouse fit, Kopparapu et
/// al. 2014, Table 1.
const COEFFICIENT_D: f64 = -1.931e-15;

/// Source: coolest effective temperature of the fit, K, Kopparapu et al.
/// 2014 ("2600 K ≤ Teff ≤ 7200 K").
const FIT_MIN: f64 = 2_600.0;

/// Source: hottest effective temperature of the fit, K, Kopparapu et al.
/// 2014.
const FIT_MAX: f64 = 7_200.0;

/// The flux, in units of Earth's, above which a planet lit by one star of
/// this effective temperature runs away: its oceans sit in the air as
/// vapour. The temperature is clamped to the fit's range.
///
/// The limit is per source. Under several stars a planet runs away when
/// `Σ flux_i / runaway_flux(Teff_i) > 1`, each star with its own
/// temperature (issue #592); that sum is the system's, not this law's.
#[must_use]
pub fn runaway_flux(effective_temperature: Kelvin) -> EarthFlux {
    let t = effective_temperature.0.clamp(FIT_MIN, FIT_MAX) - FIT_ORIGIN;
    EarthFlux(
        SOLAR_FLUX
            + COEFFICIENT_A * t
            + COEFFICIENT_B * t * t
            + COEFFICIENT_C * t.powi(3)
            + COEFFICIENT_D * t.powi(4),
    )
}

/// Definition: the unit of an ocean, the volume of Earth's ocean
/// (1.332e9 km³, Charette & Smith 2010, Oceanography 23(2), 112,
/// doi:10.5670/oceanog.2010.51, Table 1) rounded to 1.4e21 kg. At the
/// usual 1027 kg/m³ (a density not read here) that volume is 1.37e21 kg.
pub const OCEAN_MASS: Kilogram = Kilogram(1.4e21);

/// Source: Earth's ocean as a share of Earth's mass, `ω_o` = 2.3e-4
/// (Cowan & Abbot 2014, ApJ 781, 27, doi:10.1088/0004-637X/781/1/27,
/// Table 1): 1.37e21 kg over 5.97e24 kg, the ocean alone, without the
/// 0.19 ocean held in the continental crust (Korenaga, Planavsky & Evans
/// 2017, Phil. Trans. R. Soc. A 375, 20150393, §2a).
const EARTH_OCEAN_FRACTION: f64 = 2.3e-4;

/// Source: Earth's bulk water as a share of its mass, 2700 ppm (Marty
/// 2012, Earth Planet. Sci. Lett. 313-314, 56,
/// doi:10.1016/j.epsl.2011.10.040, p. 18: 2700 ± 1350 ppm, "10 ± 5 ocean
/// masses"), the median the engine draws a dry planet's water around.
const EARTH_BULK_WATER_FRACTION: f64 = 2.7e-3;

/// Calibration: share of a rocky planet's bulk water that is at its
/// surface (its ocean), the rest bound in its mantle: Earth's ocean over
/// its bulk water, [`EARTH_OCEAN_FRACTION`] / [`EARTH_BULK_WATER_FRACTION`]
/// = 2.3e-4 / 2.7e-3 = 0.085, so that a planet drawn with Earth's bulk
/// water gets Earth's ocean. The same share for every mass and regime:
/// the calibration decided on #663, option A of study
/// [22](https://github.com/cyboooooorg/map-generator/wiki/22-surface-water).
/// Searched and not found (study
/// [22](https://github.com/cyboooooorg/map-generator/wiki/22-surface-water)):
/// a measured or modelled share for any planet other than Earth with
/// plates, and a share for heat-pipe or episodic planets. It rides on
/// Marty's large bulk: other budgets give Earth 0.07 to about 0.6. A
/// stagnant lid's lower share (about 0.02 to 0.05, Tosi et al. 2017) and
/// the water cycle over time are #665.
pub const SURFACE_WATER_SHARE: f64 = EARTH_OCEAN_FRACTION / EARTH_BULK_WATER_FRACTION;

#[cfg(test)]
#[path = "habitable_tests.rs"]
mod tests;
