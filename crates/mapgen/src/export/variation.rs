//! Small colour variation within a biome, for rendering.
//!
//! A biome is one colour, so a large forest or desert is a flat plate under
//! its hillshade. Real surfaces vary: soil, rock and vegetation density
//! change over tens to hundreds of kilometres. [`SurfaceVariation`] shifts
//! the brightness and the warmth (red against blue) of land colours with two
//! smooth noise fields on the sphere. The fields depend on the position
//! alone, at fixed wavelengths in kilometres, so the base map and the detail
//! tiles of a world show the same variation. This is rendering only.
use crate::noise::{fbm, stream, stream_seed};
use noise::Perlin;

/// Longest wavelength of the variation, km.
const WAVELENGTH_KM: f64 = 400.0;
/// Octaves below [`WAVELENGTH_KM`]: the finest is 50 km, about two and a half base
/// pixels on an Earth-sized world, so the base map resolves it.
const OCTAVES: u32 = 4;
/// Largest relative change of brightness.
const TONE: f32 = 0.18;
/// Largest relative shift of red against blue.
const WARMTH: f32 = 0.08;
/// Gain on the noise, whose octave sum rarely leaves `[-0.5, 0.5]`.
const GAIN: f32 = 2.5;

/// The variation fields of one world.
pub struct SurfaceVariation {
    tone: Perlin,
    warmth: Perlin,
    /// Noise frequency on the unit sphere.
    frequency: f64,
}

impl SurfaceVariation {
    /// Fields of the world `seed`, whose equator is `circumference_km` long.
    pub fn new(seed: u32, circumference_km: f64) -> Self {
        let radius_km = circumference_km / std::f64::consts::TAU;
        Self {
            tone: Perlin::new(stream_seed(seed, stream::SURFACE)),
            warmth: Perlin::new(stream_seed(seed, stream::SURFACE).wrapping_add(1)),
            frequency: radius_km / WAVELENGTH_KM,
        }
    }

    /// `colour` (sRGB in `[0, 255]`) varied at longitude `lon` and latitude
    /// `lat`, in radians.
    pub fn apply(&self, colour: [f32; 3], lon: f64, lat: f64) -> [f32; 3] {
        let f = self.frequency;
        let p = [
            lat.cos() * lon.cos() * f,
            lat.cos() * lon.sin() * f,
            lat.sin() * f,
        ];
        let field =
            |noise: &Perlin| (fbm(noise, p[0], p[1], p[2], OCTAVES) * GAIN).clamp(-1.0, 1.0);
        let tone = 1.0 + TONE * field(&self.tone);
        let warmth = WARMTH * field(&self.warmth);
        [
            colour[0] * tone * (1.0 + warmth),
            colour[1] * tone,
            colour[2] * tone * (1.0 - warmth),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variation_is_small_deterministic_and_varies() {
        let a = SurfaceVariation::new(42, 40_075.0);
        let b = SurfaceVariation::new(42, 40_075.0);
        let grey = [100.0; 3];
        let mut seen = std::collections::BTreeSet::new();
        for k in 0..200 {
            let (lon, lat) = (k as f64 * 0.031, (k as f64 * 0.017).sin());
            let c = a.apply(grey, lon, lat);
            assert_eq!(c, b.apply(grey, lon, lat));
            for channel in c {
                assert!((channel - 100.0).abs() <= 100.0 * (TONE + WARMTH) + 1.0);
            }
            seen.insert(c[1].round() as i32);
        }
        assert!(seen.len() > 5, "flat variation: {seen:?}");
    }
}
