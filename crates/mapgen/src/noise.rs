//! Shared noise primitives used by both world generation and noise-map export.
use noise::{NoiseFn, Perlin};

/// Earth's equatorial circumference used as the noise-scale baseline.
pub const EARTH_CIRCUMFERENCE_KM: f32 = 40_075.0;

/// Fractional Brownian Motion — combines multiple octaves of Perlin noise for
/// natural-looking detail.  Samples all three sphere-surface axes so there is
/// no mirror symmetry along any axis.
///
/// * `noise`   — Perlin noise generator to sample.
/// * `x, y, z` — 3-D coordinates to sample (unit-sphere surface).
/// * `octaves` — Number of noise layers to combine; more = more detail.
///
/// Returns a normalised value in roughly `[-1.0, 1.0]`.
pub fn fbm(noise: &Perlin, x: f64, y: f64, z: f64, octaves: u32) -> f32 {
    let mut value = 0.0f64;
    let mut amplitude = 1.0f64;
    let mut frequency = 1.0f64;
    let mut max_value = 0.0f64;

    for _ in 0..octaves {
        value += noise.get([x * frequency, y * frequency, z * frequency]) * amplitude;
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    (value / max_value) as f32
}

/// Ridged noise — inverts the absolute value to produce sharp mountain peaks
/// instead of smooth hills.  Returns a value in `[0.0, 1.0]`.
pub fn ridged(noise: &Perlin, x: f64, y: f64, z: f64) -> f32 {
    let v = noise.get([x, y, z]) as f32;
    1.0 - v.abs()
}

/// Voronoi cell noise: the value, in `[-1, 1]`, of the cell whose jittered
/// feature point is nearest to `(x, y, z)`, blended with its neighbours over
/// a border about `1 / sharpness` cells wide. Nearly piecewise constant, with
/// straight borders between cells: added to a smooth field, it gives
/// contours straight stretches and angular corners, without cliffs.
pub fn cell_value(seed: u32, x: f64, y: f64, z: f64, sharpness: f64) -> f32 {
    let (cx, cy, cz) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
    let mut cells = [(0.0f64, 0.0f64); 27];
    let mut nearest = f64::MAX;
    let mut n = 0;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (i, j, k) = (cx + dx, cy + dy, cz + dz);
                let h = hash(seed, i, j, k);
                let unit = |shift: u32| ((h >> shift) & 0xFFFF) as f64 / 65_535.0;
                let p = [i as f64 + unit(0), j as f64 + unit(16), k as f64 + unit(32)];
                let d = ((p[0] - x).powi(2) + (p[1] - y).powi(2) + (p[2] - z).powi(2)).sqrt();
                nearest = nearest.min(d);
                cells[n] = (d, unit(48) * 2.0 - 1.0);
                n += 1;
            }
        }
    }
    let (mut sum, mut total) = (0.0, 0.0);
    for &(d, value) in &cells {
        let weight = (-(d - nearest) * sharpness).exp();
        sum += weight * value;
        total += weight;
    }
    (sum / total) as f32
}

/// The points of a jittered lattice (one point per unit cell, present
/// with probability `presence`) in the cells around `(x, y, z)`: each one's
/// distance from it, in cells, and its hash, for per-point properties.
pub fn lattice_points(
    seed: u32,
    x: f64,
    y: f64,
    z: f64,
    presence: f64,
) -> impl Iterator<Item = (f64, u64)> {
    let (cx, cy, cz) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
    (0..27).filter_map(move |n: i64| {
        let (i, j, k) = (cx + n % 3 - 1, cy + (n / 3) % 3 - 1, cz + n / 9 - 1);
        let h = hash(seed, i, j, k);
        if ((h >> 48) as f64 / 65_536.0) >= presence {
            return None;
        }
        let unit = |shift: u32| ((h >> shift) & 0xFFFF) as f64 / 65_535.0;
        let p = [i as f64 + unit(0), j as f64 + unit(16), k as f64 + unit(32)];
        let d = ((p[0] - x).powi(2) + (p[1] - y).powi(2) + (p[2] - z).powi(2)).sqrt();
        Some((d, h))
    })
}

