//! The light of the star, by wavelength: the bands the shortwave table is
//! built on, each with its Rayleigh scale and its share of the light
//! (ported from `legacy-final`'s `planet/albedo.rs`, `spectrum` and
//! `band_edge_m`).
//!
//! A redder star's light scatters less, Rayleigh's `lambda^-4` weighing the
//! blue, and more of it falls in the near infrared, where CO2 and water
//! absorb it: the two reasons the outer edge of an M dwarf's habitable
//! zone lies at a lower flux (Kopparapu et al. 2013, section 4.1,
//! doi:10.1088/0004-637X/765/2/131, read on arXiv:1301.6674v2).

use crate::constants::{SECOND_RADIATION_CONSTANT_CM_K, SOLAR_EFFECTIVE_TEMPERATURE};

/// Definition: number of bands of the star's light, a numerical choice
/// (bands even in `ln lambda`), see the radiative reference page in
/// `docs/science/`, shortwave section.
pub(crate) const BANDS: usize = 240;

/// Definition: shortest wavelength of the bands, m, a numerical choice of
/// the band model's range.
pub(crate) const FROM_M: f64 = 0.2e-6;

/// Definition: longest wavelength of the bands, m, a numerical choice of
/// the band model's range.
pub(crate) const TO_M: f64 = 5.0e-6;

/// Definition: wavelength at which the Rayleigh scale is 1, m (550 nm, the
/// reference of the Rayleigh cross sections).
const REFERENCE_M: f64 = 0.55e-6;

/// Definition: metres per centimetre, a unit conversion.
const M_PER_CM: f64 = 1e-2;

/// Wavelength, m, of the `e`-th edge of the bands (0 to [`BANDS`]), from
/// 0.2 um: even in `ln lambda`. The shortwave table is built on the same
/// edges (`tools/opacity-table --shortwave`) and checked against them.
pub(crate) fn band_edge_m(e: usize) -> f64 {
    let step = (TO_M / FROM_M).ln() / BANDS as f64;
    FROM_M * (step * e as f64).exp()
}

/// The light of a star: its effective temperature and its bands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StarBands {
    /// Effective temperature, K, the bands were computed for.
    pub(crate) star_k: f64,
    /// Each band's `(550 nm / lambda)^4` and its share of the light (they
    /// sum to 1), bluest first.
    pub(crate) bands: Vec<(f64, f64)>,
}

/// The light of a black body at `star_k` (a star of that effective
/// temperature), by wavelength. A temperature that is not positive (or is
/// NaN) is the Sun's.
pub(crate) fn star_bands(star_k: f64) -> StarBands {
    let star_k = if star_k > 0.0 {
        star_k
    } else {
        SOLAR_EFFECTIVE_TEMPERATURE.0
    };
    // h c / k, m.K.
    let c2 = SECOND_RADIATION_CONSTANT_CM_K * M_PER_CM;
    let mut bands: Vec<(f64, f64)> = (0..BANDS)
        .map(|i| {
            let lambda = (band_edge_m(i) * band_edge_m(i + 1)).sqrt();
            // B_lambda d(lambda) with d(lambda) = lambda d(ln lambda).
            let planck = lambda.powi(-4) / ((c2 / (lambda * star_k)).exp() - 1.0);
            ((REFERENCE_M / lambda).powi(4), planck)
        })
        .collect();
    let total: f64 = bands.iter().map(|b| b.1).sum();
    for band in &mut bands {
        band.1 /= total;
    }
    StarBands { star_k, bands }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source: IAU 2015 Resolution B3, nominal solar effective temperature
    /// (the value of `SOLAR_EFFECTIVE_TEMPERATURE`).
    const SUN_K: f64 = 5772.0;

    /// Definition: test temperature of an M dwarf, K.
    const M_DWARF_K: f64 = 3300.0;

    /// Definition: test temperature of a cool brown-dwarf-like star, K.
    const COOL_K: f64 = 2600.0;

    /// Definition: test temperature of a hot star, K.
    const HOT_K: f64 = 50_000.0;

    /// Definition: the 4 of `x = 4 (1 - e^-x)`, the condition `d(lambda
    /// B_lambda)/d(lambda) = 0` (the exponent of `lambda` in `lambda
    /// B_lambda` is -4, the 5 of Wien's `B_lambda` less the 1 of `d ln
    /// lambda`).
    const WIEN_LAMBDA_B_LEVEL: f64 = 4.0;

    /// Definition: wavelength past which light counts as infrared here, m.
    const INFRARED_M: f64 = 0.7e-6;

    #[test]
    fn the_bands_carry_all_the_light() {
        for k in [COOL_K, SUN_K, HOT_K] {
            let s = star_bands(k);
            assert_eq!(s.bands.len(), BANDS);
            let sum: f64 = s.bands.iter().map(|b| b.1).sum();
            assert!((sum - 1.0).abs() < 1e-12, "{k} K: {sum}");
            assert!(
                s.bands
                    .iter()
                    .all(|b| b.0.is_finite() && b.1.is_finite() && b.0 >= 0.0 && b.1 >= 0.0)
            );
        }
    }

    #[test]
    fn a_cold_or_unknown_star_is_the_sun() {
        let sun = star_bands(SUN_K);
        assert_eq!(sun.star_k, SUN_K);
        for k in [0.0, -1.0, f64::NAN] {
            assert_eq!(star_bands(k), sun, "{k}");
        }
    }

    #[test]
    fn a_red_star_puts_its_light_in_the_infrared() {
        let share = |k: f64| -> f64 {
            star_bands(k)
                .bands
                .iter()
                .enumerate()
                .filter(|(i, _)| (band_edge_m(*i) * band_edge_m(i + 1)).sqrt() > INFRARED_M)
                .map(|(_, b)| b.1)
                .sum()
        };
        assert!(share(M_DWARF_K) > share(SUN_K));
    }

    #[test]
    fn the_bands_peak_where_planck_does() {
        // The bands are even in ln(lambda), so a band's weight follows
        // lambda B_lambda, whose maximum is at lambda T = c2 / x, with x
        // the root of x = 4 (1 - e^-x) (3.9207). Not Wien's b, which is the
        // maximum of B_lambda.
        let mut x: f64 = WIEN_LAMBDA_B_LEVEL;
        for _ in 0..200 {
            x = WIEN_LAMBDA_B_LEVEL * (1.0 - (-x).exp());
        }
        let peak_m = SECOND_RADIATION_CONSTANT_CM_K * M_PER_CM / (x * SUN_K);
        let s = star_bands(SUN_K);
        let (best, _) = s
            .bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.1.total_cmp(&b.1.1))
            .unwrap();
        assert!(
            band_edge_m(best) <= peak_m && peak_m <= band_edge_m(best + 1),
            "{best}: {peak_m}"
        );
    }
}
