//! Small colour variation within a biome, for rendering.
//!
//! A biome is one colour, so a large forest or desert is a flat plate under
//! its hillshade. Real surfaces vary: soil, rock and vegetation density
//! change over tens to hundreds of kilometres. [`SurfaceVariation`] shifts
//! the brightness and the warmth (red against blue) of land colours with two
//! smooth noise fields on the sphere. The fields depend on the position
//! alone, at fixed wavelengths in kilometres, so the base map and the detail
//! tiles of a world show the same variation. This is rendering only.
//!
//! Seas get a pattern of their own kind ([`SurfaceVariation::sea`]): swirls
//! of plankton-like colour drawn by currents on water and acid, open leads
//! between floes on ice, rafts of cooled crust on magma.
use super::ecotone::{linear_to_srgb, srgb_to_linear};
use crate::biome::Biome;
use crate::noise::{fbm, smoothstep, stream, stream_seed};
use noise::{NoiseFn, Perlin};

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

/// Longest wavelength of the swirls on water and acid seas, km.
const SWIRL_WAVELENGTH_KM: f64 = 900.0;
/// Octaves of the swirls: down to about 56 km.
const SWIRL_OCTAVES: u32 = 5;
/// Strength of the domain warp that bends the swirls into eddies, in
/// noise units.
const SWIRL_WARP: f64 = 0.8;
/// Largest share of the swirl colour over a water pixel.
const SWIRL_SHARE: f32 = 0.3;
/// Largest share of the swirl colour over an acid pixel: its tint is far
/// brighter than the acid, and stronger swirls read as islands.
const ACID_SWIRL_SHARE: f32 = 0.14;
/// Largest relative change of brightness of a sea.
const SEA_TONE: f32 = 0.08;
/// Swirl colour on water: a teal plankton bloom, sRGB.
const WATER_SWIRL: [f32; 3] = [24.0, 92.0, 84.0];
/// Swirl colour on acid: a sulphur-yellow precipitate, sRGB.
const ACID_SWIRL: [f32; 3] = [150.0, 150.0, 30.0];
/// Longest wavelength of the cracks between ice floes, km.
const FLOE_WAVELENGTH_KM: f64 = 700.0;
/// Largest share of open water in a lead between floes.
const LEAD_SHARE: f32 = 0.35;
/// Colour of the open water in leads, sRGB.
const LEAD: [f32; 3] = [36.0, 72.0, 104.0];