/// Hermite step from 0 at `edge0` to 1 at `edge1`, clamped outside.
pub(crate) fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Independent noise streams of a world: each gets its own seed from
/// [`stream_seed`].
pub mod stream {
    pub const ELEVATION: u32 = 1;
    pub const MOISTURE: u32 = 2;
    pub const CONTINENT: u32 = 3;
    pub const WARP_A: u32 = 4;
    pub const WARP_B: u32 = 5;
    pub const VOLCANO: u32 = 6;
    pub const ANOMALY: u32 = 7;
    pub const VARIABILITY: u32 = 8;
    pub const HIGHLAND: u32 = 9;
    pub const COAST: u32 = 10;
    pub const PLATES: u32 = 11;
    pub const CONES: u32 = 12;
    pub const FLATS: u32 = 13;
    pub const ROUTING: u32 = 14;
    pub const MEANDERS: u32 = 15;
    pub const DELTAS: u32 = 16;
    pub const EROSION: u32 = 17;
    pub const EROSION_FINE: u32 = 18;
    pub const CLOUDS: u32 = 19;
    /// Colour variation within biomes (rendering only).
    pub const SURFACE: u32 = 20;
    /// Impact craters: positions, sizes, ages and rim shapes.
    pub const CRATERS: u32 = 21;
    /// Impact craters still to come, for the time slider.
    pub const FUTURE_CRATERS: u32 = 22;
    /// Tectonic plates: their centres, weights and Euler poles, and the
    /// jagged subplates they are made of.
    pub const PLATE_MOTION: u32 = 23;
    pub const SUBPLATES: u32 = 24;
    /// Volcanoes of the volcanic arcs over subducting plates.
    pub const ARC_CONES: u32 = 25;
    /// Moons: their number, origins, sizes and orbits.
    pub const MOONS: u32 = 26;
    /// The star's metallicity, when drawn.
    pub const METALLICITY: u32 = 27;
    /// Giant planets: their number, masses, orbits and looks.
    pub const GIANTS: u32 = 28;
    /// The bands, storms, spin and rings of a giant world.
    pub const GIANT_LOOK: u32 = 29;
    /// Whether a world orbits a giant, and that giant and orbit.
    pub const HOST: u32 = 30;
    /// The seeds of a giant's other regular moons (see
    /// `crate::exomoon::sibling_params`), one stream past this per moon.
    pub const SIBLINGS: u32 = 31;
    pub const BELTS: u32 = 32;
    pub const TROJANS: u32 = 33;
    /// The rings of a giant of a planetary system, drawn with the system
    /// (see `crate::system`): 32 and 33 are the belts' and the trojans'.
    pub const RINGS: u32 = 34;
    /// The moons in and next to those rings (see `crate::rings`).
    pub const RING_MOONS: u32 = 35;
    /// The dust rings its moons feed (see `crate::rings::dust_rings`).
    pub const DUST_RINGS: u32 = 36;
    /// A system's draw (see `crate::system`): its planets, and its bodies'
    /// seeds and orbits.
    pub const SYSTEM: u32 = 64;
}

/// The seed of one noise stream (see [`stream`]) of the world `seed`.
///
/// Hashed rather than offset (`seed + 100`…): offsets let neighbouring
/// worlds share fields (the moisture of one world being the elevation of
/// the next) and two streams collide when their offsets do.
pub fn stream_seed(seed: u32, stream: u32) -> u32 {
    let mut z = ((seed as u64) << 32 | stream as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) as u32
}

/// SplitMix64 hash of a lattice cell.
fn hash(seed: u32, i: i64, j: i64, k: i64) -> u64 {
    let mut z = (seed as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((i as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((j as u64).wrapping_mul(0x94D0_49BB_1331_11EB))
        .wrapping_add((k as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_seeds_differ_between_streams_and_worlds() {
        let mut seen = std::collections::HashSet::new();
        for seed in [0, 1, 2, 100, u32::MAX] {
            for s in stream::ELEVATION..=stream::MOONS {
                assert!(seen.insert(stream_seed(seed, s)), "seed {seed} stream {s}");
            }
        }
    }
}
