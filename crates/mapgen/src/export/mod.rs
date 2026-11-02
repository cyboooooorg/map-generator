//! Export backends — PNG, SVG, JSON and diagnostic noise maps.
pub mod ecotone;
pub mod noise_maps;
pub mod png;
pub mod raw_data;
pub mod svg;

pub use noise_maps::export_noise_maps;
pub use png::{export_legend_png, export_png};
pub use raw_data::export_raw_data;
pub use svg::export_svg;

use crate::generation::Generated;
use crate::geo::Grid;
use crate::rivers;
use crate::world::{Biome, World, biome_color};
use ecotone::ecotone_colour;
use rayon::prelude::*;
use std::path::Path;

/// Which optional outputs [`export_world`] writes. The maps (`world.png`,
/// `world.svg`, `legend.png`) are always written.
#[derive(Clone, Copy, Debug)]
pub struct ExportOptions {
    /// `raw_data/`: world.json and the layer PNGs.
    pub raw_data: bool,
    /// `noise_maps/`: one diagnostic PNG per intermediate layer.
    pub noise_maps: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            raw_data: true,
            noise_maps: true,
        }
    }
}

/// Name of a world's output directory: `<planet>-<seed>`.
pub fn world_dir_name(world: &World) -> String {
    format!("{}-{}", world.params.planet.name, world.params.seed)
}

/// Writes every output of a generated world into `dir` (created if needed).
/// The exports are independent and dominated by PNG encoding, so they run in
/// parallel.
pub fn export_world(
    generated: &Generated,
    dir: &Path,
    options: ExportOptions,
) -> Result<(), String> {
    let world = &generated.world;
    let path = |sub: &str| dir.join(sub).to_string_lossy().into_owned();
    let io = |e: std::io::Error| e.to_string();
    std::fs::create_dir_all(dir).map_err(io)?;
    if options.raw_data {
        std::fs::create_dir_all(dir.join("raw_data")).map_err(io)?;
    }
    if options.noise_maps {
        std::fs::create_dir_all(dir.join("noise_maps")).map_err(io)?;
    }

    type Export<'a> = Box<dyn Fn() -> Result<(), String> + Send + Sync + 'a>;
    let mut exports: Vec<Export> = vec![
        Box::new(|| export_png(world, &path("world.png")).map_err(|e| e.to_string())),
        Box::new(|| export_svg(world, &path("world.svg")).map_err(|e| e.to_string())),
        Box::new(|| export_legend_png(world, &path("legend.png")).map_err(|e| e.to_string())),
    ];
    if options.raw_data {
        exports.push(Box::new(|| {
            export_raw_data(world, &dir.join("raw_data")).map_err(|e| e.to_string())
        }));
    }
    if options.noise_maps {
        exports.push(Box::new(|| {
            export_noise_maps(
                &generated.debug,
                world.width,
                world.height,
                &path("noise_maps"),
            )
            .map_err(|e| e.to_string())
        }));
    }
    exports.par_iter().try_for_each(|export| export())
}

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
pub(crate) const RELIEF_KM: f32 = 1500.0;
/// Light direction (east, north, up), from the north-west at 45° altitude.
pub(crate) const LIGHT: [f32; 3] = [-0.5, 0.5, std::f32::consts::FRAC_1_SQRT_2];

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

/// How a map is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Hillshaded land and soft biome borders (PNG).
    Detailed,
    /// Flat biome colours, which keep the SVG run-length encoding compact.
    Flat,
}

/// Computes the final colour of every pixel, in the same row-major order as
/// the world layers. Contour lines are drawn under water only, as bathymetry:
/// on land the hillshade shows the relief, and contours would draw rings.
pub fn render_pixels(world: &World, style: Style) -> Vec<[u8; 3]> {
    let (w, h) = (world.width, world.height);
    let grid = world.params.grid();
    let planet = &world.params.planet;
    let elev = |x: u32, y: u32| world.elevation[world.index(x, y)];
    let crosses_contour = |a: f32, b: f32| CONTOUR_LEVELS.iter().any(|&lvl| (a < lvl) != (b < lvl));
    let coverage = rivers::coverage(&world.rivers, w, h, world.params.circumference_km as f64);
    let river_biome = planet.remap(Biome::River);
    let river_colour = biome_color(river_biome).map(f32::from);

    (0..h)
        .into_par_iter()
        .flat_map_iter(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let i = world.index(x, y);
            let e = elev(x, y);
            let biome = world.biomes[i];
            let on_river = biome == river_biome && coverage[i] >= rivers::RIVER_COVERAGE;
            if biome.is_water() && !on_river {
                let mut color = biome_color(biome);
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
                return color;
            }
            // The land under a river: the biome the pixel has without it.
            let input = world.biome_input(i);
            let land = planet.remap(planet.rules.choose(input));
            if style == Style::Flat {
                // Rivers are drawn as vector paths over the flat SVG map.
                return biome_color(if on_river { land } else { biome });
            }
            // Lakes are not recomputed without their flag: a pixel whose
            // biome differs from its recomputed one keeps a flat colour.
            let mut color = if land == biome || on_river {
                ecotone_colour(world, i, input)
            } else {
                biome_color(biome).map(f32::from)
            };
            let f = hillshade(world, &grid, x, y);
            color = color.map(|c| c * f);
            // Anti-aliased river on top, except where it is frozen.
            let c = coverage[i];
            if c > 0.0 && world.temperature[i] >= planet.rules.polar {
                for (channel, river) in color.iter_mut().zip(river_colour) {
                    *channel = *channel * (1.0 - c) + river * c;
                }
            }
            color.map(|c| c.clamp(0.0, 255.0) as u8)
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
