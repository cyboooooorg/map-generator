//! A system's place in the Milky Way: where it sits and how its frame is
//! turned, so the sky seen from it (the band, the clouds, the stars) is
//! its own and a metal-rich star under a bright, crowded sky is not
//! drawn like a metal-poor one in the outskirts.
//!
//! **Frame.** Galactocentric Cartesian, in kpc: the centre at the origin,
//! +z the galactic north pole, the Sun at `(8.2, 0, 0.02)`. A system sits
//! at `(R cos φ, R sin φ, z)`. [`GalacticPlace::axes`] gives the galactic
//! x, y, z axes expressed in the system's frame (the star's equatorial
//! frame, y its spin axis).
//!
//! **Radius.** The disc's metallicity falls with radius at about
//! −0.06 dex/kpc (Cepheids, Genovali et al. 2014), so the system's
//! `[Fe/H]` is inverted into a radius, with the 0.1 dex spread of
//! `[Fe/H]` at a given radius turned into 1.67 kpc. The generator's mean
//! metallicity (−0.1) lands on the Sun's radius.
//!
//! **Height.** Two-sided exponential, scale height 0.3 kpc (thin disc).
//!
//! Everything is drawn from [`stream::GALAXY`], shared with no other draw,
//! so no other value of a system shifts.
use crate::giants::normal;
use crate::noise::{stream, stream_seed};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

/// The Sun's galactocentric radius, kpc.
pub const SUN_RADIUS_KPC: f64 = 8.2;
/// `[Fe/H]` at the Sun's radius (the generator's mean metallicity).
const SUN_METALLICITY: f64 = -0.1;
/// Radial metallicity gradient of the disc, dex/kpc (Genovali et al. 2014).
const GRADIENT: f64 = 0.06;
/// Scatter of the radius, kpc: 0.1 dex of `[Fe/H]` over the gradient.
const RADIUS_SCATTER_KPC: f64 = 1.67;
/// The disc, kpc.
const RADIUS_RANGE: (f64, f64) = (2.0, 16.0);
/// Scale height of the thin disc, kpc.
const SCALE_HEIGHT_KPC: f64 = 0.3;
/// Largest distance from the plane drawn, kpc.
const MAX_HEIGHT_KPC: f64 = 1.5;

/// Where a place comes from: drawn from the seed, or taken from a
/// catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "lowercase")]
pub enum PlaceSource {
    Drawn,
    Observed,
}

/// A system's place and orientation in the galaxy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GalacticPlace {
    pub source: PlaceSource,
    /// Galactocentric position, kpc (see the module doc for the frame).
    pub position_kpc: [f64; 3],
    /// The galactic x, y, z axes in the system's frame (orthonormal,
    /// right-handed).
    pub axes: [[f64; 3]; 3],
    /// Seed of the sky's clouds and stars.
    pub sky_seed: u32,
}

/// Draws the place of the system `seed` whose star has `[Fe/H]` =
/// `metallicity`.
pub fn draw_place(seed: u32, metallicity: f32) -> GalacticPlace {
    let mut rng = StdRng::seed_from_u64(u64::from(stream_seed(seed, stream::GALAXY)));
    let radius = (SUN_RADIUS_KPC
        + (SUN_METALLICITY - f64::from(metallicity)) / GRADIENT
        + RADIUS_SCATTER_KPC * normal(&mut rng))
    .clamp(RADIUS_RANGE.0, RADIUS_RANGE.1);
    let magnitude = -(1.0 - rng.random::<f64>()).ln() * SCALE_HEIGHT_KPC;
    let sign = if rng.random::<bool>() { 1.0 } else { -1.0 };
    let height = (sign * magnitude).clamp(-MAX_HEIGHT_KPC, MAX_HEIGHT_KPC);
    let azimuth = std::f64::consts::TAU * rng.random::<f64>();
    let axes = random_rotation(&mut rng);
    let sky_seed = rng.random::<u32>();
    GalacticPlace {
        source: PlaceSource::Drawn,
        position_kpc: [radius * azimuth.cos(), radius * azimuth.sin(), height],
        axes,
        sky_seed,
    }
}

/// A uniform random rotation, from a random unit quaternion; returns its
/// columns.
fn random_rotation(rng: &mut impl RngExt) -> [[f64; 3]; 3] {
    let mut q = [0.0; 4];
    let mut norm = 0.0;
    while norm < 1e-9 {
        q = [normal(rng), normal(rng), normal(rng), normal(rng)];
        norm = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    }
    let [w, x, y, z] = q.map(|v| v / norm);
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + w * z),
            2.0 * (x * z - w * y),
        ],
        [
            2.0 * (x * y - w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + w * x),
        ],
        [
            2.0 * (x * z + w * y),
            2.0 * (y * z - w * x),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radius(p: &GalacticPlace) -> f64 {
        p.position_kpc[0].hypot(p.position_kpc[1])
    }

    fn mean_radius(metallicity: f32) -> f64 {
        (0..2000)
            .map(|s| radius(&draw_place(s, metallicity)))
            .sum::<f64>()
            / 2000.0
    }

    #[test]
    fn a_place_is_deterministic() {
        assert_eq!(draw_place(7, -0.1), draw_place(7, -0.1));
    }

    #[test]
    fn the_mean_metallicity_sits_at_the_sun() {
        assert!((mean_radius(-0.1) - SUN_RADIUS_KPC).abs() < 0.2);
    }

    #[test]
    fn metal_rich_stars_sit_closer_in() {
        assert!(mean_radius(0.3) < mean_radius(-0.5) - 5.0);
    }

    #[test]
    fn radius_and_height_stay_in_the_disc() {
        for m in [-1.0, 0.5] {
            for s in 0..2000 {
                let p = draw_place(s, m);
                assert!(
                    (2.0..=16.0).contains(&(radius(&p) + 0.0))
                        || (radius(&p) - 2.0).abs() < 1e-9
                        || (radius(&p) - 16.0).abs() < 1e-9
                );
                assert!(p.position_kpc[2].abs() <= 1.5);
            }
        }
    }

    #[test]
    fn axes_are_a_rotation() {
        for s in 0..200 {
            let a = draw_place(s, -0.1).axes;
            for i in 0..3 {
                for j in 0..3 {
                    let dot: f64 = (0..3).map(|k| a[i][k] * a[j][k]).sum();
                    let want = if i == j { 1.0 } else { 0.0 };
                    assert!((dot - want).abs() < 1e-9);
                }
            }
            let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
                - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
            assert!((det - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn seeds_give_different_sky_seeds() {
        let seeds: std::collections::HashSet<u32> =
            (0..100).map(|s| draw_place(s, -0.1).sky_seed).collect();
        assert!(seeds.len() > 95);
    }
}
