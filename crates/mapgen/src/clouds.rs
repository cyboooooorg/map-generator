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
    let shift = input.cover_shift;
    (0..grid.pixel_count())
        .into_par_iter()
        .map(|i| {
            let (x, y) = ((i % w) as u32, (i / w) as u32);
            let lat = grid.latitude(y);
            let lon = (x as f64 + 0.5) / grid.width as f64 * std::f64::consts::TAU;
            let unit = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
            match input.kind {
                CloudKind::Water => {
                    let rain = input.precipitation.get(i).copied().unwrap_or(0.0);
                    water(&perlin, unit, lat, frequency, rain, shift)
                }
                CloudKind::Sulfuric | CloudKind::Dust => {
                    // The squeeze fades out towards the poles, where
                    // east-west has no meaning and it would fold the whole
                    // cap into one value.
                    let stretch = if input.kind == CloudKind::Sulfuric {
                        6.0
                    } else {
                        1.5
                    };
                    let squeeze = 1.0 + (stretch - 1.0) * lat.cos() * lat.cos();
                    let n = fbm(
                        &perlin,
                        unit[0] * frequency,
                        unit[1] * frequency,
                        unit[2] * frequency * squeeze,
                        5,
                    );
                    if input.kind == CloudKind::Sulfuric {
                        smoothstep(-0.3, 0.6, 0.2 + 0.7 * shift + n)
                    } else {
                        0.8 * smoothstep(0.15, 0.6, 0.05 + 0.5 * shift + 1.4 * n)
                    }
                }
                CloudKind::None => 0.0,
            }
        })
        .collect()
}

/// Water cloud cover at a point of the unit sphere.
///
/// - **Where**: a narrow convergence band along the equator, the storm
///   tracks around 60° and rainy ground get the most clouds, the subtropical
///   highs around 30° the fewest; cold polar air stays under low stratus.
///   Earth's mean cover is about two-thirds, half of it thin.
/// - **Shape**: the noise is warped by a second, larger noise, which curls
///   cloud systems into swirls and fronts instead of straight streaks. It is
///   stretched east-west only at mid-latitudes, where the westerlies draw
///   fronts into long bands; tropical clouds are convective clusters, polar
///   ones shapeless decks. A finer noise breaks the edges into cells.
fn water(perlin: &Perlin, unit: [f64; 3], lat: f64, frequency: f64, rain: f32, shift: f32) -> f32 {
    let at = |offset: f64, scale: f64, octaves: u32, p: [f64; 3]| {
        fbm(
            perlin,
            p[0] * scale + offset,
            p[1] * scale + offset * 0.7,
            p[2] * scale - offset * 1.3,
            octaves,
        ) as f64
    };
    // Domain warp, in radians of the sphere: up to a system's size.
    let warp = 0.35;
    let base = [unit[0], unit[1], unit[2]];
    let q = [
        base[0] + warp * at(11.3, frequency * 0.6, 3, base),
        base[1] + warp * at(23.7, frequency * 0.6, 3, base),
        base[2] + warp * at(37.1, frequency * 0.6, 3, base),
    ];
    // East-west stretch, strongest around 45°.
    let squeeze = 1.0 + (2.0 * lat).sin().powi(2);
    let q = [q[0], q[1], q[2] * squeeze];
    let systems = at(0.0, frequency, 4, q) as f32;
    let cells = at(51.9, frequency * 5.0, 3, q) as f32;
    let lat_f = lat as f32;
    // The intertropical convergence: a band some 10° wide.
    let itcz = (-(lat_f / 0.14).powi(2)).exp();
    let polar = smoothstep(0.15, 0.5, lat.cos() as f32);
    let stratus = 0.3 * (1.0 - polar);
    let rising = 0.12 * itcz + 0.15 * circulation_band(lat) + 0.25 * rain * polar + stratus;
    let value = rising + 1.5 * systems + (0.6 + 0.3 * itcz) * cells + 0.5 * shift + 0.2;
    smoothstep(0.0, 0.9, value)
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

    #[test]
    fn water_clouds_gather_at_the_equator_and_clear_over_the_subtropics() {
        let n = 128 * 64;
        let cover = clouds(&input(CloudKind::Water, 0.0, &vec![0.0; n]));
        let rows = |lat_min: f64, lat_max: f64| {
            let grid = input(CloudKind::Water, 0.0, &[]).grid;
            let picked: Vec<f32> = (0..n)
                .filter(|i| {
                    let lat = grid.latitude((i / 128) as u32).to_degrees().abs();
                    (lat_min..lat_max).contains(&lat)
                })
                .map(|i| cover[i])
                .collect();
            mean(&picked)
        };
        assert!(rows(0.0, 8.0) > rows(25.0, 35.0) + 0.15);
        assert!(
            (0.3..0.6).contains(&mean(&cover)),
            "mean cover {}",
            mean(&cover)
        );
    }
}
