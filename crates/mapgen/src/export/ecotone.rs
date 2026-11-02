//! Soft biome borders (ecotones) for rendering.
//!
//! Biome selection is a hard classification: every threshold is a one-pixel
//! colour step. Real borders are gradual. A pixel's colour is the mean colour
//! of the biomes its climate selects when nudged by small temperature and
//! moisture offsets: far from any threshold every offset gives the same
//! biome and the colour is flat; near a threshold the colours mix, over a
//! width set in climate units (not pixels), so it suits any zoom level.
//!
//! Only temperature and moisture are nudged: coastlines, shores, rivers,
//! lakes and volcanic biomes keep their exact inputs, so they stay crisp.
//! This is rendering only: the exported biome layer is unchanged.
use crate::biome::{BiomeInput, biome_color};
use crate::world::World;

/// Standard deviation of the temperature offsets (`[0, 1]` scale).
const SIGMA_T: f32 = 0.02;
/// Standard deviation of the moisture offsets (`[-1, 1]` scale).
const SIGMA_M: f32 = 0.05;
/// Offsets per pixel.
const SAMPLES: usize = 16;

/// Offsets drawn from a 2D unit Gaussian: equal-area rings on a golden-angle
/// spiral, so equal weights give the Gaussian mean. Each point comes with its
/// opposite, so the offsets average to exactly zero and never bias colours.
fn unit_offsets() -> [(f32, f32); SAMPLES] {
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    let half = SAMPLES / 2;
    std::array::from_fn(|k| {
        let j = k % half;
        let u = (j as f32 + 0.5) / half as f32;
        let r = (-2.0 * (1.0 - u).ln()).sqrt();
        let a = j as f32 * golden;
        let sign = if k < half { 1.0 } else { -1.0 };
        (sign * r * a.cos(), sign * r * a.sin())
    })
}

/// Cheap per-pixel hash in `[0, 1)`, to rotate the offsets so the finite
/// sample count shows as fine grain rather than terraces.
fn hash(i: usize) -> f32 {
    let mut x = i as u32 ^ 0x9E37_79B9;
    x = (x ^ (x >> 16)).wrapping_mul(0x85EB_CA6B);
    x = (x ^ (x >> 13)).wrapping_mul(0xC2B2_AE35);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Colour of pixel `i` with soft borders, as linear-ish sRGB floats in `[0, 255]`.
/// `input` is the exact biome input of the pixel.
pub fn ecotone_colour(world: &World, i: usize, input: BiomeInput) -> [f32; 3] {
    let planet = &world.params.planet;
    let (sin, cos) = (hash(i) * std::f32::consts::TAU).sin_cos();
    let mut sum = [0.0f32; 3];
    for (x, y) in unit_offsets() {
        let (dt, dm) = (x * cos - y * sin, x * sin + y * cos);
        let nudged = BiomeInput {
            temperature: (input.temperature + dt * SIGMA_T).clamp(0.0, 1.0),
            moisture: (input.moisture + dm * SIGMA_M).clamp(-1.0, 1.0),
            ..input
        };
        let colour = biome_color(planet.remap(planet.rules.choose(nudged)));
        for (s, c) in sum.iter_mut().zip(colour) {
            *s += c as f32;
        }
    }
    sum.map(|s| s / SAMPLES as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_are_centred_with_unit_spread() {
        let offsets = unit_offsets();
        let mean = offsets
            .iter()
            .fold((0.0, 0.0), |a, o| (a.0 + o.0, a.1 + o.1));
        assert!(mean.0.abs() < 1e-5 && mean.1.abs() < 1e-5, "{mean:?}");
        let var = offsets.iter().map(|o| o.0 * o.0 + o.1 * o.1).sum::<f32>() / SAMPLES as f32;
        // E[x² + y²] = 2 for a 2D unit Gaussian.
        assert!((var - 2.0).abs() < 0.3, "{var}");
    }

    #[test]
    fn hash_is_in_unit_range() {
        for i in [0, 1, 17, 1 << 20, usize::from(u16::MAX)] {
            assert!((0.0..1.0).contains(&hash(i)));
        }
    }
}
