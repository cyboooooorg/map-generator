//! The heat a planet makes inside (spec "Derived, in order", item 2): the
//! decay of its radioactive isotopes, from its silicate mass, its
//! abundance at formation and its age ([`radiogenic_w`]), and the tides a
//! host raises in it on an eccentric orbit ([`tidal_power_w`]), with the
//! response of a rocky, icy or enveloped body ([`k2_over_q`]).
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

/// Tidal response `k₂/Q` of a rocky body: Io's, `0.015 ± 0.003`, measured
/// from the drift of the Galilean moons' orbits (Lainey et al. 2009,
/// doi:10.1038/nature08108; table 1 of Lainey 2016,
/// doi:10.1007/s10569-016-9695-y), the value `crate::exomoon` gives its
/// moons. Io is partly molten: a cold rocky body dissipates less (Mars,
/// `k₂` 0.173 over `Q` 99.5, about 1.7e-3, same table).
pub const ROCKY_K2_OVER_Q: f64 = 0.015;

/// Tidal response `k₂/Q` of an icy body, an ice shell over an ocean.
/// Calibration, no source: no icy moon has a measured `k₂/Q`. A `k₂` of
/// about 0.25 for a shell over an ocean (Wahr et al. 2006,
/// doi:10.1029/2006JE002729) over a `Q` of 100 gives Europa about
/// 1.1e12 W, 35 mW/m², within the 10 to 50 mW/m² of tidal heating that
/// thermal models of its shell consider (Kimura 2024, arXiv:2403.01336),
/// and Enceladus about 3.7 GW, a quarter of the 15.8 ± 3.1 GW it radiates
/// (Howett et al. 2011, doi:10.1029/2010JE003718) and that the
/// equilibrium of its orbit can supply given Saturn's strong measured
/// dissipation (Lainey et al. 2012, doi:10.1088/0004-637X/752/1/14).
pub const ICY_K2_OVER_Q: f64 = 2.5e-3;

/// Tidal response `k₂/Q` of a body wrapped in hydrogen and helium:
/// Jupiter's, `(1.102 ± 0.203) × 10⁻⁵`, measured with Io's (Lainey et al.
/// 2009, doi:10.1038/nature08108). Saturn's, measured at `(2.3 ± 0.7) ×
/// 10⁻⁴` (Lainey et al. 2012, doi:10.1088/0004-637X/752/1/14) and `(1.59
/// ± 0.74) × 10⁻⁴` (Lainey 2016, doi:10.1007/s10569-016-9695-y), is about
/// 15 to 20 times larger; the eccentricities of close-in giants point to a
/// smaller one, `Q′ = 3Q / 2k₂` of 10^6.5, about 5e-7 (Jackson, Greenberg
/// & Barnes 2008, doi:10.1086/529187). Io's response on a giant's radius,
/// whose fifth power the heat follows, gave a close eccentric hot Jupiter
/// 60 times the starlight it absorbs (issue #433); Jupiter's gives it a
/// few per cent.
pub const ENVELOPED_K2_OVER_Q: f64 = 1.1e-5;

/// Least hydrogen and helium share of the mass that makes a body respond
/// to tides as an enveloped one. Calibration, no source: from 0.1 % of
/// the mass, the envelope lifts a 5 M⊕ core by about 0.15 R⊕ (Lopez &
/// Fortney 2014, doi:10.1088/0004-637X/792/1/1), so it sets the radius
/// the tides act on; the mini-Neptune line of [`super::classify`] is the
/// same share.
pub const ENVELOPED_MIN_ENVELOPE: f64 = 1e-3;

/// Least share of water and ices of the mass that makes a body without
/// an envelope respond to tides as an icy one. Calibration, no source:
/// Europa (about 8 % water, see [`super::reference`]) is icy, Io and
/// Earth (0.02 %) are rocky; the ice world line of [`super::classify`] is
/// the same share.
pub const ICY_MIN_WATER: f64 = 0.05;

