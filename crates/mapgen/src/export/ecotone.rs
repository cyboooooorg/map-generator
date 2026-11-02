//! Soft biome borders (ecotones) for rendering.
//!
//! Biome selection is a hard classification: every threshold is a one-pixel
//! colour step. Real borders are gradual. A pixel's colour is the mean colour
//! of the biomes its climate selects when nudged by small offsets of its
//! temperature, moisture, highland line and volcanic zone: far from any
//! threshold every offset gives the same biome and the colour is flat; near
//! one the colours mix, over a width set in input units (not pixels), so it
//! suits any zoom level.
//!
//! The highland line is nudged through `highland_offset`, so mountain and
//! snow borders soften without moving the coastline. The volcanic zone is
//! nudged less: volcanoes stay sharper than climate borders. Coastlines,
//! shores, rivers and lakes keep their exact inputs and stay crisp. This is
//! rendering only: the exported biome layer is unchanged.
use crate::biome::{BiomeInput, biome_color};
use crate::preset::Preset;
use crate::world::World;
use std::sync::LazyLock;

/// Standard deviation of the temperature offsets (`[0, 1]` scale).
const SIGMA_T: f32 = 0.02;
/// Standard deviation of the moisture offsets (`[-1, 1]` scale).
const SIGMA_M: f32 = 0.05;
/// Standard deviation of the highland line offsets (elevation units).
const SIGMA_H: f32 = 0.003;
/// Standard deviation of the volcanic zone offsets (`[0, 1]` scale).
const SIGMA_V: f32 = 0.004;
/// Offsets per pixel near a border. The same offsets serve every pixel: a
/// per-pixel rotation hid the finite count as grain, which zooming in shows.
const SAMPLES: usize = 128;
/// Probes, in standard deviations, telling a pixel far from every border.
const PROBE: f32 = 2.5;
/// [`unit_offsets`], computed once: they cost two thirds of a border pixel.
static OFFSETS: LazyLock<[[f32; 4]; SAMPLES]> = LazyLock::new(unit_offsets);

/// 4D offsets (temperature, moisture, highland, volcanic) drawn from a unit
/// Gaussian with a Halton sequence and Box-Muller; each point comes with its
/// opposite, so the offsets average to exactly zero and never bias colours.
fn unit_offsets() -> [[f32; 4]; SAMPLES] {
    fn halton(mut i: usize, base: usize) -> f32 {
        let (mut f, mut r) = (1.0f32, 0.0f32);
        while i > 0 {
            f /= base as f32;
            r += f * (i % base) as f32;
            i /= base;
        }
        r
    }
    let gauss = |u: f32, v: f32| {
        let r = (-2.0 * (1.0 - u).ln()).sqrt();
        let a = std::f32::consts::TAU * v;
        (r * a.cos(), r * a.sin())
    };
    let half = SAMPLES / 2;
    std::array::from_fn(|k| {
        let j = k % half + 1;
        let (a, b) = gauss(halton(j, 2), halton(j, 3));
        let (c, d) = gauss(halton(j, 5), halton(j, 7));
        let sign = if k < half { 1.0 } else { -1.0 };
        [sign * a, sign * b, sign * c, sign * d]
    })
}

/// The input nudged by an offset in standard deviations.
fn nudge(input: BiomeInput, o: [f32; 4]) -> BiomeInput {
    BiomeInput {
        temperature: (input.temperature + o[0] * SIGMA_T).clamp(0.0, 1.0),
        moisture: (input.moisture + o[1] * SIGMA_M).clamp(-1.0, 1.0),
        highland_offset: input.highland_offset + o[2] * SIGMA_H,
        volcanic_zone: (input.volcanic_zone + o[3] * SIGMA_V).clamp(0.0, 1.0),
        ..input
    }
}

/// Colour of a pixel with soft borders, as sRGB floats in `[0, 255]`.
/// `input` is the exact biome input of the pixel.
pub fn ecotone_colour(world: &World, input: BiomeInput) -> [f32; 3] {
    ecotone_colour_for(&world.params.planet, input)
}

/// Like [`ecotone_colour`] for any planet.
pub fn ecotone_colour_for(planet: &Preset, input: BiomeInput) -> [f32; 3] {
    let choose = |o: [f32; 4]| planet.remap(planet.rules.choose(nudge(input, o)));
    let centre = choose([0.0; 4]);
    // Far from every border (the common case), the colour is flat.
    let far = (0..8).all(|k| {
        let mut o = [0.0; 4];
        o[k / 2] = if k % 2 == 0 { PROBE } else { -PROBE };
        choose(o) == centre
    });
    if far {
        return biome_color(centre).map(f32::from);
    }
    let mut sum = [0.0f32; 3];
    for &o in OFFSETS.iter() {
        for (s, c) in sum.iter_mut().zip(biome_color(choose(o))) {
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
        for axis in 0..4 {
            let mean = offsets.iter().map(|o| o[axis]).sum::<f32>() / SAMPLES as f32;
            let var = offsets.iter().map(|o| o[axis] * o[axis]).sum::<f32>() / SAMPLES as f32;
            assert!(mean.abs() < 1e-5, "axis {axis}: mean {mean}");
            assert!((var - 1.0).abs() < 0.3, "axis {axis}: variance {var}");
        }
    }

    #[test]
    fn borders_blend_and_interiors_stay_flat() {
        let planet = Preset::load("terran").unwrap();
        let rules = &planet.rules;
        let input = |highland_offset: f32| BiomeInput {
            // Temperate forest, far from the climate thresholds.
            elevation: 0.05,
            moisture: 0.6,
            temperature: 0.42,
            volcanic_zone: 0.0,
            coast_distance_km: 1000.0,
            lake: false,
            river: false,
            highland_offset,
        };
        // Right on the highland line: mountain and forest colours mix.
        let edge = ecotone_colour_for(&planet, input(rules.highland - 0.05));
        let low = biome_color(planet.remap(rules.choose(input(rules.highland - 0.2))));
        let high = biome_color(planet.remap(rules.choose(input(rules.highland + 0.2))));
        assert_ne!(edge, low.map(f32::from));
        assert_ne!(edge, high.map(f32::from));
        // Far from it, the exact colour.
        assert_eq!(
            ecotone_colour_for(&planet, input(rules.highland - 0.2)),
            low.map(f32::from)
        );
    }
}
