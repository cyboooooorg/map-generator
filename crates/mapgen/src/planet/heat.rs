//! The heat a planet makes inside (spec "Derived, in order", item 2): the
//! decay of its radioactive isotopes, from its silicate mass, its
//! abundance at formation and its age ([`radiogenic_w`]), and the tides a
//! host raises in it on an eccentric orbit ([`tidal_power_w`]).
//! [`InternalHeat`] holds both and the mean flux they push through the
//! surface.
use serde::{Deserialize, Serialize};

use crate::exomoon::{EARTH_M, G};
use std::f64::consts::PI;

/// Mass of Earth's bulk silicate part (mantle and crust), kg: Earth's
/// 5.972e24 less its core's 1.94e24 (PREM, Dziewonski & Anderson 1981,
/// doi:10.1016/0031-9201(81)90046-7).
pub const EARTH_SILICATE_KG: f64 = 4.03e24;

/// Earth's present radiogenic heat, W: 20 TW from the bulk silicate Earth
/// (Arevalo, McDonough & Luong 2009, doi:10.1016/j.epsl.2008.12.023;
/// Korenaga 2008, doi:10.1029/2007RG000241, gives 16 to 20).
const EARTH_RADIOGENIC_W: f64 = 20e12;

/// Earth's age, Gyr, at which [`EARTH_RADIOGENIC_W`] holds.
const EARTH_AGE_GYR: f64 = 4.567;

/// Each heat-producing isotope: its share of Earth's present radiogenic
/// heat and its half-life, Gyr. The shares follow from the silicate
/// Earth's 20 ppb U and 80 ppb Th (McDonough & Sun 1995,
/// doi:10.1016/0009-2541(94)00140-4: 20.3 and 79.5), its K/U of 13 800, that is 280 ppm K
/// (Arevalo et al. 2009), and the heat each isotope makes per kilogram
/// (Turcotte & Schubert 2014, table 4.2: U-238 9.46e-5 W/kg, U-235
/// 5.69e-4, Th-232 2.64e-5, K-40 2.92e-5); the half-lives are from the
/// same table. Together they give 20.3 TW, rounded to
/// [`EARTH_RADIOGENIC_W`].
const ISOTOPES: [(f64, f64); 4] = [
    // U-238
    (0.372, 4.468),
    // U-235
    (0.016, 0.704),
    // Th-232
    (0.419, 14.0),
    // K-40
    (0.193, 1.248),
];

/// Radiogenic heat, W, of a body with `silicate_kg` of silicates, formed
/// with `abundance` times Earth's initial inventory of U, Th and K per
/// kilogram, `age_gyr` after its formation: each isotope's share of
/// Earth's present 20 TW, scaled by mass and abundance and carried from
/// Earth's age to `age_gyr` by its decay, `Σ sᵢ H⊕ (M/M⊕) a exp(λᵢ (4.567
/// − t))`, `λᵢ = ln 2 / t½ᵢ`. Earth's heat was about 5 times today's when
/// it formed.
pub fn radiogenic_w(silicate_kg: f64, abundance: f64, age_gyr: f64) -> f64 {
    let scale = EARTH_RADIOGENIC_W * (silicate_kg / EARTH_SILICATE_KG) * abundance;
    ISOTOPES
        .iter()
        .map(|&(share, half_life)| {
            share * scale * (std::f64::consts::LN_2 / half_life * (EARTH_AGE_GYR - age_gyr)).exp()
        })
        .sum()
}

/// Tidal heating, W, of a satellite of radius `radius_m` on an orbit of
/// semi-major axis `a_m` and eccentricity `eccentricity` around a host of
/// `host_kg`, with a tidal response `k2_over_q` (`k₂/Q`): Peale, Cassen &
/// Reynolds 1979 (doi:10.1126/science.203.4383.892), `Ė = (21/2)(k₂/Q) G
/// M_host² R⁵ n e² / a⁶`, `n = √(G M_host / a³)` the mean motion (the
/// satellite's own mass neglected). Io (Jupiter, `k₂/Q` 0.015) gets about
/// 1e14 W, as observed (Lainey et al. 2009, doi:10.1038/nature08108).
pub fn tidal_power_w(
    host_kg: f64,
    radius_m: f64,
    a_m: f64,
    eccentricity: f64,
    k2_over_q: f64,
) -> f64 {
    // The order of the products is `exomoon.rs`'s, so that its worlds keep
    // their bits.
    let n = (G * host_kg / a_m.powi(3)).sqrt();
    10.5 * k2_over_q * G * host_kg * host_kg * radius_m.powi(5) * n * eccentricity * eccentricity
        / a_m.powi(6)
}

/// The heat a planet makes inside, W.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct InternalHeat {
    /// From the decay of U, Th and K ([`radiogenic_w`]).
    pub radiogenic_w: f64,
    /// From a host's tides ([`tidal_power_w`]); 0 without one.
    pub tidal_w: f64,
}

impl InternalHeat {
    /// Total internal heat, W.
    pub fn total_w(&self) -> f64 {
        self.radiogenic_w + self.tidal_w
    }

    /// Mean flux through the surface, W/m², for a radius in Earth radii.
    pub fn flux_w_m2(&self, radius_earth: f64) -> f64 {
        let r = radius_earth * EARTH_M;
        self.total_w() / (4.0 * PI * r * r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_makes_twenty_terawatts_today() {
        let w = radiogenic_w(EARTH_SILICATE_KG, 1.0, 4.567);
        assert!((w / 20e12 - 1.0).abs() < 0.01, "{w}");
    }

    #[test]
    fn a_young_earth_was_four_to_six_times_hotter() {
        let ratio =
            radiogenic_w(EARTH_SILICATE_KG, 1.0, 0.0) / radiogenic_w(EARTH_SILICATE_KG, 1.0, 4.567);
        assert!((4.5..6.0).contains(&ratio), "{ratio}");
    }

    #[test]
    fn io_tidal_heat_is_about_a_hundred_terawatts() {
        // Io: Jupiter 1.898e27 kg, R 1.8216e6 m, a 4.217e8 m, e 0.0041,
        // k₂/Q 0.015 (exomoon's): observed output about 1e14 W (Lainey et al. 2009).
        let w = tidal_power_w(1.898e27, 1.8216e6, 4.217e8, 0.0041, 0.015);
        assert!((3e13..3e14).contains(&w), "{w}");
    }

    #[test]
    fn heat_adds_up_and_spreads_over_the_surface() {
        let heat = InternalHeat {
            radiogenic_w: 20e12,
            tidal_w: 1e12,
        };
        assert_eq!(heat.total_w(), 21e12);
        // 21 TW over Earth's 5.10e14 m²: 0.041 W/m².
        let flux = heat.flux_w_m2(1.0);
        assert!((flux / 0.0412 - 1.0).abs() < 0.01, "{flux}");
        assert!((heat.flux_w_m2(2.0) / flux - 0.25).abs() < 1e-12);
    }
}
