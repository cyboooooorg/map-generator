//! Export backends — PNG, SVG, JSON and diagnostic noise maps.
pub mod noise_maps;
pub mod png;
pub mod raw_data;
pub mod svg;

pub use noise_maps::export_noise_maps;
pub use png::{export_legend_png, export_png};
pub use raw_data::export_raw_data;
pub use svg::export_svg;

use crate::geo::Grid;
use crate::world::{World, biome_color};
use rayon::prelude::*;

/// Contour lines are drawn whenever a tile and a neighbour straddle one of these levels.
const CONTOUR_LEVELS: &[f32] = &[-0.45, -0.15, 0.0, 0.15, 0.30, 0.45, 0.60, 0.75, 0.90];
/// Fraction to darken a pixel by when it sits on a contour line (0.0 = no change, 1.0 = black).
const CONTOUR_DARKNESS: f32 = 0.40;

/// Dotted latitude reference lines as (latitude in degrees, RGB colour):
/// the equator, the tropics at ±tilt and the polar circles at ±(90° − tilt).
pub fn reference_lines(axial_tilt_deg: f32) -> [(f64, [u8; 3]); 5] {
    let tilt = axial_tilt_deg as f64;
    [
        (0.0, [220, 50, 50]),            // Equator — red
        (tilt, [220, 150, 0]),           // Tropic of Cancer — amber
        (-tilt, [220, 150, 0]),          // Tropic of Capricorn — amber
        (90.0 - tilt, [0, 200, 240]),    // Arctic Circle — cyan
        (-(90.0 - tilt), [0, 200, 240]), // Antarctic Circle — cyan
    ]
}
/// Dash pattern of the reference lines, in pixels.
pub const DASH_ON: u32 = 6;
pub const DASH_OFF: u32 = 4;

/// Row of a latitude in the equirectangular projection used by the map:
/// north pole (+90°) at row 0, south pole (-90°) at the last row.
pub fn latitude_row(height: u32, latitude_deg: f64) -> f64 {
    height as f64 * (0.5 - latitude_deg / 180.0)
}

/// Vertical exaggeration of the hillshade: one elevation unit is treated as
/// `RELIEF_KM` kilometres. Planet-scale slopes are tiny, so this is far above
/// real relief on purpose.
const RELIEF_KM: f32 = 1500.0;
/// Light direction (east, north, up), from the north-west at 45° altitude.
const LIGHT: [f32; 3] = [-0.5, 0.5, std::f32::consts::FRAC_1_SQRT_2];

/// Brightness factor of a land pixel lit from the north-west: 1.0 on flat
/// ground, darker on slopes facing south-east, brighter facing north-west.
fn hillshade(world: &World, grid: &Grid, x: u32, y: u32) -> f32 {
    let (w, h) = (world.width, world.height);
    let elev = |x: u32, y: u32| world.elevation[world.index(x, y)] * RELIEF_KM;
    // Central differences; longitude wraps, latitude clamps at the poles.
    let (west, east) = ((x + w - 1) % w, (x + 1) % w);
    let (north, south) = (y.saturating_sub(1), (y + 1).min(h - 1));
    let dz_east = (elev(east, y) - elev(west, y)) / (2.0 * grid.dx_km(y));
    let dz_north =
        (elev(x, north) - elev(x, south)) / ((south - north).max(1) as f32 * grid.dy_km());
    let normal = [-dz_east, -dz_north, 1.0];
    let len = normal.iter().map(|c| c * c).sum::<f32>().sqrt();
    let lit: f32 = normal.iter().zip(LIGHT).map(|(n, l)| n * l).sum::<f32>() / len;
    (lit / LIGHT[2]).clamp(0.55, 1.3)
}

/// Computes the final colour of every pixel (biome colour darkened on contour
/// lines), in the same row-major order as the world layers. With `relief`,
/// land is also hillshaded.
pub fn render_pixels(world: &World, relief: bool) -> Vec<[u8; 3]> {
    let (w, h) = (world.width, world.height);
    let grid = world.params.grid();
    let elev = |x: u32, y: u32| world.elevation[world.index(x, y)];
    let crosses_contour = |a: f32, b: f32| CONTOUR_LEVELS.iter().any(|&lvl| (a < lvl) != (b < lvl));

    (0..h)
        .into_par_iter()
        .flat_map_iter(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let e = elev(x, y);
            let biome = world.biomes[world.index(x, y)];
            let mut color = biome_color(biome);
            if relief && !biome.is_water() {
                let f = hillshade(world, &grid, x, y);
                color = color.map(|c| (c as f32 * f).min(255.0) as u8);
            }
            // 4-connected neighbours that exist inside the map.
            let neighbours = [
                (x > 0).then(|| elev(x - 1, y)),
                (x + 1 < w).then(|| elev(x + 1, y)),
                (y > 0).then(|| elev(x, y - 1)),
                (y + 1 < h).then(|| elev(x, y + 1)),
            ];
            if neighbours
                .into_iter()
                .flatten()
                .any(|n| crosses_contour(e, n))
            {
                color = color.map(|c| (c as f32 * (1.0 - CONTOUR_DARKNESS)) as u8);
            }
            color
        })
        .collect()
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
