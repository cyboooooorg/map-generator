//! Water cloud cover of an atmosphere, from its vapour and its humidity.
//!
//! Ported from `legacy-final` (`planet/albedo.rs`, `cloud_cover`), with its
//! test. The humidity reads the IAPWS saturation of [`crate::water`].

use crate::units::{Bar, Fraction, Kelvin};
use crate::water::vapour_saturation_bar;

/// Calibration: water vapour, bar, over which the cloud cover grows as
/// `1 - exp(-p_H2O / CLOUD_H2O_BAR)`. Fitted on Earth alone, whose
/// 1.26e-2 bar of vapour at 77 % relative humidity gives the cover of
/// 0.68 +- 0.03 (GEWEX assessment, Stubenrauch et al. 2013,
/// doi:10.1175/BAMS-D-12-00117.1; MODIS 0.67, King et al. 2013). What was
/// searched and not found: `docs/science/radiative-transfer.md`, "What we
/// do not know".
const CLOUD_H2O_BAR: f64 = 1.0e-2;

/// Source: Xu & Randall 1996, *J. Atmos. Sci.* 53, 3084,
/// doi:10.1175/1520-0469(1996)053<3084:ASCPFU>2.0.CO;2: the factor
/// `RH^0.25` of their semi-empirical cloudiness. Their other factor is
/// replaced by the vapour term of [`CLOUD_H2O_BAR`].
const CLOUD_HUMIDITY_EXPONENT: f64 = 0.25;

/// Share of the disc, 0 to 1, under water clouds in an air at `t_k` that
/// holds `h2o_bar` of vapour: the relative humidity (vapour over its
/// saturation at `t_k`, at most 1; a runaway's steam past the critical
/// point is saturated) to the power [`CLOUD_HUMIDITY_EXPONENT`], times
/// `1 - exp(-p_H2O / CLOUD_H2O_BAR)`. A negative or NaN vapour is none.
pub(crate) fn cloud_cover_share(t_k: f64, h2o_bar: f64) -> f64 {
    let h2o = h2o_bar.max(0.0);
    if h2o <= 0.0 || h2o.is_nan() {
        return 0.0;
    }
    let humidity = (h2o / vapour_saturation_bar(t_k)).min(1.0);
    humidity.powf(CLOUD_HUMIDITY_EXPONENT) * (1.0 - (-h2o / CLOUD_H2O_BAR).exp())
}

/// Share of the disc under water clouds at a surface at `surface` whose
/// air holds `h2o` of vapour. See [`cloud_cover_share`].
pub fn cloud_cover(surface: Kelvin, h2o: Bar) -> Fraction {
    Fraction::new(cloud_cover_share(surface.0, h2o.0).clamp(0.0, 1.0)).unwrap_or_else(|| {
        // Unreachable for finite inputs; a NaN surface reads as no cloud.
        Fraction::new(0.0).expect("zero is a fraction")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radiation::test_points::EARTH_SURFACE_K;
    use crate::surface::RELATIVE_HUMIDITY;

    /// Source: Stubenrauch et al. 2013 (GEWEX assessment), Earth's cloud
    /// cover 0.68 +- 0.03.
    const EARTH_COVER_RANGE: std::ops::RangeInclusive<f64> = 0.65..=0.71;

    #[test]
    fn a_dry_air_has_no_cloud() {
        assert_eq!(cloud_cover_share(300.0, 0.0), 0.0);
        assert_eq!(cloud_cover_share(300.0, -1e-12), 0.0);
        assert_eq!(cloud_cover(Kelvin(300.0), Bar(0.0)).get(), 0.0);
    }

    /// The calibration point of `CLOUD_H2O_BAR`, which was fitted on this
    /// cover: the test checks the fit holds, not an independent
    /// validation.
    #[test]
    fn earth_is_two_thirds_cloudy() {
        let vapour = RELATIVE_HUMIDITY * vapour_saturation_bar(EARTH_SURFACE_K);
        let cover = cloud_cover(Kelvin(EARTH_SURFACE_K), Bar(vapour)).get();
        println!("Earth's cloud cover at 288 K: {cover:.4}");
        assert!(EARTH_COVER_RANGE.contains(&cover), "{cover}");
    }

    #[test]
    fn cloud_cover_depends_on_more_than_vapour() {
        let vapour = RELATIVE_HUMIDITY * vapour_saturation_bar(EARTH_SURFACE_K);
        let cool = cloud_cover_share(EARTH_SURFACE_K, vapour);
        let warm = cloud_cover_share(320.0, vapour);
        assert!(warm < cool - 0.01, "{warm} vs {cool}");
        // A saturated air clouds more than the same air unsaturated, and
        // never past 1.
        let saturated = cloud_cover_share(EARTH_SURFACE_K, vapour_saturation_bar(EARTH_SURFACE_K));
        assert!(saturated > cool, "{saturated} vs {cool}");
        assert!(saturated <= 1.0);
    }

    #[test]
    fn steam_past_the_critical_point_stays_finite() {
        let cover = cloud_cover_share(1500.0, 100.0);
        assert!((0.0..=1.0).contains(&cover), "{cover}");
        let cold = cloud_cover_share(0.0, 1e-4);
        assert!((0.0..=1.0).contains(&cold), "{cold}");
    }
}
