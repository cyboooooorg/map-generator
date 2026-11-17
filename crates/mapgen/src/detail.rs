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
//! - **Poles**: beyond 60 degrees of latitude, the base layers the tiles
//!   interpolate are smoothed along their parallels towards a base row's
//!   length, as the base resolves them along the meridians, so the narrow
//!   polar columns' 16-bit stairs and edges draw no rays out of the poles.
//! - **Climate**: temperature, moisture and the other biome inputs are read
//!   at a slightly warped position, so biome borders become irregular instead
//!   of interpolated straight edges; temperature follows the new elevation.
//! - **Tiger stripes**: the stripes of a body's geysers are taken off the
//!   base elevation and albedo ([`crate::stripes::raster`]) and drawn
//!   again on each tile at its own resolution, as the small craters are:
//!   interpolated, the base map's stripes made a comb of teeth one base
//!   row long near the pole.
//! - **Biomes**: the generation's own rules and look remap, with soft
//!   borders and the vector rivers drawn at the tile's scale, without
//!   hillshade: a tile holds its colour above and its relief normals below
//!   (256 × 512), which the viewer lights.
//!
//! Tiles form an equirectangular quadtree: level `z` has `2^(z+1)` × `2^z`
//! tiles of [`TILE_SIZE`] pixels, column 0 at longitude 0 and row 0 at the
//! north pole.
//!
//! A [`DetailWorld`] keeps the base layers as stored in `raw_data/` (16 bits
//! per value, see [`Layer16`]), decoded as they are sampled, plus a byte of
//! albedo (fresh craters' ejecta and rays, format version 8), a byte of
//! flags and the water depth per pixel, and four layers smoothed along
//! the parallels on the rows beyond 60 degrees of latitude (32 bits a
//! value, see [`PolarBand`]): about 28 bytes per base pixel (0.95 GB at
//! 8192 × 4096).
use crate::biome::{Biome, BiomeInput, surface_color};
use crate::climate::temperature;
use crate::craters::{CraterField, Window, base_min_diameter_km};
use crate::export::colour::{linear_of, to_srgb};
use crate::export::ecotone::ecotone_colour_around;
use crate::export::raw_data::albedo_of;
use crate::export::raw_data::{Layer16, LoadedWorld};
use crate::export::variation::{SurfaceVariation, sphere};
use crate::export::{blend_river, relief, relief_km, water_colour, water_depths_of, with_albedo};
use crate::hex;
use crate::rivers;
use crate::stripes::Stripes;
use image::{Rgb, RgbImage};
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use std::f64::consts::{PI, TAU};

/// Tile side in pixels.
pub const TILE_SIZE: u32 = 256;
/// Version of the tile renderer. Tiles are cached by browsers for good
/// under a URL naming the world revision and this version: bump it (and
/// `TILE_RENDER_VERSION` in `web/src/lib/viewer/TileLayer.ts`) whenever
/// the colours or the shading of tiles change, or browsers keep showing
/// tiles drawn by the previous renderer.
pub const RENDER_VERSION: u32 = 11;
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
/// Smallest impact crater a tile draws, in tile pixels across (as the base
/// map, see [`crate::craters`]).
const CRATER_MIN_PIXELS: f64 = 4.0;
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
    /// The world's impact craters, to find and draw those smaller than the
    /// base map's.
    craters: CraterField,
    /// Whether the world's crust is an ice shell (see [`ice_shell`]).
    ice_shell: bool,
    /// The base layers the tiles interpolate, near the poles, smoothed
    /// along their parallels (see [`PolarBand`]): the elevation, the
    /// albedo, the roughness of the surfaces and the floodplains.
    polar_elevation: PolarBand,
    polar_albedo: PolarBand,
    polar_roughness: PolarBand,
    polar_floodplain: PolarBand,
    /// The stripes of the world's geysers, which the tiles draw at their
    /// own resolution, and the base map's raster of them, which they take
    /// off the base layers first (see [`DetailWorld::tile_stripes`]).
    stripes: Option<(Stripes, BaseStripes)>,
}

/// The stripes of a world's geysers as its base layers hold them (see
/// [`crate::stripes::raster`]): per row from `y0`, runs of consecutive
/// pixels from a column, with their height and albedo changes.
struct BaseStripes {
    width: usize,
    y0: usize,
    rows: Vec<Vec<(usize, Vec<[f32; 2]>)>>,
}

impl BaseStripes {
    /// The runs of `raster` (by index) on a base map `width` pixels wide.
    fn new(raster: &[(usize, f32, f32)], width: usize) -> Self {
        let y0 = raster.first().map_or(0, |r| r.0 / width);
        let y1 = raster.last().map_or(0, |r| r.0 / width + 1);
        let mut rows: Vec<Vec<(usize, Vec<[f32; 2]>)>> = vec![Vec::new(); y1 - y0];
        let mut last = usize::MAX;
        for &(i, e, a) in raster {
            let runs = &mut rows[i / width - y0];
            match runs.last_mut() {
                Some((_, values)) if last.wrapping_add(1) == i && i % width != 0 => {
                    values.push([e, a]);
                }
                _ => runs.push((i % width, vec![[e, a]])),
            }
            last = i;
        }
        Self { width, y0, rows }
    }

    /// The height (`k` 0) or albedo (`k` 1) change at base pixel `i`.
    #[inline]
    fn get(&self, i: usize, k: usize) -> f32 {
        let (y, x) = (i / self.width, i % self.width);
        let Some(runs) = y.checked_sub(self.y0).and_then(|r| self.rows.get(r)) else {
            return 0.0;
        };
        runs.iter()
            .find(|(x0, values)| x >= *x0 && x < x0 + values.len())
            .map_or(0.0, |(x0, values)| values[x - x0][k])
    }

    fn bytes(&self) -> usize {
        self.rows
            .iter()
            .flatten()
            .map(|(_, v)| size_of::<(usize, Vec<[f32; 2]>)>() + v.len() * size_of::<[f32; 2]>())
            .sum()
    }
}

/// A base layer with the stripes its base map drew taken off: its height
/// (`k` 0) or its albedo (`k` 1).
struct Unstriped<'a, L: ?Sized> {
    layer: &'a L,
    stripes: Option<&'a BaseStripes>,
    k: usize,
}

impl<L: Samples + ?Sized> Samples for Unstriped<'_, L> {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        let v = self.layer.value(i);
        self.stripes.map_or(v, |s| v - s.get(i, self.k))
    }
}

/// A base layer with the stripes its base map drew added back (to an
/// [`Unstriped`] one smoothed near the poles, see [`DetailWorld::base_ground`]).
struct Striped<'a, L> {
    layer: L,
    stripes: Option<&'a BaseStripes>,
    k: usize,
}

impl<L: Samples> Samples for Striped<'_, L> {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        let v = self.layer.value(i);
        self.stripes.map_or(v, |s| v + s.get(i, self.k))
    }
}

impl<L: Samples + ?Sized> Samples for &L {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        (**self).value(i)
    }
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

/// The albedo shift of the base map (see [`crate::world::World::albedo`]).
struct Albedo<'a>(&'a [u8]);

impl Samples for Albedo<'_> {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        albedo_of(self.0[i])
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
/// [`roughness`]) on a world whose crust is an ice shell or not, sampled
/// bilinearly so it changes smoothly across biome borders.
struct Roughness<'a>(&'a [Biome], bool);

impl Samples for Roughness<'_> {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        roughness(self.0[i], self.1)
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

/// [`bicubic64`] in single precision.
#[cfg(test)]
fn bicubic(layer: &(impl Samples + ?Sized), width: u32, height: u32, x: f64, y: f64) -> f32 {
    bicubic64(layer, width, height, x, y) as f32
}

/// Bicubic (uniform cubic B-spline) sample of a base layer, in double
/// precision: its slope and its curvature are continuous, so the relief
/// lit from the tiles' normals shows no crease along base pixel edges
/// (Catmull-Rom's curvature jumped there, a washboard on every slope). It
/// smooths the samples slightly rather than passing through them: a base
/// pixel's own value moves by at most a third of its difference with the
/// mean of its neighbours.
///
/// Rows beyond a pole are those as far on its other side, half a turn
/// round (the sphere continues through the pole), rather than the polar
/// row repeated: the interpolated field stays smooth through the pole,
/// where the polar row repeated held each column's value up to it, a cone
/// of slopes that grew without end towards it. In double precision, a
/// tile's pixels near a pole (metres apart along their parallel at the
/// deepest level) keep the differences their slopes are read from.
fn bicubic64(layer: &(impl Samples + ?Sized), width: u32, height: u32, x: f64, y: f64) -> f64 {
    let (w, h) = (width as i64, height as i64);
    let (x, y) = (x - 0.5, y - 0.5);
    let (x0, y0) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = (x - x0 as f64, y - y0 as f64);
    let at = |xx: i64, yy: i64| {
        let (xx, yy) = if yy < 0 {
            (xx + w / 2, -1 - yy)
        } else if yy >= h {
            (xx + w / 2, 2 * h - 1 - yy)
        } else {
            (xx, yy)
        };
        f64::from(layer.value((yy.clamp(0, h - 1) * w + xx.rem_euclid(w)) as usize))
    };
    let cubic = |p: [f64; 4], t: f64| {
        let (t2, t3, u) = (t * t, t * t * t, 1.0 - t);
        (u * u * u * p[0]
            + (3.0 * t3 - 6.0 * t2 + 4.0) * p[1]
            + (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) * p[2]
            + t3 * p[3])
            / 6.0
    };
    let rows: [f64; 4] = std::array::from_fn(|j| {
        let yy = y0 - 1 + j as i64;
        cubic(std::array::from_fn(|i| at(x0 - 1 + i as i64, yy)), fx)
    });
    cubic(rows, fy)
}

