//! Detail tiles for zooming far below the resolution of a generated world.
//!
//! The generated layers stop at the base resolution (about 20 km per pixel
//! at 1920 pixels on an Earth-sized planet), and the generation noise has no
//! feature smaller than that anyway: generating a larger map adds no detail.
//! Detail is therefore synthesised on demand, consistently with the base
//! world:
//!
//! - **Elevation**: the base elevation, smoothly interpolated, plus fractal
//!   octaves limited to features smaller than two base pixels (so the base
//!   coastlines move by less than a base pixel) and larger than six tile
//!   pixels (so a tile is the low-pass of its children, without grain). Amplitudes follow
//!   a 1/f law on high ground and fall faster on plains, which stay calm at
//!   every zoom level. Height detail fades out at the waterline; near the
//!   coast the base elevation is read at a sideways-displaced position
//!   instead, so coastlines get a fractal outline without islets or ponds.
//! - **Climate**: temperature, moisture and the other biome inputs are read
//!   at a slightly warped position, so biome borders become irregular instead
//!   of interpolated straight edges; temperature follows the new elevation.
//! - **Biomes**: the generation's own rules and preset remap, with soft
//!   borders, hillshading and the vector rivers drawn at the tile's scale.
//!
//! Tiles form an equirectangular quadtree: level `z` has `2^(z+1)` × `2^z`
//! tiles of [`TILE_SIZE`] pixels, column 0 at longitude 0 and row 0 at the
//! north pole.
//!
//! A [`DetailWorld`] keeps the base layers as stored in `raw_data/` (16 bits
//! per value, see [`Layer16`]), decoded as they are sampled, plus a byte of
//! flags and the water depth per pixel: about 23 bytes per base pixel
//! (0.75 GB at 8192 × 4096).
use crate::biome::{Biome, BiomeInput, surface_color};
use crate::climate::temperature;
use crate::export::ecotone::ecotone_colour_for;
use crate::export::raw_data::{Layer16, LoadedWorld};
use crate::export::variation::SurfaceVariation;
use crate::export::{LIGHT, RELIEF_KM, blend_river, shade, water_colour, water_depths_of};
use crate::hex;
use crate::rivers;
use image::{Rgb, RgbImage};
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use std::f64::consts::{PI, TAU};

/// Tile side in pixels.
pub const TILE_SIZE: u32 = 256;
/// Deepest level served: at z = 8 an Earth-sized planet is about 300 m per pixel.
pub const MAX_LEVEL: u32 = 8;
/// Amplitude of the first detail octave (elevation units), on plains.
const DETAIL_AMPLITUDE: f32 = 0.0015;
/// Extra amplitude on high ground: detail grows to `1 + MOUNTAIN_BOOST` times
/// the plain amplitude 0.3 elevation units above the coastline.
const MOUNTAIN_BOOST: f32 = 3.0;
/// Amplitude ratio between successive detail octaves: below 1/2 on plains,
/// so they get calmer at every zoom level instead of rougher; 1/2 (1/f) on
/// high ground.
const PLAIN_GAIN: f64 = 0.3;
const MOUNTAIN_GAIN: f64 = 0.4;
/// Depth of a river's channel at the smallest river flow, in elevation units
/// (about 30 m at the hillshade scale); it grows with `flow^0.3`.
const CHANNEL_DEPTH: f32 = 5e-4;
/// Half-width of the channel banks, in drawn river widths: the banks rise
/// from the middle of the river to `CHANNEL_SPREAD / 2` widths away.
const CHANNEL_SPREAD: f64 = 2.0;
/// Wavelength of the finest detail octave, in tile pixels: features of one
/// or two pixels only read as grain.
const FINEST_PX: f64 = 6.0;
/// Share of the hillshade kept on lava: a fluid, glowing surface.
pub(crate) const LAVA_SHADE: f32 = 0.4;
/// Share of the height detail removed on a floodplain (weight 1).
const FLOODPLAIN_CALM: f32 = 0.8;
/// Height detail fades out within this distance of the waterline, so it can
/// never turn the shore into islets and ponds: coasts get their detail from
/// the sideways displacement instead.
const SHORE_FADE: f32 = 0.006;
/// Sideways displacement of the base elevation near the coast, per detail
/// octave, as a share of the octave's wavelength (thinning by
/// [`COAST_WARP_DECAY`] per octave): a fractal outline at every zoom level.
const COAST_WARP: f64 = 0.35;
const COAST_WARP_DECAY: f64 = 0.8;
/// Distance from the waterline (elevation units) over which the coastal
/// displacement fades out, so inland relief and rivers stay in place.
const COAST_BAND: (f32, f32) = (0.01, 0.04);
/// Warp of the climate sampling position, in base pixels.
const CLIMATE_WARP_PX: f64 = 1.0;

