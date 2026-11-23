//! The runaway greenhouse limit of a star's light, and the unit of an
//! ocean.
//!
//! **Runaway greenhouse** (Kopparapu et al. 2014, ApJL 787, L29,
//! doi:10.1088/2041-8205/787/2/L29, Table 1, first-column fit for a
//! 1 M_earth planet): the flux at which a planet cannot keep liquid water
//! is `S = 1.107 + a T + b T² + c T³ + d T⁴`, `T = Teff - 5780 K`. The
//! paper's mass correction is not applied. See
//! `docs/science/habitable-zone.md`.
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

#[cfg(test)]
#[path = "habitable_tests.rs"]
mod tests;