/// A base layer's rows near the poles smoothed along their parallel to a
/// base row's length, so the tiles read from it carry no structure along
/// the parallels finer than the base resolves along the meridians.
///
/// A base column narrows as `cos(lat)` towards a pole while a row keeps
/// its height: on the polar row of a 512 x 256 map a column is a 160th of
/// a row. The stored layer's 16-bit stairs, its biome borders and its
/// edges a few columns wide along a parallel are then structure hundreds
/// of times finer than a row, which the interpolation spreads over two
/// rows radially: steep east-west slopes in the tiles' normals, rays of
/// light and shade out of the pole. This is the "polar filter" of
/// latitude-longitude grids, which removes from the rows near the poles
/// the zonal waves shorter than the grid's meridional spacing (Arakawa
/// and Lamb 1977, Methods in Computational Physics 17, 173-265,
/// doi:10.1016/B978-0-12-460817-7.50009-4): a field is sampled no finer
/// than it is resolved (Shannon 1949, Proc. IRE 37, 10-21,
/// doi:10.1109/JRPROC.1949.232969).
///
/// Along a meridian the tiles see the base through a cubic B-spline one
/// row wide (see [`bicubic64`]), a smoothing of variance a third of a row
/// squared, `k² / 3` columns squared where `k = 1 / cos(lat)` columns make
/// a row; along a parallel, through one a column wide, `1 / 3`. A box `w`
/// columns wide (cutting the columns at its ends for their share) adds
/// about `(w² - 1) / 12` (exactly at odd widths, a little more between
/// them), so four in a row (see [`POLAR_PASSES`]) bring the parallel's to
/// `w² / 3`. Boxes of `w² = k² - 3 = tan²(lat) - 2` columns
/// (see [`polar_box`]) are one column, the row itself, at 60 degrees:
/// rows nearer the equator are not stored and read as they are, so the
/// tiles there are unchanged. Nowhere is the parallel's smoothing below
/// half the meridian's (in standard deviation), as the base itself has it
/// at 60 degrees, and it tends to the meridian's at the pole: 81 % of it
/// at 70 degrees, 95 % at 80.
struct PolarBand {
    width: usize,
    height: usize,
    /// Rows smoothed at each pole.
    rows: usize,
    /// The first `rows` rows, then the last `rows`, row-major.
    values: Vec<f32>,
}

/// Width in base columns of the boxes [`PolarBand`] smooths row `y` of a
/// map `height` rows high with: `sqrt(tan²(lat) - 2)` at the row's
/// centre, one column (the row itself) at 60 degrees of latitude.
fn polar_box(y: usize, height: usize) -> f64 {
    let colat = (y as f64 + 0.5) / height as f64 * PI;
    let tan = 1.0 / colat.min(PI - colat).tan();
    (tan * tan - 2.0).max(0.0).sqrt()
}

/// Boxes [`PolarBand`] smooths a row with in turn.
const POLAR_PASSES: usize = 4;

/// Base layers a [`DetailWorld`] smooths near the poles (see
/// [`PolarBand`]): the elevation, the albedo, the roughness and the
/// floodplains.
const POLAR_LAYERS: usize = 4;

/// Bytes a [`DetailWorld`] of a `width` x `height` base map holds for its
/// layers smoothed near the poles (see [`PolarBand`]): 32 bits a value on
/// the rows beyond 60 degrees of latitude, a third of them, about 5.3
/// bytes a base pixel.
pub fn polar_band_bytes(width: u32, height: u32) -> usize {
    POLAR_LAYERS * 2 * polar_rows(height as usize) * width as usize * size_of::<f32>()
}

/// Rows [`PolarBand`] smooths at each pole of a map `height` rows high:
/// those whose box is wider than a column.
fn polar_rows(height: usize) -> usize {
    (0..height / 2)
        .take_while(|&y| polar_box(y, height) > 1.0)
        .count()
}

impl PolarBand {
    fn new(layer: &(impl Samples + Sync + ?Sized), width: u32, height: u32) -> Self {
        let (w, h) = (width as usize, height as usize);
        let rows = polar_rows(h);
        let mut values = vec![0.0f32; 2 * rows * w];
        let ys: Vec<usize> = (0..rows).chain(h - rows..h).collect();
        values
            .par_chunks_mut(w.max(1))
            .zip(ys)
            .for_each(|(out, y)| {
                let mut row: Vec<f64> = (0..w).map(|x| f64::from(layer.value(y * w + x))).collect();
                for _ in 0..POLAR_PASSES {
                    row = box_filter(&row, polar_box(y, h));
                }
                for (out, v) in out.iter_mut().zip(row) {
                    *out = v as f32;
                }
            });
        Self {
            width: w,
            height: h,
            rows,
            values,
        }
    }

    /// No row smoothed: the layer as it is.
    fn none(width: u32, height: u32) -> Self {
        Self {
            width: width as usize,
            height: height as usize,
            rows: 0,
            values: Vec::new(),
        }
    }

    fn bytes(&self) -> usize {
        self.values.len() * size_of::<f32>()
    }
}

/// Mean of the periodic `row` over a box `wide` columns centred on each
/// column, the columns it cuts at its ends counted for their share: the
/// row itself where `wide` is one column or less.
fn box_filter(row: &[f64], wide: f64) -> Vec<f64> {
    let w = row.len();
    if wide <= 1.0 {
        return row.to_vec();
    }
    // Integral of the row from the west edge of column 0 to `t` columns.
    let mut prefix = Vec::with_capacity(w + 1);
    let mut sum = 0.0;
    prefix.push(0.0);
    for &v in row {
        sum += v;
        prefix.push(sum);
    }
    let integral = |t: f64| {
        let turns = (t / w as f64).floor();
        let r = t - turns * w as f64;
        let j = (r.floor() as usize).min(w - 1);
        turns * sum + prefix[j] + (r - j as f64) * row[j]
    };
    (0..w)
        .map(|c| {
            let centre = c as f64 + 0.5;
            (integral(centre + wide / 2.0) - integral(centre - wide / 2.0)) / wide
        })
        .collect()
}

/// A base layer read through its [`PolarBand`].
struct Banded<'a, L> {
    layer: L,
    band: &'a PolarBand,
}

impl<L: Samples> Samples for Banded<'_, L> {
    #[inline]
    fn value(&self, i: usize) -> f32 {
        let b = self.band;
        let y = i / b.width;
        if y < b.rows {
            b.values[i]
        } else if y >= b.height - b.rows {
            b.values[i - (b.height - 2 * b.rows) * b.width]
        } else {
            self.layer.value(i)
        }
    }
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
) -> f64 {
    let mut sum = 0.0;
    let (mut wavelength, mut amplitude) = (first_km, 1.0);
    while wavelength >= last_km {
        let f = radius_km / wavelength;
        sum += noise.get([p[0] * f, p[1] * f, p[2] * f]) * amplitude;
        wavelength /= 2.0;
        amplitude *= gain;
    }
    sum
}

/// Share of the height detail a biome's surface gets: snow, ice and lava
/// are smooth, sand and ash nearly so; rock and vegetation keep it all.
/// On a world whose crust is an ice shell (`ice_shell`, Europa, Titan),
/// its frozen ground is that shell, its bedrock, not a cover of snow or
/// glacier smoothing a rocky relief: it keeps it all too (the base relief
/// already holds ice's weakness, see [`crate::generation::relief_share`]).
/// Sea ice stays smooth.
fn roughness(biome: Biome, ice_shell: bool) -> f32 {
    match biome {
        Biome::Snow | Biome::IceCap | Biome::GlacialPlain if ice_shell => 1.0,
        Biome::Snow | Biome::IceCap | Biome::GlacialPlain | Biome::SeaIce => 0.1,
        Biome::Volcano | Biome::LavaField | Biome::MagmaSea => 0.05,
        Biome::Desert | Biome::DustPlain | Biome::Beach | Biome::AshLand => 0.4,
        _ => 1.0,
    }
}

