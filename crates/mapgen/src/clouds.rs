//! Cloud cover, from the climate and the atmosphere's cloud kind.
//!
//! A cover in `[0, 1]` per pixel (0 clear, 1 overcast), exported as
//! `raw_data/clouds.png` and drawn by the viewer above the ground:
//!
//! - **water**: where air rises (the equatorial convergence and the storm
//!   tracks around 60°, the circulation band of the moisture model) and where
//!   it rains (windward slopes, wet coasts); clear where it sinks (the desert
//!   belts around 30°, the poles);
//! - **sulfuric**: a high deck in zonal bands, nearly total at the top of the
//!   cover range (Venus);
//! - **dust**: low and patchy, with a few regional storms;
//! - **none**: no clouds.
//!
//! The texture is a noise stretched east-west, as winds draw clouds into
//! streets and bands. Its features are set in km, so they do not depend on
//! the map size.
use crate::atmosphere::CloudKind;
use crate::climate::circulation_band;
use crate::geo::Grid;
use crate::noise::{fbm, stream, stream_seed};
use noise::Perlin;
use rayon::prelude::*;

/// Wavelength of the largest cloud systems, km.
const SYSTEM_KM: f64 = 3_000.0;

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    crate::noise::smoothstep(edge0, edge1, x)
}

/// What [`clouds`] reads.
pub struct CloudInput<'a> {
    pub grid: Grid,
    pub kind: CloudKind,
    /// Shift of the cover, from −1 (clear) to 1 (overcast).
    pub cover_shift: f32,
    /// Precipitation term of the moisture model, in `[-1, 1]` (wet = 1).
    pub precipitation: &'a [f32],
    pub seed: u32,
}

/// Cloud cover of every pixel in `[0, 1]`, or an empty layer for
/// [`CloudKind::None`].
pub fn clouds(input: &CloudInput) -> Vec<f32> {
    if input.kind == CloudKind::None {
        return Vec::new();
    }
    let grid = input.grid;
    let w = grid.width as usize;
    let perlin = Perlin::new(stream_seed(input.seed, stream::CLOUDS));
    let radius_km = grid.circumference_km as f64 / std::f64::consts::TAU;
    let frequency = radius_km / SYSTEM_KM;
    // North-south squeeze of the noise: clouds stretch along the winds.
    let stretch = match input.kind {
        CloudKind::Sulfuric => 6.0,
        CloudKind::Dust => 1.5,
        _ => 3.0,
    };
    let shift = input.cover_shift;
    (0..grid.pixel_count())
        .into_par_iter()
        .map(|i| {
            let (x, y) = ((i % w) as u32, (i / w) as u32);
            let lat = grid.latitude(y);
            let lon = (x as f64 + 0.5) / grid.width as f64 * std::f64::consts::TAU;
            // The squeeze fades out towards the poles, where east-west
            // has no meaning and it would fold the whole cap into one value.
            let squeeze = 1.0 + (stretch - 1.0) * lat.cos() * lat.cos();
            let p = [
                lat.cos() * lon.cos() * frequency,
                lat.cos() * lon.sin() * frequency,
                lat.sin() * frequency * squeeze,
            ];
            let n = fbm(&perlin, p[0], p[1], p[2], 5);
            match input.kind {
                CloudKind::Water => {
                    // Cold polar air holds little water and rains little,
                    // yet stays overcast with low stratus: fade the rain
                    // term out towards the poles.
                    let polar = smoothstep(0.15, 0.5, lat.cos() as f32);
                    let rain = input.precipitation.get(i).copied().unwrap_or(0.0) * polar;
                    // Earth is about two-thirds overcast: cloudiest along the
                    // storm tracks, clearest over the subtropical highs, but
                    // never bare at the poles.
                    // Polar stratus: the sinking polar air stays overcast.
                    let stratus = 0.35 * (1.0 - polar);
                    let rising = 0.15 * circulation_band(lat) + 0.3 * rain + 0.26 + stratus;
                    smoothstep(0.0, 0.7, rising + 1.4 * n + 0.5 * shift)
                }
                CloudKind::Sulfuric => smoothstep(-0.3, 0.6, 0.2 + 0.7 * shift + 1.0 * n),
                CloudKind::Dust => 0.8 * smoothstep(0.15, 0.6, 0.05 + 0.5 * shift + 1.4 * n),
                CloudKind::None => 0.0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(kind: CloudKind, cover_shift: f32, rain: &[f32]) -> CloudInput<'_> {
        CloudInput {
            grid: Grid {
                width: 128,
                height: 64,
                circumference_km: 40_000.0,
            },
            kind,
            cover_shift,
            precipitation: rain,
            seed: 3,
        }
    }

    fn mean(v: &[f32]) -> f32 {
        v.iter().sum::<f32>() / v.len() as f32
    }

    #[test]
    fn no_clouds_gives_an_empty_layer() {
        assert!(clouds(&input(CloudKind::None, 0.0, &[])).is_empty());
    }

    #[test]
    fn cover_stays_in_range_and_follows_rain_and_shift() {
        let n = 128 * 64;
        let (dry, wet) = (vec![-1.0; n], vec![1.0; n]);
        let water_dry = clouds(&input(CloudKind::Water, 0.0, &dry));
        let water_wet = clouds(&input(CloudKind::Water, 0.0, &wet));
        assert!(water_wet.iter().all(|c| (0.0..=1.0).contains(c)));
        assert!(mean(&water_wet) > mean(&water_dry) + 0.1);
        let venus = clouds(&input(CloudKind::Sulfuric, 1.0, &dry));
        assert!(mean(&venus) > 0.9, "Venus cover {}", mean(&venus));
    }
}