/// The variation fields of one world.
pub struct SurfaceVariation {
    tone: Perlin,
    warmth: Perlin,
    /// Crust and hot spots on magma seas.
    magma: Perlin,
    /// Swirls on water and acid seas, and the brightness of every sea.
    sea: Perlin,
    /// Domain warp of the swirls.
    sea_warp: Perlin,
    /// Cracks between ice floes.
    floes: Perlin,
    /// Planet radius, km.
    radius_km: f64,
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
            sea: Perlin::new(stream_seed(seed, stream::SURFACE).wrapping_add(3)),
            sea_warp: Perlin::new(stream_seed(seed, stream::SURFACE).wrapping_add(4)),
            floes: Perlin::new(stream_seed(seed, stream::SURFACE).wrapping_add(5)),
            radius_km,
            frequency: radius_km / WAVELENGTH_KM,
            magma_frequency: radius_km / MAGMA_WAVELENGTH_KM,
        }
    }

    /// `colour` of a sea, lake or ice pixel of `biome` (sRGB in `[0, 255]`)
    /// at longitude `lon` and latitude `lat`, varied by the kind of liquid:
    /// swirls on water and acid, leads between floes on ice, crust on
    /// magma. Rivers are returned unchanged. Mixed in linear light.
    pub fn sea(&self, biome: Biome, colour: [f32; 3], lon: f64, lat: f64) -> [f32; 3] {
        match biome {
            Biome::MagmaSea => self.magma(colour, lon, lat),
            Biome::DeepOcean | Biome::Ocean | Biome::Lake => {
                self.swirl(colour, WATER_SWIRL, SWIRL_SHARE, lon, lat)
            }
            Biome::DeepAcidSea | Biome::AcidSea | Biome::CausticLake => {
                self.swirl(colour, ACID_SWIRL, ACID_SWIRL_SHARE, lon, lat)
            }
            Biome::FrozenOcean | Biome::SeaIce | Biome::FrozenLake => self.floes(colour, lon, lat),
            _ => colour,
        }
    }

    /// Brightness factor of a sea, `1 ± SEA_TONE`.
    fn sea_tone(&self, p: [f64; 3]) -> f32 {
        let n = fbm(&self.sea, p[0] + 31.7, p[1] - 12.9, p[2] + 7.1, 3) * GAIN;
        1.0 + SEA_TONE * n.clamp(-1.0, 1.0)
    }

    /// Currents drawn as warped swirls of `tint`, up to `strength`, over
    /// `colour`.
    fn swirl(
        &self,
        colour: [f32; 3],
        tint: [f32; 3],
        strength: f32,
        lon: f64,
        lat: f64,
    ) -> [f32; 3] {
        let p = sphere(lon, lat, self.radius_km / SWIRL_WAVELENGTH_KM);
        let warp = |dx: f64, dy: f64, dz: f64| {
            SWIRL_WARP * fbm(&self.sea_warp, p[0] + dx, p[1] + dy, p[2] + dz, 2) as f64
        };
        let q = [
            p[0] + warp(0.0, 0.0, 0.0),
            p[1] + warp(5.2, 1.3, -2.8),
            p[2] + warp(-1.7, 9.2, 3.4),
        ];
        let n = (fbm(&self.sea, q[0], q[1], q[2], SWIRL_OCTAVES) * GAIN).clamp(-1.0, 1.0);
        let share = strength * smoothstep(0.1, 0.9, n);
        let tone = self.sea_tone(p);
        std::array::from_fn(|k| {
            let base = srgb_to_linear(colour[k]);
            linear_to_srgb((base + (srgb_to_linear(tint[k]) - base) * share) * tone)
        })
    }

    /// Ice broken into floes: thin, darker leads of open water along the
    /// crests of a ridged noise.
    fn floes(&self, colour: [f32; 3], lon: f64, lat: f64) -> [f32; 3] {
        let p = sphere(lon, lat, self.radius_km / FLOE_WAVELENGTH_KM);
        let ridge = |scale: f64| {
            1.0 - (self.floes.get([p[0] * scale, p[1] * scale, p[2] * scale]) as f32).abs()
        };
        let crack = ridge(1.0).max(ridge(2.3) * 0.97);
        let share = LEAD_SHARE * smoothstep(0.92, 0.99, crack);
        let tone = self.sea_tone(p);
        std::array::from_fn(|k| {
            let base = srgb_to_linear(colour[k]);
            linear_to_srgb((base + (srgb_to_linear(LEAD[k]) - base) * share) * tone)
        })
    }

    /// `colour` of a magma sea pixel (sRGB in `[0, 255]`) at longitude
    /// `lon` and latitude `lat`: rafts of dark cooled crust drift over it,
    /// and the magma shows yellow-hot between the largest gaps. Mixed in
    /// linear light.
    fn magma(&self, colour: [f32; 3], lon: f64, lat: f64) -> [f32; 3] {
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
            let c = v.sea(Biome::MagmaSea, magma, lon, lat);
            darkest = darkest.min(c[0]);
            brightest = brightest.max(c[1]);
        }
        assert!(darkest < 0.75 * magma[0], "no crust: darkest red {darkest}");
        assert!(
            brightest > 1.25 * magma[1],
            "no hot spot: brightest green {brightest}"
        );
    }

    #[test]
    fn every_sea_varies_and_rivers_do_not() {
        let v = SurfaceVariation::new(7, 40_075.0);
        for biome in [Biome::Ocean, Biome::AcidSea, Biome::SeaIce, Biome::MagmaSea] {
            let base = [80.0, 90.0, 100.0];
            let colours: std::collections::BTreeSet<[i32; 3]> = (0..2000)
                .map(|k| {
                    let (lon, lat) = (k as f64 * 0.0123, (k as f64 * 0.0071).sin());
                    v.sea(biome, base, lon, lat).map(|c| c.round() as i32)
                })
                .collect();
            assert!(colours.len() > 20, "{biome:?}: {} colours", colours.len());
        }
        assert_eq!(
            v.sea(Biome::River, [1.0, 2.0, 3.0], 0.3, 0.2),
            [1.0, 2.0, 3.0]
        );
    }
}
