//! Export of the diagnostic maps: one false-colour PNG per intermediate
//! generation layer (see the README for the list).
//!
//! All maps share the same "jet" ramp: blue (low) → cyan → green → yellow →
//! red (high). Signed layers are rescaled so that -1.0 → blue, 0.0 → green and
//! +1.0 → red; unsigned layers so that 0.0 → blue and 1.0 → red.
use crate::generation::{DebugLayer, Ramp};
use image::{ImageResult, Rgb, RgbImage};
use rayon::prelude::*;

// ── Colour map ────────────────────────────────────────────────────────────────

/// "Jet" ramp: blue → cyan → green → yellow → red.
/// `t` ∈ [0.0, 1.0].
fn jet(t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    // Piecewise linear hat functions shifted to R, G, B channels.
    let r = (1.5 - (4.0 * t - 3.0).abs()).clamp(0.0, 1.0);
    let g = (1.5 - (4.0 * t - 2.0).abs()).clamp(0.0, 1.0);
    let b = (1.5 - (4.0 * t - 1.0).abs()).clamp(0.0, 1.0);
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

fn colorize(ramp: Ramp, v: f32) -> [u8; 3] {
    match ramp {
        // [-1, 1] → [0, 1]: 0.0 is green.
        Ramp::Signed => jet((v.clamp(-1.0, 1.0) + 1.0) * 0.5),
        Ramp::Unsigned => jet(v),
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Writes one false-colour PNG per debug layer into `dir/`.
pub fn export_noise_maps(
    layers: &[DebugLayer],
    width: u32,
    height: u32,
    dir: &str,
) -> ImageResult<()> {
    layers.par_iter().try_for_each(|layer| {
        let img = RgbImage::from_fn(width, height, |x, y| {
            Rgb(colorize(layer.ramp, layer.values[(y * width + x) as usize]))
        });
        img.save(format!("{dir}/{}.png", layer.name))
    })
}
