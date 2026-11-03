//! Export backends — PNG, SVG, JSON and diagnostic noise maps.
pub(crate) mod colour;
pub mod ecotone;
pub mod noise_maps;
pub mod png;
pub mod raw_data;
pub mod svg;
pub mod variation;

pub use noise_maps::export_noise_maps;
pub use png::{export_legend_png, export_png};
pub use raw_data::export_raw_data;
pub use svg::export_svg;

use crate::generation::Generated;
use crate::geo::Grid;
use crate::rivers;
use crate::world::{Biome, World, WorldParams, biome_color, surface_color};
use colour::{linear_of, mix, to_linear, to_srgb};
use ecotone::ecotone_colour;
use rayon::prelude::*;
use std::path::Path;
use std::sync::LazyLock;
use variation::SurfaceVariation;

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
pub fn world_dir_name(params: &WorldParams) -> String {
    format!("{}-{}", params.planet.name, params.seed)
}

/// Writes every output of a generated world into `dir` (created if needed).
/// The exports are independent and dominated by PNG encoding, so they run in
/// parallel.
pub fn export_world(
    generated: Generated,
    dir: &Path,
    options: ExportOptions,
) -> Result<(), String> {
    let Generated {
        world,
        debug,
        river_coverage,
    } = generated;
    let world = &world;
    let path = |sub: &str| dir.join(sub).to_string_lossy().into_owned();
    let io = |e: std::io::Error| e.to_string();
    std::fs::create_dir_all(dir).map_err(io)?;
    if options.raw_data {
        std::fs::create_dir_all(dir.join("raw_data")).map_err(io)?;
    }
    if options.noise_maps {
        std::fs::create_dir_all(dir.join("noise_maps")).map_err(io)?;
    }

    let layers = MapLayers::with_coverage(world, river_coverage);
    let layers = &layers;
    // The detailed render, shared by world.png (which adds the reference
    // lines) and raw_data/surface.png (without them, for the viewer).
    let surface = render_pixels(world, layers, Style::Detailed);
    let surface = &surface;
    type Export<'a> = Box<dyn Fn() -> Result<(), String> + Send + Sync + 'a>;
    let mut exports: Vec<Export> = vec![
        Box::new(|| {
            let mut map = surface.clone();
            draw_contours(world, layers, &mut map);
            export_png(world, map, &path("world.png")).map_err(|e| e.to_string())
        }),
        Box::new(|| export_svg(world, layers, &path("world.svg")).map_err(|e| e.to_string())),
        Box::new(|| export_legend_png(world, &path("legend.png")).map_err(|e| e.to_string())),
    ];
    if options.raw_data {
        exports.push(Box::new(|| {
            export_raw_data(world, &dir.join("raw_data"), Some(surface)).map_err(|e| e.to_string())
        }));
    }
    if options.noise_maps {
        exports.push(Box::new(|| {
            export_noise_maps(&debug, world.width, world.height, &path("noise_maps"))
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
    shade(lit / LIGHT[2])
}

/// Brightness of a land pixel from its lighting relative to flat ground (1):
/// a soft curve towards 0.6 and 1.4, so steep slopes stay distinct instead
/// of all reaching a hard limit.
pub(crate) fn shade(relative: f32) -> f32 {
    1.0 + SHADE_RANGE * ((relative - 1.0) / SHADE_RANGE).tanh()
}

/// Largest brightening or darkening of the hillshade.
const SHADE_RANGE: f32 = 0.4;

/// Colour of a water pixel `depth` elevation units below its surface, in
/// linear light.
///
/// Clear liquids (water, acid) follow Beer-Lambert absorption: the light
/// reflected by the bed fades with depth, faster in the channels the liquid
/// absorbs most, towards the colour of deep water. Shelves turn turquoise
/// (red is absorbed first), and the colour depends on the depth alone, so
/// there is no step where Ocean becomes Deep Ocean. Lakes, measured from
/// their own level, are shallow: their depth counts [`LAKE_DEPTH_SCALE`]
/// times. Magma darkens into a cooled crust along its shore and ice whitens
/// into fast ice; rivers stay flat.
pub(crate) fn water_colour(biome: Biome, depth: f32) -> [f32; 3] {
    let colour = surface_color(biome).map(f32::from);
    if matches!(biome, Biome::River | Biome::AcidRiver) {
        return to_linear(colour);
    }
    let depth = depth.max(0.0);
    if let Some(liquid) = Liquid::of(biome) {
        let depth = if matches!(biome, Biome::Lake | Biome::CausticLake) {
            depth * LAKE_DEPTH_SCALE
        } else {
            depth
        };
        return std::array::from_fn(|k| {
            let seen = BED_VISIBILITY * (-liquid.absorption[k] * depth).exp();
            liquid.deep[k] + (liquid.bed[k] - liquid.deep[k]) * seen
        });
    }
    to_linear(shore_tint(biome, colour, depth))
}

/// Colour of a water pixel on the flat SVG map: its classification colour,
/// lightened towards white along the shore (darkened into crust for magma);
/// rivers stay flat.
fn flat_water_colour(biome: Biome, depth: f32) -> [f32; 3] {
    let colour = biome_color(biome).map(f32::from);
    if matches!(biome, Biome::River | Biome::AcidRiver) {
        return colour;
    }
    shore_tint(biome, colour, depth)
}

/// `colour` tinted along the shore over the last [`SHALLOW_DEPTH`]: towards
/// white (fast ice, shallow water), or towards a cooled crust for magma.
fn shore_tint(biome: Biome, colour: [f32; 3], depth: f32) -> [f32; 3] {
    let shallow = 1.0 - (depth.max(0.0) / SHALLOW_DEPTH).clamp(0.0, 1.0);
    let t = SHALLOW_TINT * shallow * shallow;
    let target = if biome == Biome::MagmaSea {
        MAGMA_CRUST
    } else {
        [255.0; 3]
    };
    std::array::from_fn(|k| colour[k] + (target[k] - colour[k]) * t)
}

/// Optical model of a clear liquid (see [`water_colour`]).
struct Liquid {
    /// Colour of deep liquid, where no light comes back from the bed,
    /// linear.
    deep: [f32; 3],
    /// Colour of the bed under a thin layer, linear.
    bed: [f32; 3],
    /// Absorption per elevation unit of depth, per linear channel.
    absorption: [f32; 3],
}

/// Water absorbs red first, then green: sandy shallows, turquoise shelves,
/// dark blue beyond the shelf break (0.30 deep).
static WATER: LazyLock<Liquid> = LazyLock::new(|| Liquid {
    deep: linear_of(surface_color(Biome::DeepOcean)),
    bed: linear_of(surface_color(Biome::Beach)),
    absorption: [30.0, 11.0, 8.0],
});

/// Acid absorbs blue first: yellow-green shallows.
static ACID: LazyLock<Liquid> = LazyLock::new(|| Liquid {
    deep: linear_of(surface_color(Biome::DeepAcidSea)),
    bed: linear_of(surface_color(Biome::AcidShore)),
    absorption: [24.0, 18.0, 40.0],
});

impl Liquid {
    fn of(biome: Biome) -> Option<&'static Self> {
        match biome {
            Biome::DeepOcean | Biome::Ocean | Biome::Lake => Some(&WATER),
            Biome::DeepAcidSea | Biome::AcidSea | Biome::CausticLake => Some(&ACID),
            _ => None,
        }
    }
}

/// Depth of every water pixel below its surface, in elevation units (0 on
/// land and rivers): below the waterline for seas; below the lake's level,
/// its highest pixel, for lakes, which lie above the sea.
pub fn water_depths(world: &World) -> Vec<f32> {
    water_depths_of(&world.params, |i| world.elevation[i], &world.biomes)
}

/// [`water_depths`] from the elevation of each pixel (`elevation(i)`,
/// row-major) and the biomes.
pub(crate) fn water_depths_of(
    params: &WorldParams,
    elevation: impl Fn(usize) -> f32 + Sync,
    biomes: &[Biome],
) -> Vec<f32> {
    let (w, h) = (params.width as usize, params.height as usize);
    let planet = &params.planet;
    let waterline = params.sea_level + planet.rules.waterline;
    let river = planet.remap(Biome::River);
    let lake = |i: usize| biomes[i].is_water() && biomes[i] != river && elevation(i) >= waterline;
    let mut depth: Vec<f32> = (0..w * h)
        .into_par_iter()
        .map(|i| {
            if biomes[i].is_water() && elevation(i) < waterline {
                waterline - elevation(i)
            } else {
                0.0
            }
        })
        .collect();
    let mut seen = vec![false; w * h];
    let mut members = Vec::new();
    for start in 0..w * h {
        if seen[start] || !lake(start) {
            continue;
        }
        members.clear();
        let mut stack = vec![start];
        seen[start] = true;
        let mut surface = f32::MIN;
        while let Some(i) = stack.pop() {
            members.push(i);
            surface = surface.max(elevation(i));
            let (x, y) = (i % w, i / w);
            let neighbours = [
                Some(y * w + (x + 1) % w),
                Some(y * w + (x + w - 1) % w),
                (y > 0).then(|| i - w),
                (y + 1 < h).then(|| i + w),
            ];
            for j in neighbours.into_iter().flatten() {
                if !seen[j] && lake(j) {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        for &i in &members {
            depth[i] = surface - elevation(i);
        }
    }
    depth
}

/// Depth (elevation units) over which ice whitens and magma crusts towards
/// its shore.
const SHALLOW_DEPTH: f32 = 0.05;
/// Colour of the cooled crust towards which magma seas darken at the shore.
const MAGMA_CRUST: [f32; 3] = [60.0, 20.0, 10.0];
/// Whitening of ice at the shore line.
const SHALLOW_TINT: f32 = 0.25;
/// Share of the bed's colour seen through a film of clear liquid: even the
/// shallowest water tints its bed.
const BED_VISIBILITY: f32 = 0.55;
/// Factor on lake depths: lakes are far shallower than seas, and would show
/// their bed everywhere.
const LAKE_DEPTH_SCALE: f32 = 4.0;

/// `colour` covered by a river over a fraction `coverage` of the pixel, both
/// sRGB in `[0, 255]`; the mix is in linear light, so anti-aliased banks do
/// not darken.
/// `river` is in linear light.
pub(crate) fn blend_river(colour: [f32; 3], river: [f32; 3], coverage: f32) -> [f32; 3] {
    to_srgb(mix(to_linear(colour), river, coverage))
}

/// How a map is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Hillshaded land and soft biome borders (PNG).
    Detailed,
    /// Flat biome colours, which keep the SVG run-length encoding compact.
    Flat,
}

/// Map-sized layers both maps (PNG and SVG) are drawn from: computed once
/// per export and shared, instead of once per map.
pub struct MapLayers {
    /// Anti-aliased river coverage (see [`rivers::coverage`]).
    coverage: Vec<f32>,
    /// See [`water_depths`].
    depths: Vec<f32>,
}

impl MapLayers {
    pub fn new(world: &World) -> Self {
        let (coverage, depths) = rayon::join(
            || {
                rivers::coverage(
                    &world.rivers,
                    world.width,
                    world.height,
                    world.params.circumference_km as f64,
                )
            },
            || water_depths(world),
        );
        Self { coverage, depths }
    }

    /// Like [`MapLayers::new`], reusing the river coverage the generation
    /// computed ([`Generated::river_coverage`]).
    pub fn with_coverage(world: &World, coverage: Vec<f32>) -> Self {
        Self {
            coverage,
            depths: water_depths(world),
        }
    }
}

/// Darkens the pixels on a contour line, under water only, as bathymetry:
/// on land the hillshade shows the relief, and contours would draw rings.
/// `pixels` come from [`render_pixels`]; the viewer draws its own contours
/// over `raw_data/surface.png`, which has none.
pub fn draw_contours(world: &World, layers: &MapLayers, pixels: &mut [[u8; 3]]) {
    let (w, h) = (world.width, world.height);
    let river_biome = world.params.planet.remap(Biome::River);
    let elev = |x: u32, y: u32| world.elevation[world.index(x, y)];
    let crosses_contour = |a: f32, b: f32| CONTOUR_LEVELS.iter().any(|&lvl| (a < lvl) != (b < lvl));
    pixels
        .par_chunks_mut(w as usize)
        .enumerate()
        .for_each(|(y, row)| {
            let y = y as u32;
            for (x, pixel) in row.iter_mut().enumerate() {
                let x = x as u32;
                let i = world.index(x, y);
                let biome = world.biomes[i];
                let on_river = biome == river_biome && layers.coverage[i] >= rivers::RIVER_COVERAGE;
                if !biome.is_water() || on_river {
                    continue;
                }
                let e = elev(x, y);
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
                    *pixel = pixel.map(|c| (c as f32 * (1.0 - CONTOUR_DARKNESS)) as u8);
                }
            }
        });
}

/// Computes the final colour of every pixel, in the same row-major order as
/// the world layers, without contour lines (see [`draw_contours`]).
pub fn render_pixels(world: &World, layers: &MapLayers, style: Style) -> Vec<[u8; 3]> {
    let (w, h) = (world.width, world.height);
    let grid = world.params.grid();
    let planet = &world.params.planet;
    let (coverage, depths) = (&layers.coverage, &layers.depths);
    let river_biome = planet.remap(Biome::River);
    let river_colour = linear_of(surface_color(river_biome));
    let variation = SurfaceVariation::new(world.params.seed, world.params.circumference_km as f64);
    let (lon_step, lat_step) = (
        std::f64::consts::TAU / w as f64,
        std::f64::consts::PI / h as f64,
    );

    (0..h)
        .into_par_iter()
        .flat_map_iter(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let i = world.index(x, y);
            let biome = world.biomes[i];
            let on_river = biome == river_biome && coverage[i] >= rivers::RIVER_COVERAGE;
            let lon = (x as f64 + 0.5) * lon_step;
            let lat = std::f64::consts::FRAC_PI_2 - (y as f64 + 0.5) * lat_step;
            if biome.is_water() && !on_river {
                // The flat SVG keeps the classification colours, with no
                // per-pixel variation, so its run-length encoding stays
                // compact.
                let color = match style {
                    Style::Detailed => {
                        to_srgb(variation.sea(biome, water_colour(biome, depths[i]), lon, lat))
                    }
                    Style::Flat => flat_water_colour(biome, depths[i]),
                };
                return color.map(|c| c.clamp(0.0, 255.0) as u8);
            }
            // The land under a river: the biome the pixel has without it.
            let input = world.biome_input(i);
            let dry = planet.rules.choose(input);
            let land = planet.remap(dry);
            if style == Style::Flat {
                // Rivers are drawn as vector paths over the flat SVG map.
                return biome_color(if on_river { land } else { biome });
            }
            // Lakes are not recomputed without their flag: a pixel whose
            // biome differs from its recomputed one keeps a flat colour.
            let mut color = if land == biome || on_river {
                ecotone_colour(world, input)
            } else {
                surface_color(biome).map(f32::from)
            };
            // Lava varies too: switching the variation by the sharp lava
            // class would draw a step through the soft ecotone around it.
            color = variation.apply(color, lon, lat);
            let mut f = hillshade(world, &grid, x, y);
            // Lava is fluid: its relief barely shows.
            if dry.is_lava() {
                f = 1.0 + (f - 1.0) * crate::detail::LAVA_SHADE;
            }
            color = color.map(|c| c * f);
            // Anti-aliased river on top, except where it is frozen or under
            // lava.
            let c = coverage[i];
            if c > 0.0 && world.temperature[i] >= planet.rules.polar && !dry.is_lava() {
                color = blend_river(color, river_colour, c);
            }
            color.map(|c| c.clamp(0.0, 255.0) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Biome, latitude_row, water_colour};
    use crate::export::colour::to_srgb;

    #[test]
    fn water_darkens_with_depth_without_a_deep_ocean_step() {
        let luma = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let mut previous = f32::MAX;
        for k in 0..40 {
            let l = luma(to_srgb(water_colour(Biome::Ocean, k as f32 * 0.02)));
            assert!(l <= previous, "lighter at depth {}", k as f32 * 0.02);
            previous = l;
        }
        // Same depth, either side of the Deep Ocean threshold: same colour.
        assert_eq!(
            water_colour(Biome::Ocean, 0.3),
            water_colour(Biome::DeepOcean, 0.3)
        );
        // Shallow water shows its bed: red is absorbed first.
        let shelf = to_srgb(water_colour(Biome::Ocean, 0.05));
        let deep = to_srgb(water_colour(Biome::Ocean, 0.6));
        assert!(shelf[1] > deep[1] && shelf[2] > deep[2]);
        assert!(shelf[0] < shelf[2]);
    }

    #[test]
    fn north_is_at_the_top() {
        assert_eq!(latitude_row(1080, 90.0), 0.0);
        assert_eq!(latitude_row(1080, 0.0), 540.0);
        assert_eq!(latitude_row(1080, -90.0), 1080.0);
        // Tropic of Cancer (northern hemisphere) sits above the equator.
        assert!(latitude_row(1080, 23.5) < latitude_row(1080, 0.0));
    }
}
