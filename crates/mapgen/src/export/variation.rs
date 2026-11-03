//! Small colour variation within a biome, for rendering.
//!
//! A biome is one colour, so a large forest or desert is a flat plate under
//! its hillshade. Real surfaces vary: soil, rock and vegetation density
//! change over tens to hundreds of kilometres. [`SurfaceVariation`] shifts
//! the brightness and the warmth (red against blue) of land colours with two
//! smooth noise fields on the sphere. The fields depend on the position
//! alone, at fixed wavelengths in kilometres, so the base map and the detail
//! tiles of a world show the same variation. This is rendering only.
use super::ecotone::{linear_to_srgb, srgb_to_linear};
use crate::noise::{fbm, smoothstep, stream, stream_seed};
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

/// Longest wavelength of the crust on magma seas, km.
const MAGMA_WAVELENGTH_KM: f64 = 1200.0;
/// Octaves of the magma crust: down to about 75 km.
const MAGMA_OCTAVES: u32 = 5;
/// Colour of cooled crust floating on magma, sRGB.
const CRUST: [f32; 3] = [52.0, 22.0, 14.0];
/// Colour of the hottest, freshly exposed magma, sRGB.
const HOT: [f32; 3] = [255.0, 150.0, 40.0];
/// Largest share of crust over a magma pixel.
const CRUST_COVER: f32 = 0.6;
/// Largest share of the hot colour.
const HOT_SHARE: f32 = 0.3;

/// The variation fields of one world.
pub struct SurfaceVariation {
    tone: Perlin,
    warmth: Perlin,
    /// Crust and hot spots on magma seas.
    magma: Perlin,
    /// Noise frequency on the unit sphere.
    frequency: f64,
    /// Noise frequency of the magma crust on the unit sphere.
    magma_frequency: f64,
}

impl SurfaceVariation {
    /// Fields of the world `seed`, whose equator is `circumference_km` long.
    pub fn new(seed: u32, circumference_km: f64) -> Self {
        let radius_km = circumference_km / std::f64::consts::TAU;
        Self {
            tone: Perlin::new(stream_seed(seed, stream::SURFACE)),
            warmth: Perlin::new(stream_seed(seed, stream::SURFACE).wrapping_add(1)),
            magma: Perlin::new(stream_seed(seed, stream::SURFACE).wrapping_add(2)),
            frequency: radius_km / WAVELENGTH_KM,
            magma_frequency: radius_km / MAGMA_WAVELENGTH_KM,
        }
    }

    /// `colour` of a magma sea pixel (sRGB in `[0, 255]`) at longitude
    /// `lon` and latitude `lat`: rafts of dark cooled crust drift over it,
    /// and the magma shows yellow-hot between the largest gaps. Mixed in
    /// linear light.
    pub fn magma(&self, colour: [f32; 3], lon: f64, lat: f64) -> [f32; 3] {
        let f = self.magma_frequency;
        let p = sphere(lon, lat, f);
        let n = (fbm(&self.magma, p[0], p[1], p[2], MAGMA_OCTAVES) * GAIN).clamp(-1.0, 1.0);
        let crust = CRUST_COVER * smoothstep(0.15, 0.75, n);
        let hot = HOT_SHARE * smoothstep(-0.35, -0.85, n);
        std::array::from_fn(|k| {
            let base = srgb_to_linear(colour[k]);
            let mixed = base
                + (srgb_to_linear(CRUST[k]) - base) * crust
                + (srgb_to_linear(HOT[k]) - base) * hot;
            linear_to_srgb(mixed)
        })
    }

    /// `colour` (sRGB in `[0, 255]`) varied at longitude `lon` and latitude
    /// `lat`, in radians.
    pub fn apply(&self, colour: [f32; 3], lon: f64, lat: f64) -> [f32; 3] {
        let p = sphere(lon, lat, self.frequency);
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

/// Point at `lon`, `lat` (radians) on a sphere of radius `f`.
fn sphere(lon: f64, lat: f64, f: f64) -> [f64; 3] {
    [
        lat.cos() * lon.cos() * f,
        lat.cos() * lon.sin() * f,
        lat.sin() * f,
    ]
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

    #[test]
    fn magma_shows_crust_and_hot_spots() {
        let v = SurfaceVariation::new(42, 40_075.0);
        let magma = [196.0, 52.0, 12.0];
        let (mut darkest, mut brightest) = (f32::MAX, 0.0f32);
        for k in 0..2000 {
            let (lon, lat) = (k as f64 * 0.0123, (k as f64 * 0.0071).sin());
            let c = v.magma(magma, lon, lat);
            darkest = darkest.min(c[0]);
            brightest = brightest.max(c[1]);
        }
        assert!(darkest < 0.75 * magma[0], "no crust: darkest red {darkest}");
        assert!(brightest > 1.25 * magma[1], "no hot spot: brightest green {brightest}");
    }
}
