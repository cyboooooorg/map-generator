//! The X-ray and extreme-ultraviolet (XUV) energy a planet has received
//! over the age of its system, which drives the escape of its envelope
//! and atmosphere (photoevaporation).
//!
//! A star emits a fixed share of its light as XUV while its rotation is
//! fast (saturation), then the share decays as a power of age. The
//! fluence is that emission integrated analytically from 0 to the age,
//! times the insolation. `activity.rs` keeps its own model for the star's
//! current activity; this one is for the history.
//!
//! The star's Lyman α line, which photolyses methane (see
//! [`super::atmosphere::methane_photolysis`]), follows the same history
//! with its own share and slope ([`LYMAN_ALPHA`]).
use super::Context;

/// One band of a star's high-energy light: its share of the bolometric
/// luminosity while the star is saturated, and the power of age by which
/// it decays after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Band {
    /// Share of the bolometric luminosity while saturated.
    pub saturated_ratio: f64,
    /// `L ∝ t^-decay_exponent` after saturation.
    pub decay_exponent: f64,
}

/// Share of the bolometric luminosity emitted as XUV (1 to 1180 Å) while
/// the star is saturated: the Sun's own, from Ribas et al. 2005
/// (doi:10.1086/427977), whose fit `F = 29.7 τ^-1.23` erg s⁻¹ cm⁻² at
/// 1 AU (τ in Gyr, from 0.1 Gyr; their Table 5, row 1-1180 Å) gives
/// 504 erg s⁻¹ cm⁻² at the end of saturation, 0.1 Gyr, or 3.71e-4 of the
/// 1361 W/m² solar constant. Checked against the paper; the value before
/// 0.1 Gyr is an extrapolation, Ribas et al. use no star that young. The
/// X-ray saturation of the fastest rotators, `L_X/L_bol ≈ 10^-3` (Wright
/// et al. 2011, doi:10.1088/0004-637X/743/1/48: mean `log R_X = -3.13`,
/// checked), is higher: the young Sun of Ribas et al. sat below it.
const SATURATED_RATIO: f64 = 3.71e-4;

/// Exponent of the decay of the XUV luminosity with age after saturation:
/// `L_XUV ∝ t^-1.23` for the Sun (Ribas et al. 2005,
/// doi:10.1086/427977, Table 5; their stars span 0.1 to 6.7 Gyr).
/// Checked.
const DECAY_EXPONENT: f64 = 1.23;

/// The XUV band, 1 to 1180 Å ([`SATURATED_RATIO`], [`DECAY_EXPONENT`]).
const XUV: Band = Band {
    saturated_ratio: SATURATED_RATIO,
    decay_exponent: DECAY_EXPONENT,
};

/// The H I Lyman α line (1216 Å). Ribas et al. 2005
/// (doi:10.1086/427977, their eq. 2, from two solar analogues, "preliminary")
/// fit `F = 19.2 τ^-0.72` erg s⁻¹ cm⁻² at 1 AU, τ in Gyr: 100.8 erg s⁻¹
/// cm⁻² at 0.1 Gyr, 7.40e-5 of the 1361 W/m² solar constant, and
/// 6.4 erg s⁻¹ cm⁻² (3.9e11 photons cm⁻² s⁻¹) at 4.6 Gyr. Checked; the
/// slope agrees with the τ^-0.65 ± 0.12 they derive from Wood et al.
/// 2004. Its saturation time is taken as the XUV's ([`saturation_gyr`]):
/// a simplification, and the line's larger share of an M dwarf's light is
/// not modelled.
pub(crate) const LYMAN_ALPHA: Band = Band {
    saturated_ratio: 7.40e-5,
    decay_exponent: 0.72,
};

