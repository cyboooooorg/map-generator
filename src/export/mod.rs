//! Export backends — PNG, SVG, JSON and diagnostic noise maps.
pub mod noise_maps;
pub mod png;
pub mod raw_data;
pub mod svg;

pub use noise_maps::export_noise_maps;
pub use png::{export_legend_png, export_png};
pub use raw_data::export_raw_data;
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

/// Row of a latitude in the equirectangular projection used by the map:
/// north pole (+90°) at row 0, south pole (-90°) at the last row.
pub fn latitude_row(height: u32, latitude_deg: f64) -> f64 {
    height as f64 * (0.5 - latitude_deg / 180.0)
}

/// Computes the final colour of every pixel (biome colour darkened on contour
/// lines), in the same row-major order as the world layers.
pub fn render_pixels(world: &World) -> Vec<[u8; 3]> {
    let (w, h) = (world.width, world.height);
    let elev = |x: u32, y: u32| world.elevation[world.index(x, y)];
    let crosses_contour = |a: f32, b: f32| CONTOUR_LEVELS.iter().any(|&lvl| (a < lvl) != (b < lvl));

    let mut pixels = Vec::with_capacity(world.biomes.len());
    for y in 0..h {
        for x in 0..w {
            let e = elev(x, y);
            // 4-connected neighbours that exist inside the map.
            let neighbours = [
                (x > 0).then(|| elev(x - 1, y)),
                (x + 1 < w).then(|| elev(x + 1, y)),
                (y > 0).then(|| elev(x, y - 1)),
                (y + 1 < h).then(|| elev(x, y + 1)),
            ];
            let mut color = biome_color(world.biomes[world.index(x, y)]);
            if neighbours
                .into_iter()
                .flatten()
                .any(|n| crosses_contour(e, n))
            {
                color = color.map(|c| (c as f32 * (1.0 - CONTOUR_DARKNESS)) as u8);
            }
            pixels.push(color);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::latitude_row;

    #[test]
    fn north_is_at_the_top() {
        assert_eq!(latitude_row(1080, 90.0), 0.0);
        assert_eq!(latitude_row(1080, 0.0), 540.0);
        assert_eq!(latitude_row(1080, -90.0), 1080.0);
        // Tropic of Cancer (northern hemisphere) sits above the equator.
        assert!(latitude_row(1080, 23.5) < latitude_row(1080, 0.0));
    }
}
