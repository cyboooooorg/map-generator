//! Export backends — PNG, SVG, JSON and diagnostic noise maps.
pub mod json;
pub mod noise_maps;
pub mod png;
pub mod svg;

pub use json::export_json;
pub use noise_maps::export_noise_maps;
pub use png::{export_legend_png, export_png};
pub use svg::export_svg;

use crate::world::{World, biome_color};

/// Contour lines are drawn whenever a tile and a neighbour straddle one of these levels.
const CONTOUR_LEVELS: &[f32] = &[-0.45, -0.15, 0.0, 0.15, 0.30, 0.45, 0.60, 0.75, 0.90];
/// Fraction to darken a pixel by when it sits on a contour line (0.0 = no change, 1.0 = black).
const CONTOUR_DARKNESS: f32 = 0.40;

/// Dotted latitude reference lines: (latitude in degrees, RGB colour).
pub const REFERENCE_LINES: &[(f64, [u8; 3])] = &[
    (0.0, [220, 50, 50]),   // Equator — red
    (23.5, [220, 150, 0]),  // Tropic of Cancer — amber
    (-23.5, [220, 150, 0]), // Tropic of Capricorn — amber
    (66.5, [0, 200, 240]),  // Arctic Circle — cyan
    (-66.5, [0, 200, 240]), // Antarctic Circle — cyan
];
/// Dash pattern of the reference lines, in pixels.
pub const DASH_ON: u32 = 6;
pub const DASH_OFF: u32 = 4;

/// Row of a latitude in the equirectangular projection used by the map.
pub fn latitude_row(height: u32, latitude_deg: f64) -> f64 {
    height as f64 * (0.5 + latitude_deg / 180.0)
}

/// Computes the final colour of every pixel (biome colour darkened on contour
/// lines), in row-major order: index = r * width + q.
pub fn render_pixels(world: &World) -> Vec<[u8; 3]> {
    let (w, h) = (world.width as usize, world.height as usize);

    // Tiles are stored column-major (q outer, r inner) by generate_world;
    // index them explicitly so the renderer does not depend on that order.
    let mut elevation = vec![0.0f32; w * h];
    for t in &world.tiles {
        elevation[t.r as usize * w + t.q as usize] = t.elevation;
    }
    let crosses_contour = |a: f32, b: f32| CONTOUR_LEVELS.iter().any(|&lvl| (a < lvl) != (b < lvl));

    let mut pixels = vec![[0u8; 3]; w * h];
    for t in &world.tiles {
        let (q, r) = (t.q as usize, t.r as usize);
        // 4-connected neighbours that exist inside the map.
        let neighbours = [
            (q > 0).then(|| elevation[r * w + q - 1]),
            (q + 1 < w).then(|| elevation[r * w + q + 1]),
            (r > 0).then(|| elevation[(r - 1) * w + q]),
            (r + 1 < h).then(|| elevation[(r + 1) * w + q]),
        ];
        let mut color = biome_color(t.biome);
        if neighbours
            .into_iter()
            .flatten()
            .any(|n| crosses_contour(t.elevation, n))
        {
            color = color.map(|c| (c as f32 * (1.0 - CONTOUR_DARKNESS)) as u8);
        }
        pixels[r * w + q] = color;
    }
    pixels
}