/// Saturation time against the star's mass: `(M / M☉, t_sat / Gyr)`,
/// interpolated in log-log between the points and extrapolated along the
/// end segments ([`saturation_gyr`]). A calibration through published
/// anchors, not a published relation:
///
/// - 1 M☉, 0.1 Gyr: Jackson, Davis & Wheatley 2012 (MNRAS 422, 2024,
///   doi:10.1111/j.1365-2966.2012.20657.x) give `log τ_sat = 7.84`
///   (70 Myr) for the Sun's colour bin and a turn-off around 100 Myr for
///   FGK stars; Johnstone et al. 2021 (A&A 649, A96,
///   doi:10.1051/0004-6361/202038407) find most solar-mass stars
///   unsaturated by 100 Myr. 0.1 Gyr keeps Ribas et al.'s fit, which
///   starts there, and Earth's fluence at 1. Jackson et al. cover FGK
///   stars only.
/// - 0.5 M☉, 0.2 Gyr and 0.25 M☉, 2 Gyr: Johnstone et al. 2021, whose
///   slow and fast rotators stay on identical tracks (saturated) until
///   about 200 Myr at 0.5 M☉ and about 2 Gyr at 0.25 M☉ (read in their
///   text; their Fig. 10 has the full curve).
///
/// The slope is `M^-1` above 0.5 M☉ and `M^-3.3` below: 0.42 Gyr at
/// 0.4 M☉, 4.2 Gyr at 0.2 M☉ and 42 Gyr at 0.1 M☉, which stays saturated
/// all its life, as Johnstone et al. find the smallest stars still are
/// at 5 Gyr.
const SATURATION_GYR: [(f64, f64); 3] = [(0.25, 2.0), (0.5, 0.2), (1.0, 0.1)];

/// Saturation time of a star of `star_mass` (solar masses), Gyr: the
/// log-log interpolation of [`SATURATION_GYR`], extrapolated along its
/// first and last segments, the mass floored at 0.05 M☉.
fn saturation_gyr(star_mass: f64) -> f64 {
    let x = star_mass.max(0.05).ln();
    let i = if x < SATURATION_GYR[1].0.ln() { 0 } else { 1 };
    let (m0, t0) = SATURATION_GYR[i];
    let (m1, t1) = SATURATION_GYR[i + 1];
    let slope = (t1 / t0).ln() / (m1 / m0).ln();
    (t0.ln() + slope * (x - m0.ln())).exp()
}

/// Earth's age, Gyr: the reference of the relative fluence.
const EARTH_AGE_GYR: f64 = 4.567;

/// XUV energy Earth has received over 4.567 Gyr, J/m²: the fit of Ribas
/// et al. 2005 integrated, 0.504 W/m² for 0.1 Gyr then
/// `29.7 τ^-1.23` mW/m² to 4.567 Gyr, which is the model below for the
/// Sun, `3.71e-4 · 1361 W/m² · 0.354 Gyr` (5.64e15). Today's flux is
/// 4.6 mW/m².
pub const EARTH_XUV_FLUENCE_J_M2: f64 = 5.64e15;

/// Time-integral of the band-to-bolometric ratio from 0 to `age_gyr` for
/// a star of `star_mass`, in Gyr (constant in the saturated phase, then
/// the integral of the power-law decay, Ribas et al. 2005). The decay
/// exponent must not be 1.
pub(crate) fn integrated_ratio_gyr(band: Band, star_mass: f64, age_gyr: f64) -> f64 {
    let t_sat = saturation_gyr(star_mass);
    if age_gyr <= t_sat {
        return band.saturated_ratio * age_gyr.max(0.0);
    }
    let n = band.decay_exponent - 1.0;
    let decayed = t_sat * (1.0 - (age_gyr / t_sat).powf(-n)) / n;
    band.saturated_ratio * (t_sat + decayed)
}

/// Band-to-bolometric ratio of a star of `star_mass` at `age_gyr`.
#[cfg(test)]
pub(crate) fn ratio(band: Band, star_mass: f64, age_gyr: f64) -> f64 {
    let t_sat = saturation_gyr(star_mass);
    band.saturated_ratio * (age_gyr.max(t_sat) / t_sat).powf(-band.decay_exponent)
}

