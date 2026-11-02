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
//!   coastlines move by less than a base pixel) and larger than two tile
//!   pixels (so a tile is the low-pass of its children). Amplitudes follow
//!   a 1/f law, stronger on high ground than on plains.
//! - **Climate**: temperature, moisture and the other biome inputs are read
//!   at a slightly warped position, so biome borders become irregular instead
//!   of interpolated straight edges; temperature follows the new elevation.
//! - **Biomes**: the generation's own rules and preset remap, with soft
//!   borders, hillshading and the vector rivers drawn at the tile's scale.
//!
//! Tiles form an equirectangular quadtree: level `z` has `2^(z+1)` × `2^z`
//! tiles of [`TILE_SIZE`] pixels, column 0 at longitude 0 and row 0 at the
//! north pole.
use crate::biome::{Biome, BiomeInput, biome_color};
use crate::climate::temperature;
use crate::export::ecotone::ecotone_colour_for;
use crate::export::{LIGHT, RELIEF_KM};
use crate::rivers::{self, RiverLine};
use crate::world::World;
use image::{Rgb, RgbImage};
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use std::f64::consts::{PI, TAU};

/// Tile side in pixels.
pub const TILE_SIZE: u32 = 256;
/// Deepest level served: at z = 8 an Earth-sized planet is about 300 m per pixel.
pub const MAX_LEVEL: u32 = 8;
/// Amplitude of the first detail octave (elevation units), on plains.
const DETAIL_AMPLITUDE: f32 = 0.004;
/// Extra amplitude on high ground: detail grows to `1 + MOUNTAIN_BOOST` times
/// the plain amplitude 0.3 elevation units above the coastline.
const MOUNTAIN_BOOST: f32 = 2.0;
/// Warp of the climate sampling position, in base pixels.
const CLIMATE_WARP_PX: f64 = 1.0;
/// Real river width: `REAL_WIDTH_KM × √(flow km²)` (about 5 km at 6 M km²).
const REAL_WIDTH_KM: f64 = 0.002;

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

    /// Whether the tile exists.
    pub fn is_valid(self) -> bool {
        let (cols, rows) = self.grid();
        self.z <= MAX_LEVEL && self.x < cols && self.y < rows
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
    world: World,
    noise: Perlin,
    warp_noise: Perlin,
    /// River polylines with their bounding boxes in base pixels.
    rivers: Vec<(RiverLine, [f64; 4])>,
    /// 1 on base lake pixels (water above the waterline, not under a drawn
    /// river), 0 elsewhere; sampled bilinearly for smooth shores.
    lakes: Vec<f32>,
}

