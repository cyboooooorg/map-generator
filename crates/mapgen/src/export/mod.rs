//! Export backends — PNG, SVG, JSON and diagnostic noise maps.
pub(crate) mod colour;
pub mod ecotone;
pub mod noise_maps;
pub mod png;
pub mod raw_data;
pub mod relief;
pub mod svg;
pub mod thumbnail;
pub mod variation;

pub use noise_maps::NoiseMapWriter;
pub use png::{export_legend_png, export_png, world_image};
pub use raw_data::export_raw_data;
pub use svg::{export_svg, svg_bytes};

use crate::generation::Generated;
use crate::rivers;
use crate::world::{Biome, World, WorldParams, biome_color, surface_color};
use colour::{linear_of, mix, to_linear, to_srgb};
use ecotone::ecotone_colour_around;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use variation::SurfaceVariation;

/// The outputs a body is generated with only on request: they take disk
/// and time and the app does not read them (see
/// `docs/analysis/15-storage.md`, decision 5). `world.svg` can be drawn
/// again at any time from the stored body (see `system::body_svg`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Diagnostics {
    /// `world.svg`, the vector image of the map.
    pub svg: bool,
    /// `noise_maps/`, an image of every noise layer, written during the
    /// generation by a [`NoiseMapWriter`].
    pub noise_maps: bool,
}

impl Diagnostics {
    /// Both outputs.
    pub fn all() -> Self {
        Self {
            svg: true,
            noise_maps: true,
        }
    }
}

/// Which optional outputs [`export_world`] writes, and where. `world.png`
/// and `legend.png` are always written; `world.svg` only with
/// [`ExportOptions::svg`]. The diagnostic maps of `noise_maps/` are written
/// during the generation, by a [`NoiseMapWriter`].
#[derive(Clone, Debug)]
pub struct ExportOptions {
    /// `world.svg`. Off by default: see [`Diagnostics`].
    pub svg: bool,
    /// `raw_data/`: world.json and the layer PNGs.
    pub raw_data: bool,
    /// The directory that receives the thumbnail of `world.png`
    /// (`thumb.png`): a body keeps one for all its resolutions, beside its
    /// `sizes/` (see [`crate::sizes`]). None writes no thumbnail; it is
    /// never written into the export's own directory.
    pub thumb_dir: Option<PathBuf>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            svg: false,
            raw_data: true,
            thumb_dir: None,
        }
    }
}

/// The epoch of a drift in an id: `None` at the present, else `past` or
/// `future` and the Ma, a decimal point written `p` (ids hold lowercase
/// letters, digits and dashes only).
pub fn drift_label(drift_ma: f32) -> Option<String> {
    if drift_ma == 0.0 {
        return None;
    }
    let side = if drift_ma < 0.0 { "past" } else { "future" };
    // The shortest decimal that reads back the same f32 (12.5, not 12.50).
    let ma = format!("{}", drift_ma.abs()).replace('.', "p");
    Some(format!("{side}{ma}ma"))
}

