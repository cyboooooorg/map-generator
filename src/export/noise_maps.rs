//! Export every intermediate noise map as a false-colour PNG.
//!
//! The maps produced are:
//!
//! | File                    | Range    | Description                                         |
//! |-------------------------|----------|-----------------------------------------------------|
//! | noise_warp_x.png        | [-1, 1]  | Domain-warp field, X axis                           |
//! | noise_warp_y.png        | [-1, 1]  | Domain-warp field, Y axis                           |
//! | noise_continent.png     | [-1, 1]  | Low-freq FBM continent shape                        |
//! | noise_mountain.png      | [ 0, 1]  | Ridged noise (mountain peaks)                       |
//! | noise_mountain_wt.png   | [ 0, 1]  | Mountain blend weight (based on continent height)   |
//! | noise_elevation.png     | [-1, 1]  | Final elevation = continent + mountain×weight×blend |
//! | noise_biome_elev.png    | [-1, 1]  | Elevation shifted by sea_level (what biomes see)    |
//! | noise_moisture.png      | [-1, 1]  | Moisture FBM                                        |
//! | noise_temperature.png   | [ 0, 1]  | Temperature (latitude gradient + land altitude cooling) |
//! | noise_volcanic_raw.png  | [-1, 1]  | Raw volcanic-zone FBM                               |
//! | noise_volcanic_zone.png | [ 0, 1]  | Processed volcanic zone (threshold applied)         |
//!
//! Colour encoding
//! ───────────────
//! All maps share the same "jet" ramp:
//!   blue (low) → cyan → green → yellow → red (high)
//!
//! Signed maps are linearly rescaled so that 0.0 → green, -1.0 → blue, +1.0 → red.
//! Unsigned maps are rescaled so that 0.0 → blue and 1.0 → red.
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
