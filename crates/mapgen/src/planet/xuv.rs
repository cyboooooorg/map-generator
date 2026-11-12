//! The X-ray and extreme-ultraviolet (XUV) energy a planet has received
//! over the age of its system, which drives the escape of its envelope
//! and atmosphere (photoevaporation).
//!
//! A star emits a fixed share of its light as XUV while its rotation is
//! fast (saturation), then the share decays as a power of age. The
//! fluence is that emission integrated analytically from 0 to the age,
//! times the insolation. `activity.rs` keeps its own model for the star's
//! current activity; this one is for the history.
use super::Context;

/// Share of the bolometric luminosity emitted as XUV (1 to 1180 Å) while
/// the star is saturated: the Sun's own, from Ribas et al. 2005
/// (doi:10.1086/427977), whose fit `F = 29.7 τ^-1.23` erg s⁻¹ cm⁻² at
/// 1 AU (τ in Gyr, from 0.1 Gyr) gives 504 erg s⁻¹ cm⁻² at the end of
/// saturation, 0.1 Gyr, or 3.71e-4 of the 1361 W/m² solar constant. The
/// fit and its band are taken from the paper as cited by the review of
/// this model, not re-read in it. The X-ray saturation of the fastest
/// rotators, `L_X/L_bol ≈ 10^-3` (Wright et al. 2011,
/// doi:10.1088/0004-637X/743/1/48), is higher: the young Sun of Ribas
/// et al. sat below it.
const SATURATED_RATIO: f64 = 3.71e-4;

/// Exponent of the decay of the XUV luminosity with age after saturation:
/// `L_XUV ∝ t^-1.23` for the Sun (Ribas et al. 2005,
/// doi:10.1086/427977, 0.1 to 6.7 Gyr).
const DECAY_EXPONENT: f64 = 1.23;

/// Saturation time of a solar-mass star, Gyr: about 100 Myr (Jackson,
/// Davis & Wheatley 2012, doi:10.1111/j.1365-2966.2012.21384.x;
/// Johnstone et al. 2021, doi:10.1051/0004-6361/202038407).
const SATURATION_SUN_GYR: f64 = 0.1;

/// Growth of the saturation time as the star's mass falls:
/// `t_sat = 0.1 Gyr · M^-2`, M in solar masses. Slower rotation braking
/// in low-mass stars keeps them saturated for around a Gyr at 0.3 M☉ and
/// longer below (Jackson et al. 2012; Johnstone et al. 2021). Calibration,
/// no source, for the exponent: it gives 1.1 Gyr at 0.3 M☉ and 2.5 Gyr at
/// 0.2 M☉.
const SATURATION_MASS_EXPONENT: f64 = 2.0;

/// Earth's age, Gyr: the reference of the relative fluence.
const EARTH_AGE_GYR: f64 = 4.567;

/// XUV energy Earth has received over 4.567 Gyr, J/m²: the fit of Ribas
/// et al. 2005 integrated, 0.504 W/m² for 0.1 Gyr then
/// `29.7 τ^-1.23` mW/m² to 4.567 Gyr, which is the model below for the
/// Sun, `3.71e-4 · 1361 W/m² · 0.354 Gyr` (5.64e15). Today's flux is
/// 4.6 mW/m².
pub const EARTH_XUV_FLUENCE_J_M2: f64 = 5.64e15;

/// Time-integral of the XUV-to-bolometric ratio from 0 to `age_gyr` for a
/// star of `star_mass`, in Gyr (constant in the saturated phase, then
/// the integral of the power-law decay, Ribas et al. 2005).
fn integrated_ratio_gyr(star_mass: f64, age_gyr: f64) -> f64 {
    let t_sat = SATURATION_SUN_GYR * star_mass.max(0.05).powf(-SATURATION_MASS_EXPONENT);
    if age_gyr <= t_sat {
        return SATURATED_RATIO * age_gyr.max(0.0);
    }
    let n = DECAY_EXPONENT - 1.0;
    let decayed = t_sat * (1.0 - (age_gyr / t_sat).powf(-n)) / n;
    SATURATED_RATIO * (t_sat + decayed)
}

/// XUV energy per unit area received over the system's age, relative to
/// Earth's (the Sun at 1 AU at 4.567 Gyr is 1.0). The luminosity enters
/// through the insolation, which already includes it and the orbit's
/// eccentricity; the star's luminosity is taken constant over its age
/// (the pre-main-sequence excess, short, is ignored).
pub fn xuv_fluence(context: &Context) -> f64 {
    let earth = integrated_ratio_gyr(1.0, EARTH_AGE_GYR);
    context.insolation * integrated_ratio_gyr(context.star_mass, context.age_gyr) / earth
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
        let j = SOLAR_CONSTANT_W_M2 * integrated_ratio_gyr(1.0, EARTH_AGE_GYR) * GYR_S;
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
            * (EARTH_AGE_GYR / SATURATION_SUN_GYR).powf(-DECAY_EXPONENT);
        assert!((now / ribas(EARTH_AGE_GYR) - 1.0).abs() < 0.01, "{now}");
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