/// Whether the crust of the world `p` is an ice shell whose frozen ground
/// is its bedrock (see [`roughness`]): a world with a model (a planet or a
/// round moon) whose crust is ice ([`crate::generation::icy`]). A small
/// moon, without a model, keeps the smooth ice of its look.
fn ice_shell(p: &crate::world::WorldParams) -> bool {
    p.model.is_some() && crate::generation::icy(p)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl DetailWorld {
    /// Whether the world is a giant planet or a steam world: cloud bands,
    /// no ground to detail (it has no detail tiles).
    pub fn is_giant(&self) -> bool {
        self.world.params.renders_as_deck()
    }

    pub fn new(world: LoadedWorld) -> Self {
        Self::at_epoch(world, 0.0)
    }

    /// A world shown at `epoch_ga` (see [`crate::epoch`]): `world` holds
    /// its layers at that epoch, and the tiles draw the small craters
    /// formed by then.
    pub fn at_epoch(world: LoadedWorld, epoch_ga: f64) -> Self {
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
        let look = &world.params.look;
        let waterline = world.params.open_waterline();
        let river = look.remap(Biome::River);
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
        let stripes = Stripes::of(&world.params).map(|stripes| {
            let raster = crate::stripes::raster(&world.params.grid(), &world.params);
            (stripes, BaseStripes::new(&raster, world.width as usize))
        });
        // The band is smoothed without the base map's stripes, which the
        // tiles draw again at their own resolution (see `tile_stripes`).
        let base_stripes = stripes.as_ref().map(|s| &s.1);
        let polar_elevation = PolarBand::new(
            &Unstriped {
                layer: &world.elevation,
                stripes: base_stripes,
                k: 0,
            },
            world.width,
            world.height,
        );
        let polar_albedo = if world.albedo.is_empty() {
            PolarBand::none(world.width, world.height)
        } else {
            PolarBand::new(
                &Unstriped {
                    layer: &Albedo(&world.albedo),
                    stripes: base_stripes,
                    k: 1,
                },
                world.width,
                world.height,
            )
        };
        let ice_shell = ice_shell(&world.params);
        let polar_roughness = PolarBand::new(
            &Roughness(&world.biomes, ice_shell),
            world.width,
            world.height,
        );
        let polar_floodplain = PolarBand::new(&world.floodplain, world.width, world.height);
        Self {
            stripes,
            polar_elevation,
            polar_albedo,
            polar_roughness,
            polar_floodplain,
            craters: CraterField::context_at(&world.params, epoch_ga),
            ice_shell,
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
            + self.polar_elevation.bytes()
            + self.polar_albedo.bytes()
            + self.polar_roughness.bytes()
            + self.polar_floodplain.bytes()
            + self.stripes.as_ref().map_or(0, |s| s.1.bytes())
    }

    /// The base elevation as the tiles interpolate it: without the
    /// stripes the base map drew (see [`Self::unstriped`]), smoothed near
    /// the poles (see [`PolarBand`]).
    fn base_elevation(&self) -> Banded<'_, Unstriped<'_, Layer16>> {
        Banded {
            layer: self.unstriped(&self.world.elevation, 0),
            band: &self.polar_elevation,
        }
    }

    /// The base elevation of the ground, its stripes included: as
    /// [`Self::base_elevation`], with the base map's stripes added back.
    fn base_ground(&self) -> Striped<'_, Banded<'_, Unstriped<'_, Layer16>>> {
        Striped {
            layer: self.base_elevation(),
            stripes: self.stripes.as_ref().map(|s| &s.1),
            k: 0,
        }
    }

    /// Base pixel coordinates of a longitude and latitude.
    fn base_xy(&self, lon: f64, lat: f64) -> (f64, f64) {
        (
            lon / TAU * self.world.width as f64,
            (0.5 - lat / PI) * self.world.height as f64,
        )
    }

    /// The base elevation (`k` 0) or albedo (`k` 1) without the stripes
    /// the base map drew, which the tiles draw at their own resolution.
    fn unstriped<'a, L: Samples + ?Sized>(&'a self, layer: &'a L, k: usize) -> Unstriped<'a, L> {
        Unstriped {
            layer,
            stripes: self.stripes.as_ref().map(|s| &s.1),
            k,
        }
    }

    /// The stripes of the world's geysers drawn on a tile with its
    /// one-pixel border (row-major, `TILE_SIZE + 2` wide) at the tile's
    /// own resolution ([`crate::stripes::drawn_width_km`] of its pixels,
    /// averaged over each one, as the base map draws them on its own):
    /// the height change, in elevation units, and the albedo shift; `None`
    /// when they are nowhere near the tile.
    fn tile_stripes(&self, tile: TileId, pixel_km: f64) -> Option<Vec<(f32, f32)>> {
        let (stripes, _) = self.stripes.as_ref()?;
        let side = TILE_SIZE as usize + 2;
        // Not clamped: a point beyond a pole (a border pixel, or a
        // pixel's footprint on the polar row) lies on its other side, as
        // `tile_elevation` reads the ground there.
        let at = |x: f64, y: f64| {
            let (lon, lat) = tile.lon_lat(x, y);
            sphere(lon, lat, 1.0)
        };
        let angle = |a: [f64; 3], b: [f64; 3]| {
            (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
                .clamp(-1.0, 1.0)
                .acos()
        };
        let half = TILE_SIZE as f64 / 2.0 - 0.5;
        let centre = at(half, half);
        let radius = [
            (-1.0, -1.0),
            (side as f64, -1.0),
            (-1.0, side as f64),
            (side as f64, side as f64),
        ]
        .iter()
        .map(|&(x, y)| angle(centre, at(x, y)))
        .fold(0.0f64, f64::max);
        let (width, footprint) = (
            crate::stripes::drawn_width_km(pixel_km),
            pixel_km * 2f64.sqrt(),
        );
        if angle(stripes.pole, centre) - radius > stripes.cap_rad(width, footprint) {
            return None;
        }
        Some(
            (0..side * side)
                .into_par_iter()
                .map(|i| {
                    let (px, py) = ((i % side) as f64 - 1.0, (i / side) as f64 - 1.0);
                    stripes.pixel(at, px, py, width, footprint)
                })
                .collect(),
        )
    }

    fn base_px_km(&self) -> f64 {
        self.world.params.circumference_km as f64 / self.world.width as f64
    }

    /// [`Self::elevation64`] in single precision.
    #[cfg(test)]
    fn elevation(&self, lon: f64, lat: f64, pixel_km: f64) -> f32 {
        self.elevation64(lon, lat, pixel_km) as f32
    }

    /// Detailed elevation at a point, for a tile whose pixels are
    /// `pixel_km`, in double precision (see [`bicubic64`]).
    fn elevation64(&self, lon: f64, lat: f64, pixel_km: f64) -> f64 {
        let w = &self.world;
        let (x, y) = self.base_xy(lon, lat);
        let elevation = self.base_elevation();
        let base64 = bicubic64(&elevation, w.width, w.height, x, y);
        let base = base64 as f32;
        let first = 2.0 * self.base_px_km();
        let last = FINEST_PX * pixel_km;
        if last > first {
            return base64;
        }
        let p = sphere(lon, lat, 1.0);
        let radius = w.params.circumference_km as f64 / TAU;
        let waterline = w.params.sea_level + w.params.look.rules.waterline;
        let dry = w.params.look.rules.dry;

        // Near the coast, read the base elevation at a position displaced
        // sideways by every detail octave: the coastline gets capes and bays
        // at each zoom level, and stays one connected line.
        // A world without surface liquid has no coast.
        let coastal = if dry {
            0.0
        } else {
            1.0 - smoothstep(COAST_BAND.0, COAST_BAND.1, (base - waterline).abs())
        };
        let base64 = if coastal > 0.0 {
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
            bicubic64(&elevation, w.width, w.height, x + dx, y + dy)
        } else {
            base64
        };
        let base = base64 as f32;

        // Height detail: calmer on plains (and at every zoom), stronger and
        // 1/f on high ground, none at the waterline.
        let above = base - waterline;
        let high = (above / 0.3).clamp(0.0, 1.0);
        // Snow, ice, lava and sand are smooth surfaces (see `roughness`);
        // small bodies are flattened as their base relief is.
        let amplitude = DETAIL_AMPLITUDE
            * crate::generation::relief_share(&w.params)
            * (1.0 + MOUNTAIN_BOOST * high)
            * bilinear(
                &Banded {
                    layer: &Roughness(&w.biomes, self.ice_shell),
                    band: &self.polar_roughness,
                },
                w.width,
                w.height,
                x,
                y,
            );
        let gain = PLAIN_GAIN + (MOUNTAIN_GAIN - PLAIN_GAIN) * high as f64;
        let shore = if dry {
            1.0
        } else {
            smoothstep(0.0, SHORE_FADE, above.abs())
        };
        // Floodplains stay flat valley floors.
        let floodplain = Banded {
            layer: &w.floodplain,
            band: &self.polar_floodplain,
        };
        let floor = 1.0 - FLOODPLAIN_CALM * bilinear(&floodplain, w.width, w.height, x, y);
        base64
            + f64::from(amplitude * shore * floor)
                * band_limited(&self.noise, p, radius, first, last, gain)
    }

    /// Elevation of a tile with a one-pixel border, for slopes (row-major,
    /// `TILE_SIZE + 2` wide): the detailed elevation, the impact craters too
    /// small for the base map, and the rivers' channels (`channel`, same
    /// layout). Also the same without those craters, which biomes follow: a
    /// small crater changes the relief, not the ground it is dug into.
    /// The first in double precision, for the slopes near the poles (see
    /// [`bicubic64`]).
    fn tile_elevation(&self, tile: TileId, pixel_km: f64, channel: &[f32]) -> (Vec<f64>, Vec<f32>) {
        let w = &self.world;
        let side = TILE_SIZE as usize + 2;
        let waterline = w.params.open_waterline();
        let craters = self.crater_relief(tile, pixel_km);
        let stripes = self.tile_stripes(tile, pixel_km);
        (0..side * side)
            .into_par_iter()
            .map(|i| {
                let (px, py) = ((i % side) as f64 - 1.0, (i / side) as f64 - 1.0);
                let (lon, lat) = tile.lon_lat(px, py);
                // A border pixel beyond a pole lies on its other side.
                let (lon, lat) = if lat.abs() > PI / 2.0 {
                    (lon + PI, lat.signum() * PI - lat)
                } else {
                    (lon, lat)
                };
                let ground = self.elevation64(lon, lat, pixel_km)
                    + stripes.as_ref().map_or(0.0, |s| f64::from(s[i].0));
                let waterline = f64::from(waterline);
                let e = match &craters {
                    // Craters never dig the land below the sea.
                    Some(relief) if ground >= waterline => {
                        (ground + f64::from(relief[i])).max(waterline + 1e-6)
                    }
                    Some(relief) => ground + f64::from(relief[i]),
                    None => ground,
                };
                // The river's channel and banks, never below the sea.
                let channelled = |e: f64| {
                    if e >= waterline {
                        (e - f64::from(channel[i])).max(waterline + 1e-6)
                    } else {
                        e
                    }
                };
                (channelled(e), channelled(ground) as f32)
            })
            .unzip()
    }

    /// Relief of the impact craters between [`CRATER_MIN_PIXELS`] tile
    /// pixels and the base map's smallest (which the base elevation holds)
    /// on a tile with its border, or `None` when there is none.
    fn crater_relief(&self, tile: TileId, pixel_km: f64) -> Option<Vec<f32>> {
        let w = &self.world;
        let min_km = CRATER_MIN_PIXELS * pixel_km;
        let max_km = base_min_diameter_km(&w.params.grid());
        if min_km >= max_km {
            return None;
        }
        let side = TILE_SIZE as usize + 2;
        let (cols, rows) = tile.grid();
        let (lon0, lat0) = tile.lon_lat(-1.0, -1.0);
        let window = Window {
            width: side,
            height: side,
            lat0,
            dlat: PI / (rows as f64 * TILE_SIZE as f64),
            lon0,
            dlon: TAU / (cols as f64 * TILE_SIZE as f64),
        };
        // The cap around the tile's centre that holds its corners.
        let (lon_c, lat_c) =
            tile.lon_lat(TILE_SIZE as f64 / 2.0 - 0.5, TILE_SIZE as f64 / 2.0 - 0.5);
        let centre = sphere(lon_c, lat_c, 1.0);
        let angle = [
            (-1.0, -1.0),
            (side as f64, -1.0),
            (-1.0, side as f64),
            (side as f64, side as f64),
        ]
        .iter()
        .map(|&(px, py)| {
            let (lon, lat) = tile.lon_lat(px, py);
            let p = sphere(lon, lat.clamp(-PI / 2.0, PI / 2.0), 1.0);
            (p[0] * centre[0] + p[1] * centre[1] + p[2] * centre[2])
                .clamp(-1.0, 1.0)
                .acos()
        })
        .fold(0.0f64, f64::max);
        let craters = self.craters.near(centre, angle, min_km, max_km);
        if craters.is_empty() {
            return None;
        }
        let ground = |c: &crate::craters::Crater| {
            let lat = c.centre[2].clamp(-1.0, 1.0).asin();
            let lon = c.centre[1].atan2(c.centre[0]).rem_euclid(TAU);
            let (x, y) = self.base_xy(lon, lat);
            bilinear(&self.base_ground(), w.width, w.height, x, y)
        };
        // The hillshade exaggerates relief about 60 times (`RELIEF_KM`), for
        // the gentle slopes of the base map: small crater walls would turn
        // vertical. Their relief shrinks with their size, from the base
        // map's at its smallest crater down to a tenth.
        let gain = |c: &crate::craters::Crater| ((c.diameter_km / max_km) as f32).clamp(0.1, 1.0);
        let mut relief = vec![0.0; side * side];
        self.craters
            .add_relief(&craters, &window, ground, gain, &mut relief);
        Some(relief)
    }

    /// Renders one tile as an RGB image.
    pub fn render_tile(&self, tile: TileId) -> RgbImage {
        let w = &self.world;
        let look = &w.params.look;
        let (cols, _) = tile.grid();
        let n = TILE_SIZE as usize;
        let pixel_km = w.params.circumference_km as f64 / (cols as f64 * TILE_SIZE as f64);

        let side = n + 2;
        let (coverage, channel) = self.river_coverage(tile, pixel_km);
        let (elevation, surface) = self.tile_elevation(tile, pixel_km, &channel);
        let stripes = self.tile_stripes(tile, pixel_km);

        let river_colour = linear_of(surface_color(look.remap(Biome::River)));
        let lapse = w.lapse_rate;
        let warp_km = CLIMATE_WARP_PX * self.base_px_km();
        let radius = w.params.circumference_km as f64 / TAU;

        let pixels: Vec<([u8; 3], [u8; 3])> = (0..n * n)
            .into_par_iter()
            .map(|i| {
                let (px, py) = (i % n, i / n);
                let (lon, lat) = tile.lon_lat(px as f64, py as f64);
                // Biomes follow the ground without the tile's small craters;
                // the hillshade shows them.
                let e = surface[(py + 1) * side + px + 1];
                let relief = (e - w.params.sea_level).clamp(-1.0, 1.0);
                let above = relief - look.rules.waterline;
                // Height above open water: `above` but on a dry world, which
                // has none.
                let above_sea = relief - look.rules.open_water();

                // Climate at a warped position: irregular biome borders.
                let p = sphere(lon, lat, 1.0);
                let f = radius / (4.0 * warp_km);
                let wx = self.warp_noise.get([p[0] * f, p[1] * f, p[2] * f]);
                let wy = self
                    .warp_noise
                    .get([p[0] * f + 19.1, p[1] * f - 7.3, p[2] * f + 3.7]);
                let (bx, by) = self.base_xy(lon, lat);
                let (cx, cy) = (bx + wx * CLIMATE_WARP_PX, by + wy * CLIMATE_WARP_PX);
                let sample = |layer: &dyn Samples| bilinear(layer, w.width, w.height, cx, cy);
                let base_above =
                    sample(&self.base_ground()) - w.params.sea_level - look.rules.waterline;
                // Temperature follows the detailed elevation (lapse rate).
                let t = sample(&w.temperature) + temperature(0.0, above, lapse)
                    - temperature(0.0, base_above, lapse);
                // Lakes: smooth shores from the base lake mask.
                let lake = sample(&Lakes(&self.flags)) > 0.5 && above_sea >= 0.0;
                let input = BiomeInput {
                    elevation: relief,
                    moisture: sample(&w.moisture),
                    temperature: t,
                    seasonality: w.seasons.at(
                        (cy.max(0.0) as u32).min(w.height - 1),
                        above_sea >= 0.0,
                        sample(&w.coast_distance_km),
                    ),
                    volcanic_zone: sample(&w.volcanic_zone),
                    coast_distance_km: sample(&w.coast_distance_km),
                    lake,
                    river: false,
                    highland_offset: sample(&w.highland_offset),
                };
                let dry = look.rules.choose(input);
                let biome = look.remap(dry);
                let (mut colour, normal) = if biome.is_water() {
                    // Seas from the detailed elevation; lakes from the base
                    // depth below their surface.
                    let depth = if above_sea < 0.0 {
                        -above_sea
                    } else {
                        sample(&self.depths)
                    };
                    let colour =
                        to_srgb(
                            self.variation
                                .sea(biome, water_colour(biome, depth), lon, lat),
                        );
                    (colour, relief::FLAT)
                } else {
                    // Unshaded: the viewer lights the relief with the
                    // normals of the lower half.
                    let mut normal = self.normal(&elevation, side, px + 1, py + 1, pixel_km, lat);
                    if dry.is_lava() {
                        normal = relief::soften(normal, LAVA_SHADE);
                    }
                    let colour = self.variation.apply(
                        ecotone_colour_around(look, input, Some(biome)),
                        lon,
                        lat,
                    );
                    let colour = if dry.is_lava() || w.albedo.is_empty() {
                        colour
                    } else {
                        // Fresh craters' ejecta and rays, at the base
                        // resolution; the stripes at the tile's.
                        let albedo = Albedo(&w.albedo);
                        let albedo = Banded {
                            layer: self.unstriped(&albedo, 1),
                            band: &self.polar_albedo,
                        };
                        let a = bilinear(&albedo, w.width, w.height, bx, by)
                            + stripes
                                .as_ref()
                                .map_or(0.0, |s| s[(py + 1) * side + px + 1].1);
                        with_albedo(colour, a)
                    };
                    (colour, relief::encode_normal(normal))
                };
                let c = coverage[i];
                if c > 0.0 && !look.rules.frozen(input) && !biome.is_water() && !dry.is_lava() {
                    colour = blend_river(colour, river_colour, c);
                }
                (colour.map(|c| c.clamp(0.0, 255.0) as u8), normal)
            })
            .collect();

        RgbImage::from_fn(TILE_SIZE, 2 * TILE_SIZE, |x, y| {
            let (colour, normal) = pixels[((y % TILE_SIZE) * TILE_SIZE + x) as usize];
            Rgb(if y < TILE_SIZE { colour } else { normal })
        })
    }

    /// The relief normal (east, north, up) of a tile pixel, at the base
    /// map's exaggeration, from the tile's elevation with its one-pixel
    /// border: neighbouring tiles share their edge pixels, so their normals
    /// meet without a seam.
    ///
    /// The run towards the east is the pixel's true width on its parallel,
    /// down to the pixels next to the poles (whose centres lie half a pixel
    /// from them), as on the base map (see [`relief::slopes`]): a floor at
    /// 0.05 of the equator's shrank the slopes along the parallels within
    /// 2.9 degrees of the poles, a cone in the light. In double precision:
    /// at the deepest level, those pixels are metres apart.
    fn normal(
        &self,
        elevation: &[f64],
        side: usize,
        x: usize,
        y: usize,
        pixel_km: f64,
        lat: f64,
    ) -> [f32; 3] {
        let relief = f64::from(relief_km(self.world().params.circumference_km));
        let at = |x: usize, y: usize| elevation[y * side + x] * relief;
        let dx_km = pixel_km * lat.cos();
        let dz_east = (at(x + 1, y) - at(x - 1, y)) / (2.0 * dx_km);
        let dz_north = (at(x, y - 1) - at(x, y + 1)) / (2.0 * pixel_km);
        relief::normal_of(dz_east as f32, dz_north as f32)
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

    fn detail_world() -> DetailWorld {
        let params = crate::world::WorldParams {
            sea_level: 0.2,
            volcanic_intensity: 0.3,
            circumference_km: 40_000.0,
            ..crate::generation::tests::preset_params("terran", 3, 512, 256)
        };
        let world = generate(&params).world;
        DetailWorld::new(LoadedWorld::from_world(&world))
    }

    /// A small airless, heavily cratered world.
    fn cratered_world() -> DetailWorld {
        let planet = "mercury-like";
        let params = crate::world::WorldParams {
            sea_level: 0.15,
            surface_age_ga: 4.0,
            orbit_au: 1.0,
            eccentricity: 0.0,
            // Its spin holds at its own orbit only.
            locked_to_star: None,
            resonance_to_star: None,
            moon: false,
            day_hours: None,
            liquid_poleward_deg: None,
            asked_moons: Vec::new(),
            moon_sizes: Vec::new(),
            ..crate::generation::tests::preset_params(planet, 5, 512, 256)
        };
        let world = generate(&params).world;
        DetailWorld::new(LoadedWorld::from_world(&world))
    }

    #[test]
    fn tiles_draw_craters_too_small_for_the_base_map() {
        let d = cratered_world();
        // Base pixels are 30 km: the base map has no crater under 120 km.
        let tile = TileId { z: 5, x: 20, y: 12 };
        let pixel_km = d.world().params.circumference_km as f64 / (64.0 * TILE_SIZE as f64);
        let relief = d.crater_relief(tile, pixel_km).expect("craters");
        let deepest = relief.iter().fold(0.0f32, |m, &v| m.min(v)) * crate::craters::ELEVATION_KM;
        assert!(deepest < -0.5, "deepest cavity {deepest} km");
        // Coarse tiles leave the craters to the base elevation.
        assert!(d.crater_relief(TileId { z: 0, x: 0, y: 0 }, 1e3).is_none());
    }

    #[test]
    fn neighbouring_tiles_agree_on_craters() {
        let d = cratered_world();
        let (z, x, y) = (5, 20, 12);
        let pixel_km = d.world().params.circumference_km as f64 / (64.0 * TILE_SIZE as f64);
        let side = TILE_SIZE as usize + 2;
        let no_rivers = vec![0.0; side * side];
        let (a, _) = d.tile_elevation(TileId { z, x, y }, pixel_km, &no_rivers);
        let (b, _) = d.tile_elevation(TileId { z, x: x + 1, y }, pixel_km, &no_rivers);
        // Column 256 of the first tile is column 0 of the next one.
        for row in 0..side {
            let (ea, eb) = (a[row * side + side - 1], b[row * side + 1]);
            assert!((ea - eb).abs() < 1e-5, "row {row}: {ea} vs {eb}");
        }
    }

    #[test]
    fn bicubic_keeps_smooth_samples() {
        // A plane is kept exactly; a sample is pulled towards its
        // neighbours by at most a third of their difference.
        let plane: Vec<f32> = (0..64)
            .map(|i| (i % 8) as f32 * 0.1 + (i / 8) as f32 * 0.05)
            .collect();
        for (i, v) in plane
            .iter()
            .enumerate()
            .filter(|(i, _)| (2..6).contains(&(i % 8)) && (2..6).contains(&(i / 8)))
        {
            let (x, y) = ((i % 8) as f64 + 0.5, (i / 8) as f64 + 0.5);
            assert!((bicubic(&plane, 8, 8, x, y) - v).abs() < 1e-5);
        }
        let spike: Vec<f32> = (0..64).map(|i| if i == 27 { 1.0 } else { 0.0 }).collect();
        let at = bicubic(&spike, 8, 8, 3.5, 3.5);
        assert!((at - 4.0 / 9.0).abs() < 1e-5, "{at}");
    }

    /// The interpolated base continues through a pole: a plane through the
    /// centre (`x`, the unit vector's first component), sampled at the base
    /// pixels, is read back near the pole within a hundredth of a row's
    /// rise. Repeating the polar row held each column's value up to the
    /// pole (an error of half a row's rise there), a cone whose slopes grew
    /// without end towards it.
    #[test]
    fn the_interpolated_base_continues_through_the_poles() {
        let (w, h) = (64u32, 32u32);
        let grid = crate::geo::Grid {
            width: w,
            height: h,
            circumference_km: 40_000.0,
        };
        let plane: Vec<f32> = (0..w * h)
            .map(|i| grid.unit_vector_at(f64::from(i % w), f64::from(i / w))[0] as f32)
            .collect();
        let row = PI / f64::from(h);
        let mut worst = 0.0f64;
        for k in 0..200 {
            let lon = f64::from(k) * 0.173 % TAU;
            // Within a row of either pole.
            let colat = f64::from(k % 20) / 20.0 * row;
            for lat in [PI / 2.0 - colat, colat - PI / 2.0] {
                let (x, y) = (lon / TAU * f64::from(w), (0.5 - lat / PI) * f64::from(h));
                let want = lat.cos() * lon.cos();
                worst = worst.max((bicubic64(&plane, w, h, x, y) - want).abs() / row);
            }
        }
        assert!(worst < 0.01, "{worst} of a row's rise");
    }

    /// A tile's normals keep the slopes of the ground up to the pole: on a
    /// plane through the centre, at the deepest level, the slope towards
    /// the east of the polar tile's pixels is the plane's. With the east-
    /// west run floored at 0.05 of the equator's (2.9 degrees) and the
    /// elevation in single precision, it was off by the whole slope.
    #[test]
    fn tile_normals_keep_a_planes_slopes_up_to_the_pole() {
        let d = detail_world();
        let radius = f64::from(d.world().params.circumference_km) / TAU;
        let relief = f64::from(relief_km(d.world().params.circumference_km));
        // At longitude 90 degrees, where the plane rises along the parallels.
        let tile = TileId {
            z: MAX_LEVEL,
            x: 1 << (MAX_LEVEL - 1),
            y: 0,
        };
        let (cols, _) = tile.grid();
        let pixel_km = d.world().params.circumference_km as f64 / (cols as f64 * TILE_SIZE as f64);
        let side = TILE_SIZE as usize + 2;
        // A gentle tilt (a slope of 0.2 at the exaggeration), on ground
        // standing at 0.3 as land does.
        let a = 0.2 * radius / relief;
        let elevation: Vec<f64> = (0..side * side)
            .map(|i| {
                let (lon, lat) = tile.lon_lat((i % side) as f64 - 1.0, (i / side) as f64 - 1.0);
                0.3 + a * lat.cos() * lon.cos()
            })
            .collect();
        let mut worst = 0.0f64;
        for y in 1..40 {
            for x in (1..side - 1).step_by(5) {
                let (lon, lat) = tile.lon_lat(x as f64 - 1.0, y as f64 - 1.0);
                let n = d.normal(&elevation, side, x, y, pixel_km, lat);
                let east = -f64::from(n[0]) / f64::from(n[2]);
                let want = relief * a * -lon.sin() / radius;
                worst = worst.max((east - want).abs() / 0.2);
            }
        }
        assert!(worst < 0.01, "{worst}");
    }

    /// The curvature of the interpolated base is continuous across the base
    /// pixels: a jump there shows as a crease at every base pixel in the
    /// lit relief of a tile (a washboard on every slope).
    #[test]
    fn interpolated_base_curvature_is_continuous_across_pixels() {
        let layer: Vec<f32> = (0..64)
            .map(|i| ((i * 37 % 17) as f32 * 0.37).sin())
            .collect();
        // Steps large enough for f32 rounding not to dominate the second
        // difference.
        let (h, eps) = (1e-2, 2e-2);
        let curvature = |x: f64| {
            let f = |x: f64| f64::from(bicubic(&layer, 16, 4, x, 1.5));
            (f(x + h) - 2.0 * f(x) + f(x - h)) / (h * h)
        };
        let mut worst = 0.0f64;
        for k in 3..12 {
            // Base sample k sits at x = k + 0.5: the joint of two cubic pieces.
            let c = k as f64 + 0.5;
            worst = worst.max((curvature(c + eps) - curvature(c - eps)).abs());
        }
        assert!(worst < 0.05, "curvature jumps by {worst} at a base pixel");
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
    fn tiles_stack_their_colour_over_their_normals() {
        let d = detail_world();
        let img = d.render_tile(TileId { z: 3, x: 5, y: 3 });
        assert_eq!(img.dimensions(), (TILE_SIZE, 2 * TILE_SIZE));
        // The lower half holds unit normals facing out of the ground.
        for y in (TILE_SIZE..2 * TILE_SIZE).step_by(17) {
            for x in (0..TILE_SIZE).step_by(13) {
                let n = crate::export::relief::decode_normal(img.get_pixel(x, y).0);
                assert!(n[2] > 0.0, "{x} {y}: {n:?}");
            }
        }
    }

    /// Mean step between two columns of normals, per channel.
    fn column_step(a: &RgbImage, ax: u32, b: &RgbImage, bx: u32) -> f32 {
        let mut sum = 0.0;
        for y in TILE_SIZE..2 * TILE_SIZE {
            let (p, q) = (a.get_pixel(ax, y), b.get_pixel(bx, y));
            sum += (0..3)
                .map(|c| (p[c] as f32 - q[c] as f32).abs())
                .sum::<f32>();
        }
        sum / (TILE_SIZE as f32 * 3.0)
    }

    #[test]
    fn tile_normals_agree_across_tile_borders() {
        // The normals across the border of two tiles step no more than the
        // normals one pixel apart inside a tile: the relief has no seam.
        let d = detail_world();
        let (left, right) = (TileId { z: 3, x: 5, y: 3 }, TileId { z: 3, x: 6, y: 3 });
        let (a, b) = (d.render_tile(left), d.render_tile(right));
        let across = column_step(&a, TILE_SIZE - 1, &b, 0);
        let inside = column_step(&a, TILE_SIZE - 2, &a, TILE_SIZE - 1);
        assert!(
            across <= 1.5 * inside + 2.0,
            "across {across}, inside {inside}"
        );
    }

    #[test]
    fn coarse_tiles_agree_with_the_base_world() {
        // Level 0 tiles are 512 × 256 together: the size of this base world,
        // as the tiles read it (smoothed along the parallels beyond 60
        // degrees, see `PolarBand`).
        let d = detail_world();
        let w = d.world();
        let look = &w.params.look;
        let (mut same_water, mut total) = (0usize, 0usize);
        for x in 0..2 {
            let img = d.render_tile(TileId { z: 0, x, y: 0 });
            for py in (0..TILE_SIZE).step_by(2) {
                for px in (0..TILE_SIZE).step_by(2) {
                    let (lon, lat) = TileId { z: 0, x, y: 0 }.lon_lat(px as f64, py as f64);
                    let (bx, by) = d.base_xy(lon, lat);
                    let i = w.index(bx as u32 % w.width, (by as u32).min(w.height - 1));
                    let base_water =
                        d.base_elevation().value(i) - w.params.sea_level < look.rules.waterline;
                    let tile_water =
                        d.elevation(lon, lat, 1e9) - w.params.sea_level < look.rules.waterline;
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
        // Within the coastal band the base is read at a displaced position
        // instead (capes and bays), which moves it by the slope of the
        // coast: left out.
        let d = detail_world();
        let w = d.world();
        let waterline = w.params.sea_level + w.params.look.rules.waterline;
        let mut worst = 0.0f32;
        for k in 0..2000 {
            let lon = (k as f64 * 0.731) % TAU;
            let lat = ((k as f64 * 0.377) % PI) - PI / 2.0;
            let (x, y) = d.base_xy(lon, lat);
            let base = bicubic(&d.base_elevation(), w.width, w.height, x, y);
            if (base - waterline).abs() < COAST_BAND.1 {
                continue;
            }
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
        let waterline = w.params.sea_level + w.params.look.rules.waterline;
        for k in 0..5000 {
            let lon = (k as f64 * 0.731) % TAU;
            let lat = ((k as f64 * 0.377) % PI) - PI / 2.0;
            let (x, y) = d.base_xy(lon, lat);
            let base = bicubic(&d.base_elevation(), w.width, w.height, x, y);
            if (base - waterline).abs() < COAST_BAND.1 {
                continue;
            }
            for pixel_km in [5.0, 1.0, 0.3] {
                let e = d.elevation(lon, lat, pixel_km);
                assert_eq!(e >= waterline, base >= waterline, "{lon} {lat} {pixel_km}");
            }
        }
    }

    /// Relief detail of a world's deepest tiles: the mean east slope of the
    /// height detail alone (the ground, without the small craters, minus
    /// the interpolated base elevation) on land tiles away from the coast,
    /// in encoded normal units.
    fn relief_detail(d: &DetailWorld) -> f64 {
        let w = d.world();
        let waterline = w.params.open_waterline();
        let (cols, rows) = TileId {
            z: MAX_LEVEL,
            x: 0,
            y: 0,
        }
        .grid();
        let pixel_km = w.params.circumference_km as f64 / (cols as f64 * TILE_SIZE as f64);
        let side = TILE_SIZE as usize + 2;
        let base = |tile: TileId, px: f64, py: f64| {
            let (lon, lat) = tile.lon_lat(px, py);
            let (x, y) = d.base_xy(lon, lat.clamp(-PI / 2.0, PI / 2.0));
            bicubic(&d.base_elevation(), w.width, w.height, x, y)
        };
        let (mut sum, mut count) = (0.0, 0.0);
        let tiles = (0..4000u32)
            .map(|k| TileId {
                z: MAX_LEVEL,
                x: (k * 7919) % cols,
                y: rows / 4 + (k * 104_729) % (rows / 2),
            })
            .filter(|&tile| base(tile, 128.0, 128.0) > waterline + COAST_BAND.1)
            .take(8);
        for tile in tiles {
            let (_, ground) = d.tile_elevation(tile, pixel_km, &vec![0.0; side * side]);
            let detail: Vec<f64> = (0..side * side)
                .map(|i| {
                    f64::from(
                        ground[i] - base(tile, (i % side) as f64 - 1.0, (i / side) as f64 - 1.0),
                    )
                })
                .collect();
            for y in (1..side - 1).step_by(4) {
                let (_, lat) = tile.lon_lat(0.0, y as f64 - 1.0);
                for x in 1..side - 1 {
                    let n = d.normal(&detail, side, x, y, pixel_km, lat);
                    sum += f64::from(n[0].abs() * 127.5);
                    count += 1.0;
                }
            }
        }
        sum / count
    }

    /// The ice shell of a round moon is its bedrock, not a smooth cover of
    /// snow or glacier over it: its deepest tiles carry fine relief as a
    /// planet's land does, not a tenth of it. (The Titan of
    /// `{"system":{"planets":{"titan-like":1},"seed":1}}`, moon b-3, at
    /// 512 × 256, as stored; a small icy moon keeps the smooth ice of its look.)
    #[test]
    fn an_ice_shell_keeps_its_relief_detail() {
        use rand::SeedableRng;
        let request: crate::system::SystemRequest =
            serde_json::from_str(r#"{"system":{"planets":{"titan-like":1},"seed":1}}"#).unwrap();
        let system =
            crate::system::draw(&request, &mut rand::rngs::StdRng::seed_from_u64(1)).unwrap();
        let moon = system.bodies.iter().find(|b| b.id == "b-3").unwrap();
        let mut params = crate::system::generation_params(moon);
        assert!(ice_shell(&params));
        (params.width, params.height) = (512, 256);
        let titan = DetailWorld::new(LoadedWorld::from_world(&generate(&params).world));
        let (titan, planet) = (relief_detail(&titan), relief_detail(&detail_world()));
        assert!(titan > 0.5 * planet, "Titan {titan}, planet {planet}");

        let planet = system.bodies.iter().find(|b| b.id == "b").unwrap();
        let small = crate::moons::small_body_params(
            &planet.params,
            crate::moons::Composition::Ice,
            200.0,
            7,
            None,
        );
        assert!(crate::generation::icy(&small) && !ice_shell(&small));
    }

    /// A Europa's ocean lies under its ice shell: its surface is that
    /// shell, land with the relief and the small craters of its crust,
    /// not a flat frozen sea, whose tiles its craters dig. (The Europa of
    /// `{"system":{"planets":{"europa-like":1},"seed":1}}` at 256 × 128.)
    #[test]
    fn a_europa_is_an_ice_crust_not_a_frozen_sea() {
        use rand::SeedableRng;
        let request: crate::system::SystemRequest =
            serde_json::from_str(r#"{"system":{"planets":{"europa-like":1},"seed":1}}"#).unwrap();
        let system =
            crate::system::draw(&request, &mut rand::rngs::StdRng::seed_from_u64(1)).unwrap();
        let moon = system
            .bodies
            .iter()
            .find(|b| b.params.archetype.as_deref() == Some("europa-like"))
            .unwrap();
        let model = moon.model.as_ref().unwrap();
        assert!(model.ice_shell.is_some_and(|s| s.ocean_km > 0.0));
        let mut params = crate::system::generation_params(moon);
        assert!(ice_shell(&params));
        (params.width, params.height) = (256, 128);
        // Its tidal heat keeps its surface young, a few Ma here
        // (`planet::interim::ICE_LID_HALF_FLUX_W_M2`), with hardly a
        // crater: aged as a Callisto, so that its tiles have small craters
        // to dig.
        assert!(params.surface_age_ga < 0.1, "{}", params.surface_age_ga);
        params.surface_age_ga = 4.0;
        let world = generate(&params).world;
        let grid = params.grid();
        let (mut land, mut total) = (0.0, 0.0);
        for (i, biome) in world.biomes.iter().enumerate() {
            let area = grid.latitude((i / world.width as usize) as u32).cos();
            total += area;
            if !biome.is_water() {
                land += area;
            }
        }
        assert!(land / total > 0.9, "land {}", land / total);
        // Its relief is a small body's, flattened (`relief_share`, about a
        // ninth of a planet's): its tiles keep their full detail against
        // it, not the tenth that smooth ice keeps (see `roughness`).
        let europa = DetailWorld::new(LoadedWorld::from_world(&world));
        let share = f64::from(crate::generation::relief_share(&params));
        let (detail, planet) = (relief_detail(&europa), relief_detail(&detail_world()));
        assert!(
            detail > 0.5 * share * planet,
            "Europa {detail}, planet {planet}, share {share}"
        );
        // And the small craters dig its tiles.
        let (cols, rows) = TileId {
            z: MAX_LEVEL,
            x: 0,
            y: 0,
        }
        .grid();
        let pixel_km = f64::from(params.circumference_km) / f64::from(cols * TILE_SIZE);
        let side = TILE_SIZE as usize + 2;
        let dug = (0..16)
            .filter(|&k| {
                let tile = TileId {
                    z: MAX_LEVEL,
                    x: (k * 7919) % cols,
                    y: rows / 4 + (k * 104_729) % (rows / 2),
                };
                let (cratered, bare) =
                    europa.tile_elevation(tile, pixel_km, &vec![0.0; side * side]);
                cratered.iter().zip(&bare).any(|(&c, &b)| c as f32 != b)
            })
            .count();
        assert!(dug > 0, "no small crater on 16 tiles");
    }

    /// The LoadedWorld of the Enceladus of seed 1 at 512 × 256 (base
    /// rows 3.1 km), with its geysers.
    fn enceladus() -> &'static LoadedWorld {
        use rand::SeedableRng;
        use std::sync::OnceLock;
        static WORLD: OnceLock<LoadedWorld> = OnceLock::new();
        WORLD.get_or_init(|| {
            let request: crate::system::SystemRequest = serde_json::from_str(
                r#"{"system":{"star":"G2V","metallicity":0.0,"planets":{"enceladus-like":1},"seed":1}}"#,
            )
            .unwrap();
            let system =
                crate::system::draw(&request, &mut rand::rngs::StdRng::seed_from_u64(1)).unwrap();
            let moon = system
                .bodies
                .iter()
                .find(|b| b.params.archetype.as_deref() == Some("enceladus-like"))
                .unwrap();
            let mut params = crate::system::generation_params(moon);
            (params.width, params.height) = (512, 256);
            assert!(params.model.as_ref().unwrap().geysers.is_some());
            LoadedWorld::from_world(&generate(&params).world)
        })
    }

    /// The z = 5 tiles a stripe crosses, and the tile pixel coordinates of
    /// the drawn lines of the stripes (`crate::planet::geysers::drawn_line`),
    /// their middle only (the ends taper away).
    fn stripe_tiles(world: &LoadedWorld) -> (Vec<TileId>, Vec<Vec<(f64, f64)>>) {
        let p = &world.params;
        let geysers = p.model.as_ref().unwrap().geysers.as_ref().unwrap();
        let radius_km = f64::from(p.circumference_km) / TAU;
        let z = 5;
        let (cols, rows) = TileId { z, x: 0, y: 0 }.grid();
        let lines: Vec<Vec<(f64, f64)>> = geysers
            .fractures
            .iter()
            .enumerate()
            .filter(|(_, f)| f.length_km > 0.0)
            .map(|(i, f)| {
                let line = crate::planet::geysers::drawn_line(f, i, radius_km);
                let n = line.len();
                line[n / 5..n - n / 5]
                    .iter()
                    .map(|v| {
                        let lat = v[2].clamp(-1.0, 1.0).asin();
                        let lon = v[1].atan2(v[0]).rem_euclid(TAU);
                        (
                            lon / TAU * f64::from(cols * TILE_SIZE) - 0.5,
                            (0.5 - lat / PI) * f64::from(rows * TILE_SIZE) - 0.5,
                        )
                    })
                    .collect()
            })
            .collect();
        let mut tiles: Vec<TileId> = lines
            .iter()
            .flat_map(|l| [l[l.len() / 4], l[l.len() / 2], l[3 * l.len() / 4]])
            .map(|(x, y)| TileId {
                z,
                x: (x / f64::from(TILE_SIZE)) as u32 % cols,
                y: ((y / f64::from(TILE_SIZE)) as u32).min(rows - 1),
            })
            .collect();
        tiles.sort_by_key(|t| (t.x, t.y));
        tiles.dedup();
        (tiles, lines)
    }

    /// How far, in tile pixels, the dark trough of the stripes drawn on
    /// the z = 5 tiles of an Enceladus (base rows 3.1 km, 32 tile pixels)
    /// lies from the drawn line of its fracture: measured across each
    /// tile column (or row, where the line runs closer to a column) as
    /// the centroid of the darkening within 20 pixels of the line.
    fn trough_offsets(d: &DetailWorld) -> Vec<f64> {
        let (tiles, lines) = stripe_tiles(d.world());
        let ts = f64::from(TILE_SIZE);
        let mut offsets = Vec::new();
        for tile in tiles {
            let image = d.render_tile(tile);
            let (ox, oy) = (f64::from(tile.x) * ts, f64::from(tile.y) * ts);
            let lum = |x: i64, y: i64| -> Option<f64> {
                if !(0..TILE_SIZE as i64).contains(&x) || !(0..TILE_SIZE as i64).contains(&y) {
                    return None;
                }
                let p = image.get_pixel(x as u32, y as u32);
                Some(p.0.iter().map(|&c| f64::from(c)).sum())
            };
            for line in &lines {
                for s in line.windows(2) {
                    let ((x0, y0), (x1, y1)) =
                        ((s[0].0 - ox, s[0].1 - oy), (s[1].0 - ox, s[1].1 - oy));
                    if (x1 - x0).abs() > ts {
                        continue; // across longitude 0
                    }
                    let along_x = (x1 - x0).abs() >= (y1 - y0).abs();
                    // Measure across the axis the line runs closer to.
                    let (a0, a1, b0, b1) = if along_x {
                        (x0, x1, y0, y1)
                    } else {
                        (y0, y1, x0, x1)
                    };
                    let (lo, hi) = (a0.min(a1).ceil() as i64, a0.max(a1).floor() as i64);
                    for a in lo..=hi {
                        let t = if a1 == a0 {
                            0.0
                        } else {
                            (a as f64 - a0) / (a1 - a0)
                        };
                        let b = b0 + t * (b1 - b0);
                        let at = |k: i64| {
                            if along_x { lum(a, k) } else { lum(k, a) }
                        };
                        let window: Option<Vec<(i64, f64)>> = (b.round() as i64 - 20
                            ..=b.round() as i64 + 20)
                            .map(|k| at(k).map(|l| (k, l)))
                            .collect();
                        let Some(window) = window else { continue };
                        let bright = window.iter().map(|w| w.1).fold(0.0, f64::max);
                        let (mut sum, mut weight) = (0.0, 0.0);
                        for (k, l) in &window {
                            sum += *k as f64 * (bright - l);
                            weight += bright - l;
                        }
                        if weight > 200.0 {
                            offsets.push(sum / weight - b);
                        }
                    }
                }
            }
        }
        offsets
    }

    /// The tiger stripes in the tiles follow their fracture's drawn line
    /// at the tile's resolution: no comb of teeth one base row long, as
    /// when the tiles interpolated the stripes the base map drew at its
    /// own pixel centres.
    #[test]
    fn tiles_draw_the_stripes_along_their_line() {
        let d = DetailWorld::new(enceladus().clone());
        let mut offsets = trough_offsets(&d);
        assert!(offsets.len() > 500, "{} samples", offsets.len());
        offsets.iter_mut().for_each(|o| *o = o.abs());
        offsets.sort_by(f64::total_cmp);
        let (median, p95) = (
            offsets[offsets.len() / 2],
            offsets[offsets.len() * 95 / 100],
        );
        println!(
            "{} samples, offset median {median:.2} px, 95th {p95:.2} px",
            offsets.len()
        );
        assert!(
            p95 < 2.0,
            "median {median:.2} px, 95th percentile {p95:.2} px"
        );
    }

    /// At the base map's resolution (z = 0 on a map 512 pixels wide), a
    /// tile draws the stripes the base map holds, pixel for pixel: the
    /// tiles take off exactly what they draw back.
    #[test]
    fn tiles_at_the_base_resolution_draw_the_base_stripes() {
        let d = DetailWorld::new(enceladus().clone());
        let (_, base) = d.stripes.as_ref().unwrap();
        let w = d.world();
        assert_eq!(w.width, 2 * TILE_SIZE);
        let pixel_km = f64::from(w.params.circumference_km) / f64::from(w.width);
        let side = TILE_SIZE as usize + 2;
        let mut touched = 0;
        for x in 0..2 {
            let drawn = d.tile_stripes(TileId { z: 0, x, y: 0 }, pixel_km).unwrap();
            for py in 0..TILE_SIZE as usize {
                for px in 0..TILE_SIZE as usize {
                    let i = py * w.width as usize + x as usize * TILE_SIZE as usize + px;
                    let (e, a) = drawn[(py + 1) * side + px + 1];
                    assert!((e - base.get(i, 0)).abs() < 1e-6, "{px}, {py}");
                    assert!((a - base.get(i, 1)).abs() < 1e-6, "{px}, {py}");
                    touched += usize::from(a != 0.0);
                }
            }
        }
        assert!(touched > 50, "{touched} pixels");
    }

    /// Tiles away from the stripes are drawn as without them.
    #[test]
    fn tiles_away_from_the_stripes_are_unchanged() {
        let world = enceladus();
        let mut bare = world.clone();
        let geysers = bare.params.model.as_mut().unwrap().geysers.take().unwrap();
        let (with, without) = (DetailWorld::new(world.clone()), DetailWorld::new(bare));
        // The other hemisphere's tiles, at a few levels.
        let row = |z: u32| if geysers.south { 0 } else { (1 << z) - 1 };
        for (z, x) in [(2, 3), (4, 11), (5, 40), (6, 77)] {
            let tile = TileId {
                z,
                x,
                y: row(z) + if z > 2 { 2 } else { 0 },
            };
            assert!(
                with.render_tile(tile) == without.render_tile(tile),
                "{tile:?}"
            );
        }
    }

    #[test]
    fn longitude_seam_is_continuous() {
        let d = detail_world();
        let (east, west) = (TileId { z: 2, x: 7, y: 1 }, TileId { z: 2, x: 0, y: 1 });
        let (a, b) = (d.render_tile(east), d.render_tile(west));
        let mut diff = 0.0;
        // Both halves: the colour and the normals.
        for y in 0..2 * TILE_SIZE {
            let (p, q) = (a.get_pixel(TILE_SIZE - 1, y), b.get_pixel(0, y));
            diff += (0..3)
                .map(|c| (p[c] as f32 - q[c] as f32).abs())
                .sum::<f32>();
        }
        // Neighbouring columns across the seam: similar colours.
        assert!(
            diff / (2.0 * TILE_SIZE as f32 * 3.0) < 20.0,
            "mean step {diff}"
        );
    }

    /// Base worlds of an Enceladus-sized moon (500 km across, 512 x 256:
    /// a base row is 3.1 km, a column 19 m on the polar row) whose
    /// elevation is a gentle isotropic random relief (Perlin noise on the
    /// sphere, a lattice cell four rows wide, one per seed), quantised to
    /// 16 bits as stored: a 16-bit step is about 50 m at the hillshade's
    /// exaggeration, a sixteenth of the relief's rise over a row.
    fn moons_of_random_relief(seeds: std::ops::Range<u32>) -> Vec<DetailWorld> {
        let params = crate::world::WorldParams {
            sea_level: 0.2,
            circumference_km: 40_000.0,
            ..crate::generation::tests::preset_params("terran", 3, 512, 256)
        };
        let mut world = generate(&params).world;
        world.params.circumference_km = 1_590.0;
        let grid = world.params.grid();
        // Four rows a lattice cell: 256 rows span pi.
        let f = 256.0 / (4.0 * PI);
        seeds
            .map(|seed| {
                let noise = Perlin::new(seed);
                for y in 0..world.height {
                    for x in 0..world.width {
                        let p = grid.unit_vector_at(f64::from(x), f64::from(y));
                        // Off the lattice's lines, where Perlin noise is calm.
                        let v = noise.get([p[0] * f + 0.37, p[1] * f + 0.61, p[2] * f + 0.23]);
                        let i = world.index(x, y);
                        world.elevation[i] = 0.6 + 0.004 * v as f32;
                    }
                }
                DetailWorld::new(LoadedWorld::from_world(&world))
            })
            .collect()
    }

    /// Mean squares of the east and north components of the tile normals
    /// of level `z` and row `y` whose latitudes lie in `band` (degrees,
    /// either hemisphere), over the tiles round the globe, and their count.
    fn normal_squares(d: &DetailWorld, z: u32, y: u32, band: (f64, f64)) -> (f64, f64, usize) {
        let (cols, _) = TileId { z, x: 0, y }.grid();
        let pixel_km = d.world().params.circumference_km as f64 / (cols as f64 * TILE_SIZE as f64);
        let side = TILE_SIZE as usize + 2;
        let no_rivers = vec![0.0; side * side];
        let (mut east, mut north, mut n) = (0.0, 0.0, 0usize);
        for x in (0..cols).step_by((cols as usize / 8).max(1)) {
            let tile = TileId { z, x, y };
            let (elevation, _) = d.tile_elevation(tile, pixel_km, &no_rivers);
            for py in 1..side - 1 {
                for px in (1..side - 1).step_by(3) {
                    let (_, lat) = tile.lon_lat(px as f64 - 1.0, py as f64 - 1.0);
                    if !(band.0..band.1).contains(&lat.to_degrees().abs()) {
                        continue;
                    }
                    let normal = d.normal(&elevation, side, px, py, pixel_km, lat);
                    east += f64::from(normal[0]).powi(2);
                    north += f64::from(normal[1]).powi(2);
                    n += 1;
                }
            }
        }
        (east, north, n)
    }

    /// Root mean square of the east and north components of the normals of
    /// level `z`'s polar tiles on worlds `ds`, within `band` (degrees).
    fn polar_normal_rms(ds: &[DetailWorld], z: u32, band: (f64, f64)) -> (f64, f64) {
        let rows = 1 << z;
        let (mut east, mut north, mut n) = (0.0, 0.0, 0usize);
        for d in ds {
            for y in [0, rows - 1] {
                let (e, s, k) = normal_squares(d, z, y, band);
                (east, north, n) = (east + e, north + s, n + k);
            }
        }
        assert!(n > 0, "no pixel in {band:?}");
        ((east / n as f64).sqrt(), (north / n as f64).sqrt())
    }

    /// Near a pole, a base column is a sliver of a base row (a 160th of it
    /// on the polar row of a 512 x 256 map): the 16-bit stairs, biome
    /// borders and edges a few columns wide along a parallel, spread over
    /// two rows by the interpolation, turned into steep east-west slopes,
    /// rays of light and shade out of the pole. The tiles carry no
    /// structure along the parallels finer than a base row: near a pole
    /// their normals lean east no more than north, as at mid-latitudes.
    /// Without the polar band, the east component's root mean square was
    /// 1.80 times the north's within a row of the poles (0.458 against
    /// 0.255) and 1.32 times one to four rows from them.
    #[test]
    fn polar_tiles_lean_east_no_more_than_north() {
        let ds = moons_of_random_relief(0..4);
        let row = 180.0 / f64::from(ds[0].world().height);
        // Level 4: 16 tile pixels a base row.
        let (east, north, n) = normal_squares(&ds[0], 4, 5, (20.0, 40.0));
        let (mid_east, mid_north) = ((east / n as f64).sqrt(), (north / n as f64).sqrt());
        assert!(
            (mid_east / mid_north - 1.0).abs() < 0.1,
            "mid-latitude east {mid_east} north {mid_north}"
        );
        for band in [(90.0 - row, 90.0), (90.0 - 4.0 * row, 90.0 - row)] {
            let (east, north) = polar_normal_rms(&ds, 4, band);
            assert!(east < 1.2 * north, "{band:?}: east {east} north {north}");
        }
    }

    /// The polar band leaves the rows within 60 degrees of the equator as
    /// they are: tiles there are the same, byte for byte, with or without
    /// it. Nearer the poles it changes them.
    #[test]
    fn tiles_away_from_the_poles_are_unchanged() {
        let d = detail_world();
        let mut plain = detail_world();
        let (w, h) = (d.world().width, d.world().height);
        assert!(d.polar_elevation.rows > 0);
        plain.polar_elevation = PolarBand::none(w, h);
        plain.polar_albedo = PolarBand::none(w, h);
        plain.polar_roughness = PolarBand::none(w, h);
        plain.polar_floodplain = PolarBand::none(w, h);
        // Level 3 has 8 rows of tiles, 22.5 degrees each: the second spans
        // 67.5 to 45 degrees, and reads the band above 60 only.
        for y in 1..7 {
            for x in [0, 5, 11] {
                let tile = TileId { z: 3, x, y };
                let (a, b) = (d.render_tile(tile), plain.render_tile(tile));
                for py in 0..TILE_SIZE {
                    let (_, lat) = tile.lon_lat(0.0, f64::from(py));
                    // The interpolation reads two base rows (1.4 degrees)
                    // round a pixel, the climate's warp one more.
                    if lat.to_degrees().abs() > 57.0 {
                        continue;
                    }
                    for row in [py, py + TILE_SIZE] {
                        for px in 0..TILE_SIZE {
                            assert_eq!(
                                a.get_pixel(px, row),
                                b.get_pixel(px, row),
                                "{tile:?} {px} {row}"
                            );
                        }
                    }
                }
            }
        }
        let pole = TileId { z: 3, x: 5, y: 0 };
        assert_ne!(d.render_tile(pole), plain.render_tile(pole));
    }

    /// A box one column wide or less is the row itself; a wider one keeps
    /// the row's mean and a plane along it, and cuts the columns at its
    /// ends for their share.
    #[test]
    fn box_filter_cuts_the_columns_at_its_ends() {
        let row: Vec<f64> = (0..16).map(|i| f64::from((i * 7) % 5)).collect();
        assert_eq!(box_filter(&row, 1.0), row);
        assert_eq!(box_filter(&row, 0.4), row);
        let smooth = box_filter(&row, 2.5);
        let mean = |r: &[f64]| r.iter().sum::<f64>() / r.len() as f64;
        assert!((mean(&smooth) - mean(&row)).abs() < 1e-12);
        // Column 3 over 2.5 columns: itself, and three quarters of each
        // neighbour.
        let want = (row[3] + 0.75 * (row[2] + row[4])) / 2.5;
        assert!((smooth[3] - want).abs() < 1e-12, "{} {want}", smooth[3]);
        // Wrapping round the parallel.
        let want = (row[0] + 0.75 * (row[15] + row[1])) / 2.5;
        assert!((smooth[0] - want).abs() < 1e-12, "{} {want}", smooth[0]);
    }
}
