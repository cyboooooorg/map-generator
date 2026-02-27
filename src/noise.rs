/// Shared noise primitives used by both world generation and noise-map export.
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
