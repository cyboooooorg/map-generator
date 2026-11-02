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

/// Nearest point of a jittered lattice (one point per unit cell, present
/// with probability `presence`): its distance from `(x, y, z)`, in cells,
/// and its hash, for per-point properties. `None` when no point lies in the
/// surrounding cells.
pub fn nearest_point(seed: u32, x: f64, y: f64, z: f64, presence: f64) -> Option<(f64, u64)> {
    let (cx, cy, cz) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
    let mut best: Option<(f64, u64)> = None;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (i, j, k) = (cx + dx, cy + dy, cz + dz);
                let h = hash(seed, i, j, k);
                if ((h >> 48) as f64 / 65_536.0) >= presence {
                    continue;
                }
                let unit = |shift: u32| ((h >> shift) & 0xFFFF) as f64 / 65_535.0;
                let p = [i as f64 + unit(0), j as f64 + unit(16), k as f64 + unit(32)];
                let d = ((p[0] - x).powi(2) + (p[1] - y).powi(2) + (p[2] - z).powi(2)).sqrt();
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, h));
                }
            }
        }
    }
    best
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