/// A tile of the quadtree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileId {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

impl TileId {
    /// Columns and rows at this level.
    pub fn grid(self) -> (u32, u32) {
        (1 << (self.z + 1), 1 << self.z)
    }

    /// Whether the tile exists. The level is checked first: [`grid`] would
    /// overflow on any level from 31 up.
    ///
    /// [`grid`]: Self::grid
    pub fn is_valid(self) -> bool {
        if self.z > MAX_LEVEL {
            return false;
        }
        let (cols, rows) = self.grid();
        self.x < cols && self.y < rows
    }

    /// Longitude and latitude (radians) of the centre of tile pixel `(px, py)`.
    fn lon_lat(self, px: f64, py: f64) -> (f64, f64) {
        let (cols, rows) = self.grid();
        let u = (self.x as f64 + (px + 0.5) / TILE_SIZE as f64) / cols as f64;
        let v = (self.y as f64 + (py + 0.5) / TILE_SIZE as f64) / rows as f64;
        (u * TAU, PI / 2.0 - v * PI)
    }
}

/// Everything a tile needs, prepared once per world.
pub struct DetailWorld {
    world: LoadedWorld,
    noise: Perlin,
    warp_noise: Perlin,
    /// Colour variation within biomes, as on the base map.
    variation: SurfaceVariation,
    /// Bounding box of each river polyline in base pixels.
    river_bounds: Vec<[f64; 4]>,
    /// Per base pixel: [`LAKE`] and [`RIVER`] bits.
    flags: Vec<u8>,
    /// Depth of each base water pixel below its surface (see
    /// [`water_depths_of`]), for the shallow tint of lakes.
    depths: Vec<f32>,
}

/// Flag of base lake pixels (water above the waterline, not under a drawn
/// river); sampled bilinearly for smooth shores.
const LAKE: u8 = 1;
/// Flag of base river pixels (see [`hex::is_river`]).
const RIVER: u8 = 2;

/// A row-major base layer, read one value at a time.
trait Samples {
    fn value(&self, i: usize) -> f32;
}

impl Samples for Vec<f32> {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        self[i]
    }
}

impl Samples for Layer16 {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        self.get(i)
    }
}

/// 1 on lake pixels, 0 elsewhere.
struct Lakes<'a>(&'a [u8]);

impl Samples for Lakes<'_> {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        if self.0[i] & LAKE != 0 { 1.0 } else { 0.0 }
    }
}

/// Share of the height detail each base pixel's surface gets (see
/// [`roughness`]), sampled bilinearly so it changes smoothly across biome
/// borders.
struct Roughness<'a>(&'a [Biome]);

impl Samples for Roughness<'_> {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        roughness(self.0[i])
    }
}