/// Bilinear sample of a row-major base layer at base pixel coordinates
/// (pixel centres at +0.5), wrapping longitude and clamping latitude.
fn bilinear(layer: &[f32], width: u32, height: u32, x: f64, y: f64) -> f32 {
    let (w, h) = (width as i64, height as i64);
    let (x, y) = (x - 0.5, (y - 0.5).clamp(0.0, (h - 1) as f64));
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
    let at = |xx: i64, yy: i64| layer[(yy.clamp(0, h - 1) * w + xx.rem_euclid(w)) as usize];
    let (xi, yi) = (x0 as i64, y0 as i64);
    let top = at(xi, yi) * (1.0 - fx) + at(xi + 1, yi) * fx;
    let bottom = at(xi, yi + 1) * (1.0 - fx) + at(xi + 1, yi + 1) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Bicubic (Catmull-Rom) sample of a base layer: unlike bilinear, its slope
/// is continuous, so hillshading shows no seams along base pixel edges.
fn bicubic(layer: &[f32], width: u32, height: u32, x: f64, y: f64) -> f32 {
    let (w, h) = (width as i64, height as i64);
    let (x, y) = (x - 0.5, (y - 0.5).clamp(0.0, (h - 1) as f64));
    let (x0, y0) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
    let at = |xx: i64, yy: i64| layer[(yy.clamp(0, h - 1) * w + xx.rem_euclid(w)) as usize];
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
/// (km), amplitude halving with the wavelength (1/f).
fn band_limited(noise: &Perlin, p: [f64; 3], radius_km: f64, first_km: f64, last_km: f64) -> f32 {
    let mut sum = 0.0;
    let (mut wavelength, mut amplitude) = (first_km, 1.0);
    while wavelength >= last_km {
        let f = radius_km / wavelength;
        sum += noise.get([p[0] * f, p[1] * f, p[2] * f]) * amplitude;
        wavelength /= 2.0;
        amplitude /= 2.0;
    }
    sum as f32
}

impl DetailWorld {
    pub fn new(world: World) -> Self {
        let rivers = world
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
                (line.clone(), b)
            })
            .collect();
        let seed = world.params.seed;
        let coverage = rivers::coverage(&world.rivers, world.width, world.height);
        let waterline = world.params.sea_level + world.params.planet.rules.waterline;
        let lakes = (0..world.biomes.len())
            .map(|i| {
                let lake = world.biomes[i].is_water()
                    && world.elevation[i] >= waterline
                    && coverage[i] < rivers::RIVER_COVERAGE;
                if lake { 1.0 } else { 0.0 }
            })
            .collect();
        Self {
            lakes,
            noise: Perlin::new(seed.wrapping_add(1000)),
            warp_noise: Perlin::new(seed.wrapping_add(1001)),
            rivers,
            world,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
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
        let last = 2.0 * pixel_km;
        if last > first {
            return base;
        }
        let p = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
        let radius = w.params.circumference_km as f64 / TAU;
        let above = base - w.params.sea_level - w.params.planet.rules.waterline;
        let relief = 1.0 + MOUNTAIN_BOOST * (above / 0.3).clamp(0.0, 1.0);
        base + DETAIL_AMPLITUDE * relief * band_limited(&self.noise, p, radius, first, last)
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
        let elevation: Vec<f32> = (0..side * side)
            .into_par_iter()
            .map(|i| {
                let (px, py) = ((i % side) as f64 - 1.0, (i / side) as f64 - 1.0);
                let (lon, lat) = tile.lon_lat(px, py);
                self.elevation(lon, lat.clamp(-PI / 2.0, PI / 2.0), pixel_km)
            })
            .collect();

        let coverage = self.river_coverage(tile, pixel_km);
        let river_colour = biome_color(planet.remap(Biome::River)).map(f32::from);
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
                let sample = |layer: &[f32]| bilinear(layer, w.width, w.height, cx, cy);
                let base_above = sample(&w.elevation) - w.params.sea_level - planet.rules.waterline;
                // Temperature follows the detailed elevation (lapse rate).
                let t =
                    sample(&w.temperature) + temperature(0.0, above) - temperature(0.0, base_above);
                // Lakes: smooth shores from the base lake mask.
                let lake = sample(&self.lakes) > 0.5 && above >= 0.0;
                let input = BiomeInput {
                    elevation: relief,
                    moisture: sample(&w.moisture),
                    temperature: t.clamp(0.0, 1.0),
                    volcanic_zone: sample(&w.volcanic_zone),
                    coast_distance_km: sample(&w.coast_distance_km),
                    lake,
                    river: false,
                    highland_offset: sample(&w.highland_offset),
                };
                let biome = planet.remap(planet.rules.choose(input));
                let key = (tile.x as usize * n + px) ^ ((tile.y as usize * n + py) << 20);
                let mut colour = if biome.is_water() {
                    biome_color(biome).map(f32::from)
                } else {
                    let shade = self.hillshade(&elevation, side, px + 1, py + 1, pixel_km, lat);
                    ecotone_colour_for(planet, key, input).map(|c| c * shade)
                };
                let c = coverage[i];
                if c > 0.0 && input.temperature >= planet.rules.polar && !biome.is_water() {
                    for (channel, river) in colour.iter_mut().zip(river_colour) {
                        *channel = *channel * (1.0 - c) + river * c;
                    }
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
        (lit / LIGHT[2]).clamp(0.55, 1.3)
    }

    /// Anti-aliased coverage of the rivers on a tile. Rivers keep at least
    /// the width they have on the base map, in screen pixels, and grow to
    /// their real width when zoomed in far enough.
    fn river_coverage(&self, tile: TileId, pixel_km: f64) -> Vec<f32> {
        let w = &self.world;
        let n = TILE_SIZE as usize;
        let mut cov = vec![0f32; n * n];
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
        let k = rivers::scale(w.width);
        for (line, b) in &self.rivers {
            for shift in [-(w.width as f64), 0.0, w.width as f64] {
                let visible = b[0] + shift < ox + span_x + margin
                    && b[2] + shift > ox - margin
                    && b[1] < oy + span_y + margin
                    && b[3] > oy - margin;
                if !visible {
                    continue;
                }
                let to_tile = |p: [f64; 2]| [(p[0] + shift - ox) * sx, (p[1] - oy) * sy];
                let widths = rivers::widths_px(line, w.width);
                let width_at = |i: usize| {
                    // Base-map width in screen pixels, or the real width.
                    let base = widths[i] / k;
                    let real = REAL_WIDTH_KM * (line.flow_km2[i] as f64).sqrt() / pixel_km;
                    base.max(real)
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
                    let mut prev = a;
                    for j in 1..=pieces {
                        let t = j as f64 / pieces as f64;
                        let point = catmull_rom(p0, p1, p2, p3, t);
                        let next = to_tile(point);
                        let width = wa + (wb - wa) * t;
                        stamp_segment(&mut cov, n, prev, next, width);
                        prev = next;
                    }
                }
            }
        }
        cov
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

/// Adds an anti-aliased segment of the given width to a coverage buffer (max).
fn stamp_segment(cov: &mut [f32], n: usize, a: [f64; 2], b: [f64; 2], width: f64) {
    let r = width / 2.0 + 1.0;
    let (x0, x1) = ((a[0].min(b[0]) - r).floor(), (a[0].max(b[0]) + r).ceil());
    let (y0, y1) = ((a[1].min(b[1]) - r).floor(), (a[1].max(b[1]) + r).ceil());
    if x1 < 0.0 || y1 < 0.0 || x0 >= n as f64 || y0 >= n as f64 {
        return;
    }
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = (dx * dx + dy * dy).max(1e-12);
    for py in (y0.max(0.0) as usize)..(y1.min(n as f64) as usize) {
        for px in (x0.max(0.0) as usize)..(x1.min(n as f64) as usize) {
            let c = [px as f64 + 0.5, py as f64 + 0.5];
            let t = (((c[0] - a[0]) * dx + (c[1] - a[1]) * dy) / len2).clamp(0.0, 1.0);
            let d = ((c[0] - a[0] - t * dx).powi(2) + (c[1] - a[1] - t * dy).powi(2)).sqrt();
            let v = (width / 2.0 + 0.5 - d).clamp(0.0, 1.0) as f32;
            let i = py * n + px;
            cov[i] = cov[i].max(v);
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
            width: Some(512),
            height: Some(256),
            ..Default::default()
        };
        DetailWorld::new(generate(&request.resolve(&mut rand::rng()).unwrap().params).world)
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
                    let base_water = w.elevation[i] - w.params.sea_level < planet.rules.waterline;
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