/// Tidal response `k₂/Q` of a body from what it is made of: its hydrogen
/// and helium share `envelope_fraction` and its water and ice share
/// `water_fraction`, both of the total mass. Enveloped
/// ([`ENVELOPED_K2_OVER_Q`]) from [`ENVELOPED_MIN_ENVELOPE`], whatever
/// its water (Uranus and Neptune); otherwise icy ([`ICY_K2_OVER_Q`]) from
/// [`ICY_MIN_WATER`]; otherwise rocky ([`ROCKY_K2_OVER_Q`]).
pub fn k2_over_q(envelope_fraction: f64, water_fraction: f64) -> f64 {
    if envelope_fraction >= ENVELOPED_MIN_ENVELOPE {
        ENVELOPED_K2_OVER_Q
    } else if water_fraction >= ICY_MIN_WATER {
        ICY_K2_OVER_Q
    } else {
        ROCKY_K2_OVER_Q
    }
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
    fn the_tidal_response_follows_the_composition() {
        // (envelope, water) → k₂/Q.
        let cases = [
            ((0.0, 0.0), ROCKY_K2_OVER_Q),
            ((0.0, 0.049), ROCKY_K2_OVER_Q),
            ((9e-4, 0.0), ROCKY_K2_OVER_Q),
            ((0.0, 0.05), ICY_K2_OVER_Q),
            ((9e-4, 0.4), ICY_K2_OVER_Q),
            ((1e-3, 0.0), ENVELOPED_K2_OVER_Q),
            ((1e-3, 0.4), ENVELOPED_K2_OVER_Q),
            ((0.95, 0.0), ENVELOPED_K2_OVER_Q),
        ];
        for ((envelope, water), expected) in cases {
            assert_eq!(k2_over_q(envelope, water), expected, "{envelope} {water}");
        }
    }

    #[test]
    fn the_solar_system_responds_to_tides_by_its_composition() {
        use crate::planet::reference;
        let expect = [
            ("Mercury", ROCKY_K2_OVER_Q),
            ("Earth", ROCKY_K2_OVER_Q),
            ("Moon", ROCKY_K2_OVER_Q),
            ("Io", ROCKY_K2_OVER_Q),
            ("Europa", ICY_K2_OVER_Q),
            ("Ganymede", ICY_K2_OVER_Q),
            ("Titan", ICY_K2_OVER_Q),
            ("Jupiter", ENVELOPED_K2_OVER_Q),
            ("Saturn", ENVELOPED_K2_OVER_Q),
            ("Uranus", ENVELOPED_K2_OVER_Q),
            ("Neptune", ENVELOPED_K2_OVER_Q),
        ];
        for (name, k2q) in expect {
            let body = reference(name);
            assert_eq!(
                k2_over_q(body.envelope_fraction, body.water_fraction),
                k2q,
                "{name}"
            );
        }
    }

    #[test]
    fn the_icy_response_heats_europa_and_enceladus_as_observed() {
        // Europa: Jupiter 1.898e27 kg, R 1.5608e6 m, a 6.709e8 m, e 0.009.
        let r = 1.5608e6;
        let europa = tidal_power_w(1.898e27, r, 6.709e8, 0.009, ICY_K2_OVER_Q);
        let flux = europa / (4.0 * PI * r * r);
        assert!(
            (0.01..0.05).contains(&flux),
            "Europa {europa} W, {flux} W/m²"
        );
        // Enceladus: Saturn 5.683e26 kg, R 2.521e5 m, a 2.3802e8 m,
        // e 0.0047: a few GW, under the 15.8 GW observed.
        let enceladus = tidal_power_w(5.683e26, 2.521e5, 2.3802e8, 0.0047, ICY_K2_OVER_Q);
        assert!(
            (2e9..15.8e9).contains(&enceladus),
            "Enceladus {enceladus} W"
        );
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
