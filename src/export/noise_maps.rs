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
use crate::generation::{Sample, Sampler};
use crate::world::World;
use image::{Rgb, RgbImage};

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

/// Colourize a signed value v ∈ [-1.0, 1.0] → jet(0.0 … 1.0).
fn diverge(v: f32) -> [u8; 3] {
    jet((v.clamp(-1.0, 1.0) + 1.0) * 0.5)
}

/// Colourize an unsigned value v ∈ [0.0, 1.0] → jet(0.0 … 1.0).
fn sequential(v: f32) -> [u8; 3] {
    jet(v.clamp(0.0, 1.0))
}

/// One exported layer: file stem, value extractor and colour ramp.
type Layer = (&'static str, fn(&Sample) -> f32, fn(f32) -> [u8; 3]);

const LAYERS: &[Layer] = &[
    ("noise_warp_x", |s| s.warp_x, diverge),
    ("noise_warp_y", |s| s.warp_y, diverge),
    ("noise_continent", |s| s.continent, diverge),
    ("noise_mountain", |s| s.mountain, sequential),
    ("noise_mountain_wt", |s| s.mountain_weight, sequential),
    ("noise_elevation", |s| s.elevation, diverge),
    ("noise_biome_elev", |s| s.biome_elevation, diverge),
    ("noise_moisture", |s| s.moisture, diverge),
    ("noise_temperature", |s| s.temperature, sequential),
    ("noise_volcanic_raw", |s| s.volcanic_raw, diverge),
    ("noise_volcanic_zone", |s| s.volcanic_zone, sequential),
];

// ── Public entry point ────────────────────────────────────────────────────────

/// Re-samples every intermediate noise layer of `world` with the same
/// [`Sampler`] used during generation and writes one false-colour PNG per
/// layer into `dir/`.
pub fn export_noise_maps(world: &World, dir: &str) -> image::ImageResult<()> {
    let sampler = Sampler::new(
        world.width,
        world.height,
        world.seed,
        world.sea_level,
        world.volcanic_intensity,
        world.circumference_km,
    );
    let (w, h) = (world.width as u32, world.height as u32);
    let mut images: Vec<RgbImage> = LAYERS.iter().map(|_| RgbImage::new(w, h)).collect();

    for q in 0..world.width {
        for r in 0..world.height {
            let sample = sampler.sample(q, r);
            for (img, (_, value, colorize)) in images.iter_mut().zip(LAYERS) {
                img.put_pixel(q as u32, r as u32, Rgb(colorize(value(&sample))));
            }
        }
    }

    for (img, (name, _, _)) in images.iter().zip(LAYERS) {
        img.save(format!("{dir}/{name}.png"))?;
    }
    Ok(())
}