/// Age, Gyr, from which the band-to-bolometric ratio of a star of
/// `star_mass` is at most `ratio`: 0 if it never exceeds it (at or above
/// the saturated ratio), infinite if it never falls to it.
pub(crate) fn age_below_ratio(band: Band, star_mass: f64, ratio: f64) -> f64 {
    if ratio >= band.saturated_ratio {
        0.0
    } else if ratio <= 0.0 {
        f64::INFINITY
    } else {
        saturation_gyr(star_mass) * (ratio / band.saturated_ratio).powf(-1.0 / band.decay_exponent)
    }
}

/// XUV energy per unit area received over the system's age, relative to
/// Earth's (the Sun at 1 AU at 4.567 Gyr is 1.0). The luminosity enters
/// through the insolation, which already includes it and the orbit's
/// eccentricity; the star's luminosity is taken constant over its age
/// (the pre-main-sequence excess, short, is ignored).
pub fn xuv_fluence(context: &Context) -> f64 {
    let earth = integrated_ratio_gyr(XUV, 1.0, EARTH_AGE_GYR);
    context.insolation * integrated_ratio_gyr(XUV, context.star_mass, context.age_gyr) / earth
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Solar constant, W/m² (Kopp & Lean 2011, doi:10.1029/2010GL045777).
    const SOLAR_CONSTANT_W_M2: f64 = 1361.0;

    /// Seconds in a gigayear.
    const GYR_S: f64 = 3.15576e16;

    use crate::star::{StarInfo, parse_type};

    fn sun(orbit_au: f64, age_gyr: f64) -> Context {
        let star = StarInfo::new(
            parse_type("G2V").unwrap(),
            orbit_au as f32,
            0.0,
            age_gyr as f32,
        );
        Context::of(&star, 0.0, 2.7)
    }

    #[test]
    fn earth_receives_one() {
        // The table's G2V has L = 1.023 L☉, so Earth's own insolation (1.0)
        // is set explicitly.
        let mut c = sun(1.0, 4.567);
        c.insolation = 1.0;
        c.star_mass = 1.0;
        let f = xuv_fluence(&c);
        assert!((0.99..=1.01).contains(&f), "{f}");
    }

    #[test]
    fn earth_absolute_value_matches_the_model() {
        let j = SOLAR_CONSTANT_W_M2 * integrated_ratio_gyr(XUV, 1.0, EARTH_AGE_GYR) * GYR_S;
        assert!((j / EARTH_XUV_FLUENCE_J_M2 - 1.0).abs() < 0.02, "{j:e}");
    }

    #[test]
    fn earth_matches_ribas_2005() {
        // Ribas et al. 2005: F = 29.7 τ^-1.23 erg/s/cm² = mW/m² at 1 AU,
        // saturated before 0.1 Gyr.
        let ribas = |t: f64| 29.7e-3 * t.max(0.1).powf(-DECAY_EXPONENT);
        let saturated = ribas(0.1) * 0.1;
        let decayed =
            29.7e-3 * (0.1f64.powf(-0.23) - EARTH_AGE_GYR.powf(-0.23)) / (DECAY_EXPONENT - 1.0);
        let j = (saturated + decayed) * GYR_S;
        assert!((j / EARTH_XUV_FLUENCE_J_M2 - 1.0).abs() < 0.01, "{j:e}");
        // Today's flux: the model's ratio at 4.567 Gyr times the constant.
        let now = SOLAR_CONSTANT_W_M2
            * SATURATED_RATIO
            * (EARTH_AGE_GYR / saturation_gyr(1.0)).powf(-DECAY_EXPONENT);
        assert!((now / ribas(EARTH_AGE_GYR) - 1.0).abs() < 0.01, "{now}");
    }

    #[test]
    fn lyman_alpha_matches_ribas_2005() {
        // Ribas et al. 2005, eq. 2: F = 19.2 τ^-0.72 erg/s/cm² = mW/m² at
        // 1 AU, from 0.1 Gyr.
        for age in [0.1, 1.0, 4.6, 8.0] {
            let model = SOLAR_CONSTANT_W_M2 * ratio(LYMAN_ALPHA, 1.0, age);
            let ribas = 19.2e-3 * f64::powf(age, -0.72);
            assert!((model / ribas - 1.0).abs() < 0.01, "{age}: {model} {ribas}");
        }
        // About 3.9e11 photons cm⁻² s⁻¹ today.
        let photons = SOLAR_CONSTANT_W_M2 * ratio(LYMAN_ALPHA, 1.0, 4.6) / 1.634e-18 * 1e-4;
        assert!((3.7e11..4.1e11).contains(&photons), "{photons:e}");
    }

    #[test]
    fn the_band_history_integrates_and_inverts() {
        for mass in [1.0, 0.5, 0.2] {
            for band in [XUV, LYMAN_ALPHA] {
                // The analytic integral against a midpoint sum.
                let age = 6.0;
                let steps = 200_000;
                let dt = age / f64::from(steps);
                let sum: f64 = (0..steps)
                    .map(|i| ratio(band, mass, (f64::from(i) + 0.5) * dt) * dt)
                    .sum();
                let exact = integrated_ratio_gyr(band, mass, age);
                assert!((sum / exact - 1.0).abs() < 1e-3, "{mass}: {sum} {exact}");
                // The age at which the ratio falls to its value at 3 Gyr is
                // 3 Gyr, or 0 for a star still saturated then.
                let t = age_below_ratio(band, mass, ratio(band, mass, 3.0));
                if saturation_gyr(mass) < 3.0 {
                    assert!((t / 3.0 - 1.0).abs() < 1e-9, "{mass}: {t}");
                } else {
                    assert_eq!(t, 0.0, "{mass}");
                }
            }
        }
        assert_eq!(age_below_ratio(LYMAN_ALPHA, 1.0, 1.0), 0.0);
        assert!(age_below_ratio(LYMAN_ALPHA, 1.0, 0.0).is_infinite());
    }

    #[test]
    fn fluence_falls_as_the_square_of_distance() {
        let r = xuv_fluence(&sun(2.0, 4.567)) / xuv_fluence(&sun(1.0, 4.567));
        assert!((r - 0.25).abs() < 0.0025, "{r}");
    }

    #[test]
    fn older_systems_received_more_but_not_much_more() {
        let r = xuv_fluence(&sun(1.0, 10.0)) / xuv_fluence(&sun(1.0, 5.0));
        assert!(r > 1.0 && r < 1.3, "{r}");
    }

    #[test]
    fn m_dwarfs_saturate_longer() {
        let mut c = sun(1.0, 4.567);
        c.star_mass = 0.2;
        c.insolation = 1.0;
        let f = xuv_fluence(&c);
        assert!(f > 10.0, "{f}");
    }

    #[test]
    fn the_saturation_time_goes_through_its_anchors() {
        for (mass, gyr) in SATURATION_GYR {
            let t = saturation_gyr(mass);
            assert!((t / gyr - 1.0).abs() < 1e-12, "{mass}: {t}");
        }
        // Longer as the mass falls, and finite down to the floor.
        let masses = [2.0, 1.0, 0.7, 0.5, 0.3, 0.25, 0.1, 0.05, 0.01];
        for pair in masses.windows(2) {
            assert!(
                saturation_gyr(pair[1]) >= saturation_gyr(pair[0]),
                "{pair:?}"
            );
        }
        assert!(saturation_gyr(0.01).is_finite());
        assert!(saturation_gyr(0.1) > 13.0);
    }

    #[test]
    fn the_fluence_is_finite_at_every_age() {
        for code in 0..=90_u8 {
            let code = f32::from(code);
            for age in [0.01_f32, 13.0] {
                let star = StarInfo::new(code, 1.0, 0.0, age);
                let f = xuv_fluence(&Context::of(&star, 0.0, 2.7));
                assert!(f.is_finite() && f > 0.0, "code {code} age {age}: {f}");
            }
        }
    }
}