/// Bilinear sample of a row-major base layer at base pixel coordinates
/// (pixel centres at +0.5), wrapping longitude and clamping latitude.
fn bilinear(layer: &(impl Samples + ?Sized), width: u32, height: u32, x: f64, y: f64) -> f32 {
    let (w, h) = (width as i64, height as i64);
    let (x, y) = (x - 0.5, (y - 0.5).clamp(0.0, (h - 1) as f64));
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
    let at = |xx: i64, yy: i64| layer.value((yy.clamp(0, h - 1) * w + xx.rem_euclid(w)) as usize);
    let (xi, yi) = (x0 as i64, y0 as i64);
    let top = at(xi, yi) * (1.0 - fx) + at(xi + 1, yi) * fx;
    let bottom = at(xi, yi + 1) * (1.0 - fx) + at(xi + 1, yi + 1) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Bicubic (Catmull-Rom) sample of a base layer: unlike bilinear, its slope
/// is continuous, so hillshading shows no seams along base pixel edges.
fn bicubic(layer: &(impl Samples + ?Sized), width: u32, height: u32, x: f64, y: f64) -> f32 {
    let (w, h) = (width as i64, height as i64);
    let (x, y) = (x - 0.5, (y - 0.5).clamp(0.0, (h - 1) as f64));
    let (x0, y0) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
    let at = |xx: i64, yy: i64| layer.value((yy.clamp(0, h - 1) * w + xx.rem_euclid(w)) as usize);
    let cubic = |p: [f32; 4], t: f32| {
        p[1] + 0.5
            * t
            * (p[2] - p[0]
                + t * (2.0 * p[0] - 5.0 * p[1] + 4.0 * p[2] - p[3]
                    + t * (3.0 * (p[1] - p[2]) + p[3] - p[0])))
    };
    let rows: [f32; 4] = std::array::from_fn(|j| {
        let yy = y0 - 1 + j as i64;
        cubic(std::array::from_fn(|i| at(x0 - 1 + i as i64, yy)), fx)
    });
    cubic(rows, fy)
}

/// Fractal noise at a unit vector, octaves from `first` to `last` wavelength
/// (km), each `gain` times the amplitude of the previous one.
fn band_limited(
    noise: &Perlin,
    p: [f64; 3],
    radius_km: f64,
    first_km: f64,
    last_km: f64,
    gain: f64,
) -> f32 {
    let mut sum = 0.0;
    let (mut wavelength, mut amplitude) = (first_km, 1.0);
    while wavelength >= last_km {
        let f = radius_km / wavelength;
        sum += noise.get([p[0] * f, p[1] * f, p[2] * f]) * amplitude;
        wavelength /= 2.0;
        amplitude *= gain;
    }
    sum as f32
}

/// Share of the height detail a biome's surface gets: snow, ice and lava
/// are smooth, sand and ash nearly so; rock and vegetation keep it all.
fn roughness(biome: Biome) -> f32 {
    match biome {
        Biome::Snow | Biome::IceCap | Biome::GlacialPlain | Biome::SeaIce => 0.1,
        Biome::Volcano | Biome::LavaField | Biome::MagmaSea => 0.05,
        Biome::Desert | Biome::DustPlain | Biome::Beach | Biome::AshLand => 0.4,
        _ => 1.0,
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl DetailWorld {
    pub fn new(world: LoadedWorld) -> Self {
        let river_bounds = world
            .rivers
            .iter()
            .map(|line| {
                let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
                for p in &line.points {
                    b = [
                        b[0].min(p[0]),
                        b[1].min(p[1]),
                        b[2].max(p[0]),
                        b[3].max(p[1]),
                    ];
                }
                b
            })
            .collect();
        let seed = world.params.seed;
        let coverage = rivers::coverage(
            &world.rivers,
            world.width,
            world.height,
            world.params.circumference_km as f64,
        );
        let planet = &world.params.planet;
        let waterline = world.params.sea_level + planet.rules.waterline;
        let river = planet.remap(Biome::River);
        let flags = (0..world.biomes.len())
            .into_par_iter()
            .map(|i| {
                let biome = world.biomes[i];
                let lake = biome.is_water()
                    && world.elevation.get(i) >= waterline
                    && coverage[i] < rivers::RIVER_COVERAGE;
                let mut flags = if lake { LAKE } else { 0 };
                if hex::is_river(biome, river, coverage[i]) {
                    flags |= RIVER;
                }
                flags
            })
            .collect();
        drop(coverage);
        let depths = water_depths_of(&world.params, |i| world.elevation.get(i), &world.biomes);
        Self {
            flags,
            depths,
            noise: Perlin::new(seed.wrapping_add(1000)),
            warp_noise: Perlin::new(seed.wrapping_add(1001)),
            variation: SurfaceVariation::new(seed, world.params.circumference_km as f64),
            river_bounds,
            world,
        }
    }

    /// The base layers.
    pub fn world(&self) -> &LoadedWorld {
        &self.world
    }

    /// Whether base pixel `i` is a river (see [`hex::is_river`]).
    pub fn is_river(&self, i: usize) -> bool {
        self.flags[i] & RIVER != 0
    }

    /// Bytes held, river polylines aside.
    pub fn bytes(&self) -> usize {
        self.world.bytes()
            + self.flags.len()
            + self.depths.len() * size_of::<f32>()
            + self.river_bounds.len() * size_of::<[f64; 4]>()
    }

    /// Base pixel coordinates of a longitude and latitude.
    fn base_xy(&self, lon: f64, lat: f64) -> (f64, f64) {
        (
            lon / TAU * self.world.width as f64,
            (0.5 - lat / PI) * self.world.height as f64,
        )
    }

    fn base_px_km(&self) -> f64 {
        self.world.params.circumference_km as f64 / self.world.width as f64
    }

    /// Detailed elevation at a point, for a tile whose pixels are `pixel_km`.
    fn elevation(&self, lon: f64, lat: f64, pixel_km: f64) -> f32 {
        let w = &self.world;
        let (x, y) = self.base_xy(lon, lat);
        let base = bicubic(&w.elevation, w.width, w.height, x, y);
        let first = 2.0 * self.base_px_km();
        let last = FINEST_PX * pixel_km;
        if last > first {
            return base;
        }
        let p = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
        let radius = w.params.circumference_km as f64 / TAU;
        let waterline = w.params.sea_level + w.params.planet.rules.waterline;

        // Near the coast, read the base elevation at a position displaced
        // sideways by every detail octave: the coastline gets capes and bays
        // at each zoom level, and stays one connected line.
        let coastal = 1.0 - smoothstep(COAST_BAND.0, COAST_BAND.1, (base - waterline).abs());
        let base = if coastal > 0.0 {
            let (mut dx, mut dy) = (0.0, 0.0);
            let (mut wavelength, mut share) = (first, COAST_WARP);
            let mut k = 0.0;
            while wavelength >= last {
                let f = radius / wavelength;
                let shift = share * wavelength / self.base_px_km() * coastal as f64;
                dx += shift
                    * self
                        .warp_noise
                        .get([p[0] * f + 31.0 + k, p[1] * f, p[2] * f]);
                dy += shift
                    * self
                        .warp_noise
                        .get([p[0] * f, p[1] * f - 17.0 - k, p[2] * f]);
                wavelength /= 2.0;
                share *= COAST_WARP_DECAY;
                k += 7.3;
            }
            bicubic(&w.elevation, w.width, w.height, x + dx, y + dy)
        } else {
            base
        };

        // Height detail: calmer on plains (and at every zoom), stronger and
        // 1/f on high ground, none at the waterline.
        let above = base - waterline;
        let high = (above / 0.3).clamp(0.0, 1.0);
        // Snow, ice, lava and sand are smooth surfaces (see `roughness`).
        let amplitude = DETAIL_AMPLITUDE
            * (1.0 + MOUNTAIN_BOOST * high)
            * bilinear(&Roughness(&w.biomes), w.width, w.height, x, y);
        let gain = PLAIN_GAIN + (MOUNTAIN_GAIN - PLAIN_GAIN) * high as f64;
        let shore = smoothstep(0.0, SHORE_FADE, above.abs());
        // Floodplains stay flat valley floors.
        let floor = 1.0 - FLOODPLAIN_CALM * bilinear(&w.floodplain, w.width, w.height, x, y);
        base + amplitude * shore * floor * band_limited(&self.noise, p, radius, first, last, gain)
    }

    /// Renders one tile as an RGB image.
    pub fn render_tile(&self, tile: TileId) -> RgbImage {
        let w = &self.world;
        let planet = &w.params.planet;
        let (cols, _) = tile.grid();
        let n = TILE_SIZE as usize;
        let pixel_km = w.params.circumference_km as f64 / (cols as f64 * TILE_SIZE as f64);

        // Elevation with a one-pixel border, for slopes.
        let side = n + 2;
        let (coverage, channel) = self.river_coverage(tile, pixel_km);
        let waterline = w.params.sea_level + planet.rules.waterline;
        let elevation: Vec<f32> = (0..side * side)
            .into_par_iter()
            .map(|i| {
                let (px, py) = ((i % side) as f64 - 1.0, (i / side) as f64 - 1.0);
                let (lon, lat) = tile.lon_lat(px, py);
                let e = self.elevation(lon, lat.clamp(-PI / 2.0, PI / 2.0), pixel_km);
                // The river's channel and banks, never below the sea.
                if e >= waterline {
                    (e - channel[i]).max(waterline + 1e-6)
                } else {
                    e
                }
            })
            .collect();

        let river_colour = surface_color(planet.remap(Biome::River)).map(f32::from);
        let warp_km = CLIMATE_WARP_PX * self.base_px_km();
        let radius = w.params.circumference_km as f64 / TAU;

        let pixels: Vec<[u8; 3]> = (0..n * n)
            .into_par_iter()
            .map(|i| {
                let (px, py) = (i % n, i / n);
                let (lon, lat) = tile.lon_lat(px as f64, py as f64);
                let e = elevation[(py + 1) * side + px + 1];
                let relief = (e - w.params.sea_level).clamp(-1.0, 1.0);
                let above = relief - planet.rules.waterline;

                // Climate at a warped position: irregular biome borders.
                let p = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
                let f = radius / (4.0 * warp_km);
                let wx = self.warp_noise.get([p[0] * f, p[1] * f, p[2] * f]);
                let wy = self
                    .warp_noise
                    .get([p[0] * f + 19.1, p[1] * f - 7.3, p[2] * f + 3.7]);
                let (bx, by) = self.base_xy(lon, lat);
                let (cx, cy) = (bx + wx * CLIMATE_WARP_PX, by + wy * CLIMATE_WARP_PX);
                let sample = |layer: &dyn Samples| bilinear(layer, w.width, w.height, cx, cy);
                let base_above = sample(&w.elevation) - w.params.sea_level - planet.rules.waterline;
                // Temperature follows the detailed elevation (lapse rate).
                let t =
                    sample(&w.temperature) + temperature(0.0, above) - temperature(0.0, base_above);
                // Lakes: smooth shores from the base lake mask.
                let lake = sample(&Lakes(&self.flags)) > 0.5 && above >= 0.0;
                let input = BiomeInput {
                    elevation: relief,
                    moisture: sample(&w.moisture),
                    temperature: t,
                    volcanic_zone: sample(&w.volcanic_zone),
                    coast_distance_km: sample(&w.coast_distance_km),
                    lake,
                    river: false,
                    highland_offset: sample(&w.highland_offset),
                };
                let dry = planet.rules.choose(input);
                let biome = planet.remap(dry);
                let mut colour = if biome.is_water() {
                    // Seas from the detailed elevation; lakes from the base
                    // depth below their surface.
                    let depth = if above < 0.0 {
                        -above
                    } else {
                        sample(&self.depths)
                    };
                    water_colour(biome, depth)
                } else {
                    let mut shade = self.hillshade(&elevation, side, px + 1, py + 1, pixel_km, lat);
                    let mut colour = ecotone_colour_for(planet, input);
                    if dry.is_lava() {
                        shade = 1.0 + (shade - 1.0) * LAVA_SHADE;
                    } else {
                        colour = self.variation.apply(colour, lon, lat);
                    }
                    colour.map(|c| c * shade)
                };
                let c = coverage[i];
                if c > 0.0
                    && input.temperature >= planet.rules.polar
                    && !biome.is_water()
                    && !dry.is_lava()
                {
                    colour = blend_river(colour, river_colour, c);
                }
                colour.map(|c| c.clamp(0.0, 255.0) as u8)
            })
            .collect();

        RgbImage::from_fn(TILE_SIZE, TILE_SIZE, |x, y| {
            Rgb(pixels[(y * TILE_SIZE + x) as usize])
        })
    }

    /// Brightness of a land pixel lit from the north-west (as on the base map).
    fn hillshade(
        &self,
        elevation: &[f32],
        side: usize,
        x: usize,
        y: usize,
        pixel_km: f64,
        lat: f64,
    ) -> f32 {
        let at = |x: usize, y: usize| elevation[y * side + x] * RELIEF_KM;
        let dx_km = (pixel_km * lat.cos().max(0.05)) as f32;
        let dz_east = (at(x + 1, y) - at(x - 1, y)) / (2.0 * dx_km);
        let dz_north = (at(x, y - 1) - at(x, y + 1)) / (2.0 * pixel_km as f32);
        let normal = [-dz_east, -dz_north, 1.0];
        let len = normal.iter().map(|c| c * c).sum::<f32>().sqrt();
        let lit: f32 = normal.iter().zip(LIGHT).map(|(n, l)| n * l).sum::<f32>() / len;
        shade(lit / LIGHT[2])
    }

    /// Anti-aliased coverage of the rivers on a tile, drawn with the base
    /// map's width law at the tile level's resolution (see
    /// [`rivers::draw_width_px`]), so they do not jump when tiles appear;
    /// and the depth of their channels on the tile with a one-pixel border
    /// (row-major, `TILE_SIZE + 2` wide), for the elevation.
    fn river_coverage(&self, tile: TileId, pixel_km: f64) -> (Vec<f32>, Vec<f32>) {
        let w = &self.world;
        let n = TILE_SIZE as usize;
        let mut stamp = Stamp {
            n,
            coverage: vec![0f32; n * n],
            channel: vec![0f32; (n + 2) * (n + 2)],
        };
        let (cols, rows) = tile.grid();
        // Tile pixels per base pixel, and the tile's window in base pixels.
        let sx = cols as f64 * TILE_SIZE as f64 / w.width as f64;
        let sy = rows as f64 * TILE_SIZE as f64 / w.height as f64;
        let (ox, oy) = (
            tile.x as f64 * TILE_SIZE as f64 / sx,
            tile.y as f64 * TILE_SIZE as f64 / sy,
        );
        let (span_x, span_y) = (TILE_SIZE as f64 / sx, TILE_SIZE as f64 / sy);
        let margin = 8.0 / sx.min(sy) + 1.0;
        let raster = cols as f64 * TILE_SIZE as f64;
        for (line, b) in self.world.rivers.iter().zip(&self.river_bounds) {
            for shift in [-(w.width as f64), 0.0, w.width as f64] {
                let visible = b[0] + shift < ox + span_x + margin
                    && b[2] + shift > ox - margin
                    && b[1] < oy + span_y + margin
                    && b[3] > oy - margin;
                if !visible {
                    continue;
                }
                let to_tile = |p: [f64; 2]| [(p[0] + shift - ox) * sx, (p[1] - oy) * sy];
                // The same law as the base map, at this level's resolution.
                let tapers = rivers::tapers(line, w.width);
                let width_at =
                    |i: usize| rivers::draw_width_px(line.flow_km2[i], raster, pixel_km, tapers[i]);
                let depth_at = |i: usize| {
                    let q = line.flow_km2[i].max(rivers::RIVER_MIN_FLOW_KM2)
                        / rivers::RIVER_MIN_FLOW_KM2;
                    CHANNEL_DEPTH * q.powf(0.3) * tapers[i].min(1.0) as f32
                };
                let pts = &line.points;
                for s in 0..pts.len().saturating_sub(1) {
                    // Catmull-Rom between points, so zoomed rivers stay smooth.
                    let p0 = pts[s.saturating_sub(1)];
                    let (p1, p2) = (pts[s], pts[s + 1]);
                    let p3 = pts[(s + 2).min(pts.len() - 1)];
                    let (a, b2) = (to_tile(p1), to_tile(p2));
                    let length = ((b2[0] - a[0]).powi(2) + (b2[1] - a[1]).powi(2)).sqrt();
                    let pieces = (length / 2.0).ceil().clamp(1.0, 64.0) as usize;
                    let (wa, wb) = (width_at(s), width_at(s + 1));
                    let (da, db) = (depth_at(s), depth_at(s + 1));
                    let mut prev = a;
                    for j in 1..=pieces {
                        let t = j as f64 / pieces as f64;
                        let point = catmull_rom(p0, p1, p2, p3, t);
                        let next = to_tile(point);
                        let width = wa + (wb - wa) * t;
                        let depth = da + (db - da) * t as f32;
                        stamp.segment(prev, next, width, depth);
                        prev = next;
                    }
                }
            }
        }
        (stamp.coverage, stamp.channel)
    }
}

fn catmull_rom(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], t: f64) -> [f64; 2] {
    let (t2, t3) = (t * t, t * t * t);
    std::array::from_fn(|k| {
        0.5 * ((2.0 * p1[k])
            + (-p0[k] + p2[k]) * t
            + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * t2
            + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * t3)
    })
}

/// River coverage of a tile, and the depth of the channels on the tile with
/// a one-pixel border.
struct Stamp {
    n: usize,
    coverage: Vec<f32>,
    channel: Vec<f32>,
}

impl Stamp {
    /// Adds a river segment of the given drawn width and channel depth (max
    /// with what is there): anti-aliased coverage, and a channel whose banks
    /// rise from the middle of the river to `CHANNEL_SPREAD` times its width.
    fn segment(&mut self, a: [f64; 2], b: [f64; 2], width: f64, depth: f32) {
        let n = self.n as f64;
        let bank = CHANNEL_SPREAD * width / 2.0;
        let r = bank.max(width / 2.0) + 1.0;
        let (x0, x1) = ((a[0].min(b[0]) - r).floor(), (a[0].max(b[0]) + r).ceil());
        let (y0, y1) = ((a[1].min(b[1]) - r).floor(), (a[1].max(b[1]) + r).ceil());
        if x1 < -1.0 || y1 < -1.0 || x0 > n || y0 > n {
            return;
        }
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len2 = (dx * dx + dy * dy).max(1e-12);
        let side = self.n + 2;
        for py in (y0.max(-1.0) as i64)..(y1.min(n + 1.0) as i64) {
            for px in (x0.max(-1.0) as i64)..(x1.min(n + 1.0) as i64) {
                let c = [px as f64 + 0.5, py as f64 + 0.5];
                let t = (((c[0] - a[0]) * dx + (c[1] - a[1]) * dy) / len2).clamp(0.0, 1.0);
                let d = ((c[0] - a[0] - t * dx).powi(2) + (c[1] - a[1] - t * dy).powi(2)).sqrt();
                let cut = depth * (1.0 - smoothstep(0.25 * width as f32, bank as f32, d as f32));
                let j = (py + 1) as usize * side + (px + 1) as usize;
                self.channel[j] = self.channel[j].max(cut);
                if (0..self.n as i64).contains(&px) && (0..self.n as i64).contains(&py) {
                    let i = py as usize * self.n + px as usize;
                    self.coverage[i] = self.coverage[i].max(rivers::band_coverage(d, width));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::generate;
    use crate::request::WorldRequest;

    fn detail_world() -> DetailWorld {
        let request = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(3),
            sea_level: Some(0.2),
            volcanic: Some(0.3),
            circumference_km: Some(40_000.0),
            axial_tilt_deg: Some(23.44),
            pressure_bar: Some(1.0),
            width: Some(512),
            height: Some(256),
            ..Default::default()
        };
        let world = generate(&request.resolve(&mut rand::rng()).unwrap().params).world;
        DetailWorld::new(LoadedWorld::from_world(&world))
    }

    #[test]
    fn bicubic_matches_the_samples_at_pixel_centres() {
        let layer: Vec<f32> = (0..16).map(|i| (i * 7 % 5) as f32).collect();
        for (i, v) in layer.iter().enumerate() {
            let (x, y) = ((i % 4) as f64 + 0.5, (i / 4) as f64 + 0.5);
            assert!((bicubic(&layer, 4, 4, x, y) - v).abs() < 1e-5);
        }
    }

    #[test]
    fn tiles_are_validated() {
        assert!(TileId { z: 0, x: 1, y: 0 }.is_valid());
        assert!(!TileId { z: 0, x: 2, y: 0 }.is_valid());
        assert!(!TileId { z: 3, x: 0, y: 8 }.is_valid());
        assert!(
            !TileId {
                z: MAX_LEVEL + 1,
                x: 0,
                y: 0
            }
            .is_valid()
        );
        // Levels whose grid would not fit in 32 bits are refused, not a panic.
        for z in [31, 32, 100, u32::MAX] {
            assert!(!TileId { z, x: 0, y: 0 }.is_valid(), "level {z}");
        }
    }

    #[test]
    fn rendering_is_deterministic() {
        let d = detail_world();
        let tile = TileId { z: 3, x: 5, y: 3 };
        assert_eq!(d.render_tile(tile), d.render_tile(tile));
    }

    #[test]
    fn coarse_tiles_agree_with_the_base_world() {
        // Level 0 tiles are 512 × 256 together: the size of this base world.
        let d = detail_world();
        let w = d.world();
        let planet = &w.params.planet;
        let (mut same_water, mut total) = (0usize, 0usize);
        for x in 0..2 {
            let img = d.render_tile(TileId { z: 0, x, y: 0 });
            for py in (0..TILE_SIZE).step_by(2) {
                for px in (0..TILE_SIZE).step_by(2) {
                    let (lon, lat) = TileId { z: 0, x, y: 0 }.lon_lat(px as f64, py as f64);
                    let (bx, by) = d.base_xy(lon, lat);
                    let i = w.index(bx as u32 % w.width, (by as u32).min(w.height - 1));
                    let base_water =
                        w.elevation.get(i) - w.params.sea_level < planet.rules.waterline;
                    let tile_water =
                        d.elevation(lon, lat, 1e9) - w.params.sea_level < planet.rules.waterline;
                    same_water += usize::from(base_water == tile_water);
                    total += 1;
                    let _ = img.get_pixel(px, py);
                }
            }
        }
        assert!(
            same_water as f64 / total as f64 > 0.99,
            "{same_water}/{total}"
        );
    }

    #[test]
    fn detail_only_adds_small_features() {
        // At a deep level the detail moves elevation by a bounded amount.
        let d = detail_world();
        let w = d.world();
        let mut worst = 0.0f32;
        for k in 0..2000 {
            let lon = (k as f64 * 0.731) % TAU;
            let lat = ((k as f64 * 0.377) % PI) - PI / 2.0;
            let (x, y) = d.base_xy(lon, lat);
            let base = bicubic(&w.elevation, w.width, w.height, x, y);
            worst = worst.max((d.elevation(lon, lat, 0.3) - base).abs());
        }
        assert!(worst < 0.05, "detail moved elevation by {worst}");
    }

    #[test]
    fn detail_never_moves_the_coast_away_from_it() {
        // Away from the coastal band, detail never turns land into water or
        // water into land: no islets, no ponds.
        let d = detail_world();
        let w = d.world();
        let waterline = w.params.sea_level + w.params.planet.rules.waterline;
        for k in 0..5000 {
            let lon = (k as f64 * 0.731) % TAU;
            let lat = ((k as f64 * 0.377) % PI) - PI / 2.0;
            let (x, y) = d.base_xy(lon, lat);
            let base = bicubic(&w.elevation, w.width, w.height, x, y);
            if (base - waterline).abs() < COAST_BAND.1 {
                continue;
            }
            for pixel_km in [5.0, 1.0, 0.3] {
                let e = d.elevation(lon, lat, pixel_km);
                assert_eq!(e >= waterline, base >= waterline, "{lon} {lat} {pixel_km}");
            }
        }
    }

    #[test]
    fn longitude_seam_is_continuous() {
        let d = detail_world();
        let (east, west) = (TileId { z: 2, x: 7, y: 1 }, TileId { z: 2, x: 0, y: 1 });
        let (a, b) = (d.render_tile(east), d.render_tile(west));
        let mut diff = 0.0;
        for y in 0..TILE_SIZE {
            let (p, q) = (a.get_pixel(TILE_SIZE - 1, y), b.get_pixel(0, y));
            diff += (0..3)
                .map(|c| (p[c] as f32 - q[c] as f32).abs())
                .sum::<f32>();
        }
        // Neighbouring columns across the seam: similar colours.
        assert!(diff / (TILE_SIZE as f32 * 3.0) < 20.0, "mean step {diff}");
    }
}