/// Writes every output of a generated world into `dir` (created if needed),
/// its thumbnail into `options.thumb_dir` (created if needed) when set.
/// The exports are independent and dominated by PNG encoding, so they run in
/// parallel. The raw layers no map reads are written before the render and
/// freed (`raw_data::write_early_layers`), which lowers the memory peak.
pub fn export_world(
    generated: Generated,
    dir: &Path,
    options: ExportOptions,
) -> Result<(), String> {
    let Generated {
        mut world,
        river_coverage,
    } = generated;
    let path = |sub: &str| dir.join(sub).to_string_lossy().into_owned();
    let io = |e: std::io::Error| e.to_string();
    std::fs::create_dir_all(dir).map_err(io)?;
    if let Some(thumb_dir) = &options.thumb_dir {
        std::fs::create_dir_all(thumb_dir).map_err(io)?;
    }
    // The raw layers no map reads are written first, then freed before the
    // render: they do not stay in memory beside it.
    let early = if options.raw_data {
        std::fs::create_dir_all(dir.join("raw_data")).map_err(io)?;
        Some(raw_data::write_early_layers(&world, &dir.join("raw_data"))?)
    } else {
        None
    };
    raw_data::free_early_layers(&mut world);
    let world = &world;

    let layers = MapLayers::with_coverage(world, river_coverage);
    let layers = &layers;
    // The detailed render: shaded for world.png (which adds the reference
    // lines), unshaded with its relief normals for the viewer
    // (raw_data/colour.png and normal.png). A giant's cloud bands come
    // rendered from its own pipeline, with no relief.
    let giant = !world.surface.is_empty();
    let rendered = if giant {
        Rendered {
            shaded: world.surface.clone(),
            colour: world.surface.clone(),
            normal: vec![relief::FLAT; world.surface.len()],
        }
    } else {
        render_both(world, layers)
    };
    let rendered = &rendered;
    let surface = &rendered.shaded;
    type Export<'a> = Box<dyn Fn() -> Result<(), String> + Send + Sync + 'a>;
    let mut exports: Vec<Export> = vec![
        Box::new(|| {
            let mut map = surface.clone();
            // Cloud tops have no contour lines.
            if !giant {
                draw_contours(world, layers, &mut map);
            }
            // The thumbnail is downscaled from the image itself: no other
            // full-size copy.
            let image = world_image(world, map);
            image.save(path("world.png")).map_err(|e| e.to_string())?;
            match &options.thumb_dir {
                Some(thumb_dir) => thumbnail::thumbnail(&image)
                    .save(thumb_dir.join(thumbnail::THUMB_FILE))
                    .map_err(|e| e.to_string()),
                None => Ok(()),
            }
        }),
        Box::new(|| export_legend_png(world, &path("legend.png")).map_err(|e| e.to_string())),
    ];
    if options.svg {
        exports.push(Box::new(|| {
            export_svg(world, layers, &path("world.svg")).map_err(|e| e.to_string())
        }));
    }
    if let Some(early) = &early {
        exports.push(Box::new(move || {
            let maps = raw_data::SurfaceMaps {
                colour: &rendered.colour,
                normal: &rendered.normal,
            };
            raw_data::write_raw_data(world, &dir.join("raw_data"), Some(&maps), early)
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

/// The hillshade's [`RELIEF_KM`] on a body of `circumference_km`: on small
/// bodies (moons below about 300 km in radius), whose pixels are a fraction
/// of a km, it falls with their size towards the true relief
/// (`craters::ELEVATION_KM`), or every slope would saturate.
pub(crate) fn relief_km(circumference_km: f32) -> f32 {
    let small = 2.0 * std::f32::consts::PI * SMALL_BODY_RADIUS_KM;
    (RELIEF_KM * (circumference_km / small).min(1.0)).max(crate::craters::ELEVATION_KM)
}

/// Radius under which a body's hillshade exaggerates less, km.
const SMALL_BODY_RADIUS_KM: f32 = 300.0;
/// Light direction (east, north, up), from the north-west at 45° altitude.
pub(crate) const LIGHT: [f32; 3] = [-0.5, 0.5, std::f32::consts::FRAC_1_SQRT_2];

/// Brightness factor of a land pixel lit from the north-west: 1.0 on flat
/// ground, darker on slopes facing south-east, brighter facing north-west.
/// From the slopes of [`relief::slopes`] (central differences; longitude
/// wraps, latitude clamps at the poles).
fn hillshade_of(dz_east: f32, dz_north: f32) -> f32 {
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
    let look = &params.look;
    let waterline = params.open_waterline();
    let river = look.remap(Biome::River);
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

    /// Whether pixel `i` is drawn as a river over land (a river biome the
    /// coverage marks), not as water.
    pub fn on_river(&self, world: &World, i: usize) -> bool {
        let river_biome = world.params.look.remap(Biome::River);
        world.biomes[i] == river_biome && self.coverage[i] >= rivers::RIVER_COVERAGE
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
/// over `raw_data/colour.png`, which has none.
pub fn draw_contours(world: &World, layers: &MapLayers, pixels: &mut [[u8; 3]]) {
    let (w, h) = (world.width, world.height);
    let river_biome = world.params.look.remap(Biome::River);
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

/// How much an albedo shift brightens the ground (see [`World::albedo`]):
/// a shift of 1 takes a colour this share of the way to white, so fresh
/// crater rims and rays stand out on dark ground too, as on the Moon; a
/// negative shift darkens it by the same share.
pub(crate) const ALBEDO_GAIN: f32 = 0.6;

/// A shaded colour (0–255 per channel) under an albedo shift `a` in `[-1, 1]`.
pub(crate) fn with_albedo(colour: [f32; 3], a: f32) -> [f32; 3] {
    if a >= 0.0 {
        colour.map(|c| c + (255.0 - c).max(0.0) * ALBEDO_GAIN * a)
    } else {
        colour.map(|c| c * (1.0 + ALBEDO_GAIN * a))
    }
}

/// Computes the final colour of every pixel, in the same row-major order as
/// the world layers, without contour lines (see [`draw_contours`]).
pub fn render_pixels(world: &World, layers: &MapLayers, style: Style) -> Vec<[u8; 3]> {
    match style {
        Style::Detailed => render_both(world, layers).shaded,
        Style::Flat => render_flat(world, layers),
    }
}

/// The detailed render in its three forms, from one pass over the pixels.
pub struct Rendered {
    /// Hillshaded, lit from the north-west: the flat images (`world.png`,
    /// the thumbnail, the drift previews).
    pub shaded: Vec<[u8; 3]>,
    /// The same without the hillshade (`raw_data/colour.png`): the viewer
    /// lights it with [`Rendered::normal`].
    pub colour: Vec<[u8; 3]>,
    /// The relief normals (see [`relief`]): flat on water, softened on lava.
    pub normal: Vec<[u8; 3]>,
}

/// The detailed render, shaded and unshaded, and its relief normals. Every
/// step but the hillshade is computed once per pixel.
pub fn render_both(world: &World, layers: &MapLayers) -> Rendered {
    let (w, h) = (world.width, world.height);
    let grid = world.params.grid();
    let look = &world.params.look;
    let (coverage, depths) = (&layers.coverage, &layers.depths);
    let river_biome = look.remap(Biome::River);
    let river_colour = linear_of(surface_color(river_biome));
    let variation = SurfaceVariation::new(world.params.seed, world.params.circumference_km as f64);
    let (lon_step, lat_step) = (
        std::f64::consts::TAU / w as f64,
        std::f64::consts::PI / h as f64,
    );
    let to_u8 = |c: [f32; 3]| c.map(|c| c.clamp(0.0, 255.0) as u8);

    let pixels: Vec<([u8; 3], [u8; 3], [u8; 3])> = (0..h)
        .into_par_iter()
        .flat_map_iter(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let i = world.index(x, y);
            let biome = world.biomes[i];
            let on_river = biome == river_biome && coverage[i] >= rivers::RIVER_COVERAGE;
            let lon = (x as f64 + 0.5) * lon_step;
            let lat = std::f64::consts::FRAC_PI_2 - (y as f64 + 0.5) * lat_step;
            if biome.is_water() && !on_river {
                let c = to_u8(to_srgb(variation.sea(
                    biome,
                    water_colour(biome, depths[i]),
                    lon,
                    lat,
                )));
                return (c, c, relief::FLAT);
            }
            // The land under a river: the biome the pixel has without it.
            let input = world.biome_input(i);
            let dry = look.rules.choose(input);
            let land = look.remap(dry);
            // Lakes are not recomputed without their flag: a pixel whose
            // biome differs from its recomputed one keeps a flat colour.
            let color = if land == biome || on_river {
                ecotone_colour_around(look, input, Some(land))
            } else {
                surface_color(biome).map(f32::from)
            };
            // Lava varies too: switching the variation by the sharp lava
            // class would draw a step through the soft ecotone around it.
            let color = variation.apply(color, lon, lat);
            let (dz_east, dz_north) = relief::slopes(world, &grid, x, y);
            let mut f = hillshade_of(dz_east, dz_north);
            let mut normal = relief::normal_of(dz_east, dz_north);
            // Lava is fluid: its relief barely shows.
            if dry.is_lava() {
                f = 1.0 + (f - 1.0) * crate::detail::LAVA_SHADE;
                normal = relief::soften(normal, crate::detail::LAVA_SHADE);
            }
            let frozen = look.rules.frozen(world.biome_input(i));
            let finish = |shade: f32| {
                let mut color = color.map(|c| c * shade);
                // Fresh craters: bright ejecta and rays, on rock (not on lava).
                if let Some(&a) = world.albedo.get(i)
                    && !dry.is_lava()
                {
                    color = with_albedo(color, a);
                }
                // Anti-aliased river on top, except where it is frozen or
                // under lava.
                let c = coverage[i];
                if c > 0.0 && !frozen && !dry.is_lava() {
                    color = blend_river(color, river_colour, c);
                }
                to_u8(color)
            };
            (finish(f), finish(1.0), relief::encode_normal(normal))
        })
        .collect();
    let mut rendered = Rendered {
        shaded: Vec::with_capacity(pixels.len()),
        colour: Vec::with_capacity(pixels.len()),
        normal: Vec::with_capacity(pixels.len()),
    };
    for (shaded, colour, normal) in pixels {
        rendered.shaded.push(shaded);
        rendered.colour.push(colour);
        rendered.normal.push(normal);
    }
    rendered
}

/// The flat render of the SVG: classification colours, no per-pixel
/// variation, so its run-length encoding stays compact; rivers are drawn
/// as vector paths over it.
fn render_flat(world: &World, layers: &MapLayers) -> Vec<[u8; 3]> {
    let look = &world.params.look;
    let river_biome = look.remap(Biome::River);
    (0..world.height)
        .into_par_iter()
        .flat_map_iter(|y| (0..world.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let i = world.index(x, y);
            let biome = world.biomes[i];
            let on_river = biome == river_biome && layers.coverage[i] >= rivers::RIVER_COVERAGE;
            if biome.is_water() && !on_river {
                return flat_water_colour(biome, layers.depths[i])
                    .map(|c| c.clamp(0.0, 255.0) as u8);
            }
            let land = look.remap(look.rules.choose(world.biome_input(i)));
            biome_color(if on_river { land } else { biome })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Biome, drift_label, latitude_row, water_colour};
    use crate::export::colour::to_srgb;

    #[test]
    fn epochs_of_a_world_get_ids_of_their_own() {
        assert_eq!(drift_label(0.0), None);
        assert_eq!(drift_label(-250.0).as_deref(), Some("past250ma"));
        assert_eq!(drift_label(12.5).as_deref(), Some("future12p5ma"));
        assert_eq!(drift_label(-0.1).as_deref(), Some("past0p1ma"));
        for drift in [-300.0f32, -12.5, 0.1, 300.0] {
            let label = drift_label(drift).unwrap();
            assert!(
                label
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "{label}"
            );
        }
    }

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

    /// The shaded render (world.png, the thumbnail, the drift previews)
    /// is exactly the one drawn before the colour and the normals were
    /// split out.
    #[test]
    fn the_shaded_render_is_unchanged() {
        for (seed, planet) in [(3, "terran"), (7, "volcanic")] {
            let world =
                crate::generation::generate(&crate::generation::tests::params(seed, planet)).world;
            let layers = super::MapLayers::new(&world);
            let r = super::render_both(&world, &layers);
            assert!(
                r.shaded == super::reference::reference_render(&world, &layers),
                "{planet}"
            );
        }
    }

    /// Water has no relief: its colour is the shaded one and its normal
    /// faces up. On land the normals follow the relief, softened on lava.
    #[test]
    fn the_colour_and_the_normals_follow_the_shading() {
        use super::relief::{FLAT, decode_normal, relief_normal, soften};
        let world =
            crate::generation::generate(&crate::generation::tests::params(7, "volcanic")).world;
        let layers = super::MapLayers::new(&world);
        let r = super::render_both(&world, &layers);
        let grid = world.params.grid();
        let (mut water, mut lava, mut land) = (0, 0, 0);
        for y in 0..world.height {
            for x in 0..world.width {
                let i = world.index(x, y);
                let biome = world.biomes[i];
                if biome.is_water() && !layers.on_river(&world, i) {
                    assert_eq!(r.colour[i], r.shaded[i]);
                    assert_eq!(r.normal[i], FLAT);
                    water += 1;
                    continue;
                }
                let dry = world.params.look.rules.choose(world.biome_input(i));
                let expected = if dry.is_lava() {
                    lava += 1;
                    soften(
                        relief_normal(&world, &grid, x, y),
                        crate::detail::LAVA_SHADE,
                    )
                } else {
                    land += 1;
                    relief_normal(&world, &grid, x, y)
                };
                let got = decode_normal(r.normal[i]);
                for k in 0..3 {
                    assert!(
                        (got[k] - expected[k]).abs() < 0.01,
                        "pixel {i}: {got:?} vs {expected:?}"
                    );
                }
            }
        }
        assert!(water > 0 && lava > 0 && land > 0, "{water} {lava} {land}");
    }

    /// Exports `params`' world into a fresh directory and returns it.
    fn exported(params: &crate::world::WorldParams, tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mapgen-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let generated = crate::generation::generate(params);
        super::export_world(generated, &dir, super::ExportOptions::default()).unwrap();
        dir
    }

    /// Writing the layers no map reads before the render, then freeing
    /// them, leaves `raw_data/` byte for byte as written from the whole
    /// world.
    #[test]
    fn the_early_layers_leave_the_raw_data_unchanged() {
        for (seed, planet) in [(3, "terran"), (5, "barren")] {
            let params = crate::generation::tests::params(seed, planet);
            let dir = exported(&params, &format!("early-{planet}"));

            let reference = std::env::temp_dir()
                .join(format!("mapgen-early-ref-{planet}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&reference);
            std::fs::create_dir_all(&reference).unwrap();
            let generated = crate::generation::generate(&params);
            let world = &generated.world;
            assert!(world.elevation_base.is_some(), "{planet}");
            let layers = super::MapLayers::with_coverage(world, generated.river_coverage.clone());
            let rendered = super::render_both(world, &layers);
            let maps = super::raw_data::SurfaceMaps {
                colour: &rendered.colour,
                normal: &rendered.normal,
            };
            super::export_raw_data(world, &reference, Some(&maps)).unwrap();

            let mut files: Vec<_> = std::fs::read_dir(&reference)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            files.sort();
            for early in ["flow.png", "floodplain.png", "elevation_base.png"] {
                assert!(files.iter().any(|f| f == early), "{planet}: {early}");
            }
            let written: Vec<_> = std::fs::read_dir(dir.join("raw_data"))
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert_eq!(written.len(), files.len(), "{planet}");
            for file in &files {
                assert!(
                    std::fs::read(reference.join(file)).unwrap()
                        == std::fs::read(dir.join("raw_data").join(file)).unwrap(),
                    "{planet}: {file:?}"
                );
            }
            std::fs::remove_dir_all(&dir).unwrap();
            std::fs::remove_dir_all(&reference).unwrap();
        }
    }

    #[test]
    fn exports_the_colour_and_the_normals_instead_of_the_surface() {
        use crate::export::raw_data::read_metadata;
        let params = crate::world::WorldParams {
            width: 1024,
            height: 512,
            ..crate::generation::tests::params(3, "terran")
        };
        let dir = exported(&params, "colour-normal");
        let raw = dir.join("raw_data");
        let size = |f: &str| image::image_dimensions(raw.join(f)).unwrap();
        assert_eq!(size("colour.png"), (1024, 512));
        assert_eq!(size("normal.png"), (1024, 512));
        assert_eq!(size("colour_small.png"), (512, 256));
        assert_eq!(size("normal_small.png"), (512, 256));
        assert!(!raw.join("surface.png").exists());
        let meta = read_metadata(&raw).unwrap();
        let colour = meta.layers.colour.as_ref().unwrap();
        assert_eq!(
            (colour.file.as_str(), colour.encoding.as_str()),
            ("colour.png", "rgb8")
        );
        let normal = meta.layers.normal.as_ref().unwrap();
        assert_eq!(
            (normal.file.as_str(), normal.encoding.as_str()),
            ("normal.png", "rgb8-normal")
        );
        assert_eq!(
            meta.layers.colour_small.as_ref().unwrap().file,
            "colour_small.png"
        );
        assert_eq!(
            meta.layers.normal_small.as_ref().unwrap().file,
            "normal_small.png"
        );
        // The flat images keep their hillshade: world.png is the shaded
        // render, not the colour layer.
        let world = image::open(dir.join("world.png")).unwrap().to_rgb8();
        let colour = image::open(raw.join("colour.png")).unwrap().to_rgb8();
        assert_ne!(world.as_raw(), colour.as_raw());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_giant_has_its_bands_as_colour_and_flat_normals() {
        // A Jupiter: the giant's own pipeline draws its bands.
        let params = crate::world::WorldParams {
            giant_mass_earth: Some(317.8),
            ..crate::generation::tests::params(3, "gas-giant")
        };
        let generated = crate::generation::generate(&params);
        let bands = generated.world.surface.clone();
        assert!(!bands.is_empty());
        let dir = exported(&params, "giant-normal");
        let raw = dir.join("raw_data");
        let colour = image::open(raw.join("colour.png")).unwrap().to_rgb8();
        assert_eq!(colour.as_raw().as_slice(), bands.as_flattened());
        let normal = image::open(raw.join("normal.png")).unwrap().to_rgb8();
        assert!(normal.pixels().all(|p| p.0 == super::relief::FLAT));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn moons_are_generated_as_bodies_of_their_own() {
        use crate::export::raw_data::{WorldMetadata, read_metadata};
        // A small world far from its star, of the first seed that draws it
        // moons.
        let (request, resolved, system) = (0..64)
            .find_map(|seed| {
                let request = crate::request::WorldRequest {
                    seed: Some(seed),
                    mass_earth: Some(0.02),
                    star: Some("G2V".into()),
                    orbit_au: Some(30.0),
                    eccentricity: Some(0.0),
                    host: Some(crate::exomoon::HostMode::None),
                    width: Some(128),
                    height: Some(64),
                    ..Default::default()
                };
                let resolved = request.resolve(&mut rand::rng()).unwrap();
                let system = crate::moons::MoonSystem::draw(&resolved.params)
                    .filter(|s| !s.moons.is_empty())?;
                Some((request, resolved, system))
            })
            .expect("a seed with moons");
        let dir = std::env::temp_dir().join(format!("mapgen-moons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Generated as bodies of the planet's system, with it.
        let drawn = crate::system::draw_around_params(
            &request,
            resolved.params.clone(),
            resolved.model().clone(),
        );
        crate::system::write_system(&dir, &drawn).unwrap();
        let planet = drawn.focus.clone().unwrap();
        let done = crate::system::generate_neighbourhood(&dir, &planet, |_| {}).unwrap();
        assert_eq!(done.len(), 1 + system.moons.len());
        for (k, moon) in system.moons.iter().enumerate() {
            let id = format!("{planet}-{}", k + 1);
            let body = drawn.bodies.iter().find(|b| b.id == id).unwrap();
            let (width, height) = (body.params.width, body.params.height);
            let raw = crate::sizes::size_dir(&dir.join("bodies").join(&id), width, height)
                .join("raw_data");
            let meta: WorldMetadata = read_metadata(&raw).unwrap();
            assert!(
                ["icy-moon", "rocky-moon"].contains(&meta.class.as_str()),
                "{}",
                meta.class
            );
            assert_eq!(meta.seed, moon.seed);
            assert!(
                (meta.circumference_km / (2.0 * std::f32::consts::PI) - moon.radius_km).abs()
                    < 0.01
            );
            // No moons of its own; an irregular one has a shape layer.
            assert!(meta.moons.is_none());
            if !moon.round {
                assert!(meta.shape.is_some() && meta.layers.shape.is_some());
                assert!(raw.join("shape.png").exists());
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
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

/// The detailed render as `main` drew it before the colour and the normals
/// were split out, kept for one test: the shaded render must stay exactly
/// the same, on any platform.
#[cfg(test)]
mod reference {
    use super::*;
    use crate::geo::Grid;

    fn reference_hillshade(world: &World, grid: &Grid, x: u32, y: u32) -> f32 {
        let (w, h) = (world.width, world.height);
        let relief = relief_km(world.params.circumference_km);
        let elev = |x: u32, y: u32| world.elevation[world.index(x, y)] * relief;
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

    pub(super) fn reference_render(world: &World, layers: &MapLayers) -> Vec<[u8; 3]> {
        let style = Style::Detailed;
        let (w, h) = (world.width, world.height);
        let grid = world.params.grid();
        let look = &world.params.look;
        let (coverage, depths) = (&layers.coverage, &layers.depths);
        let river_biome = look.remap(Biome::River);
        let river_colour = linear_of(surface_color(river_biome));
        let variation =
            SurfaceVariation::new(world.params.seed, world.params.circumference_km as f64);
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
                let dry = look.rules.choose(input);
                let land = look.remap(dry);
                if style == Style::Flat {
                    // Rivers are drawn as vector paths over the flat SVG map.
                    return biome_color(if on_river { land } else { biome });
                }
                // Lakes are not recomputed without their flag: a pixel whose
                // biome differs from its recomputed one keeps a flat colour.
                let mut color = if land == biome || on_river {
                    ecotone_colour_around(look, input, Some(land))
                } else {
                    surface_color(biome).map(f32::from)
                };
                // Lava varies too: switching the variation by the sharp lava
                // class would draw a step through the soft ecotone around it.
                color = variation.apply(color, lon, lat);
                let mut f = reference_hillshade(world, &grid, x, y);
                // Lava is fluid: its relief barely shows.
                if dry.is_lava() {
                    f = 1.0 + (f - 1.0) * crate::detail::LAVA_SHADE;
                }
                color = color.map(|c| c * f);
                // Fresh craters: bright ejecta and rays, on rock (not on lava).
                if let Some(&a) = world.albedo.get(i)
                    && !dry.is_lava()
                {
                    color = with_albedo(color, a);
                }
                // Anti-aliased river on top, except where it is frozen or under
                // lava.
                let c = coverage[i];
                if c > 0.0 && !look.rules.frozen(world.biome_input(i)) && !dry.is_lava() {
                    color = blend_river(color, river_colour, c);
                }
                color.map(|c| c.clamp(0.0, 255.0) as u8)
            })
            .collect()
    }
}
