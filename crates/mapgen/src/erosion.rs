//! Hydraulic erosion by simulated rain droplets.
//!
//! Each droplet starts on land, rolls downhill with some inertia, picks up
//! sediment while it speeds up on steep ground and drops it when it slows
//! down, evaporates, or reaches the sea. Thousands of droplets carve valleys
//! into the ridges and leave fans and deltas at their feet.
//!
//! The simulation runs on a coarser grid than the map: one cell per
//! [`CELL_PX`] × [`CELL_PX`] map pixels, at least [`EROSION_WIDTH`] ×
//! [`EROSION_HEIGHT`] (or the map size if smaller) and at most
//! [`MAX_EROSION_WIDTH`] × [`MAX_EROSION_HEIGHT`], so the cost stays bounded.
//! The resulting height change is smoothed back onto the full-resolution
//! elevation with a cubic B-spline, and never moves a pixel across the
//! waterline: at 16 map pixels per cell, a linear interpolation drew a grid of
//! pits and bumps that flipped flat lowlands between land and sea.
//!
//! Droplets run on heights exaggerated to the slopes their constants expect
//! ([`SLOPE_GAIN`]), only carve where the ground is steep enough, and carve
//! less and less towards the shore; on flat ground the change is blurred into
//! wide swales; sediment that reaches the sea is carried offshore. Mountains
//! get valleys while plains and deserts stay smooth, and coasts keep their
//! profile instead of a flat shelf at sea level.
use crate::noise::smoothstep;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;

/// Smallest erosion grid width, in cells.
pub const EROSION_WIDTH: u32 = 512;
/// Smallest erosion grid height, in cells.
pub const EROSION_HEIGHT: u32 = 256;
/// Largest erosion grid width, in cells.
pub const MAX_EROSION_WIDTH: u32 = 2048;
/// Largest erosion grid height, in cells.
pub const MAX_EROSION_HEIGHT: u32 = 1024;
/// Map pixels per erosion cell side, between the two grid bounds.
pub const CELL_PX: u32 = 4;

/// Share of a pixel's height above (or depth below) the waterline that
/// erosion may remove (or deposition fill): below 1, so no pixel changes
/// between land and sea.
const WATERLINE_MARGIN: f32 = 0.9;

/// Vertical exaggeration of the simulation on a 1024-cell-wide grid, scaled
/// with the cell size so a slope means the same at every grid size. The
/// droplet constants (Beyer's) assume heights of about one per cell; this
/// terrain is about 100 times flatter per cell, so without it the minimum
/// slope floored the sediment capacity everywhere, and droplets carved a
/// desert as deeply as a ridge.
const SLOPE_GAIN: f32 = 10.0;

/// Slope (exaggerated height per cell, see [`SLOPE_GAIN`]) below which
/// droplets stop carving and the change is blurred, fading in up to the
/// second value: plains and deserts stay smooth while mountains get their
/// valleys.
const FLAT_SLOPE: (f32, f32) = (0.04, 0.1);
/// Radius, in cells, of each box pass of the blur applied on flat ground.
const FLAT_BLUR: usize = 2;

/// Height above the waterline over which erosion fades out towards the
/// shore, in elevation units: coasts keep their profile instead of being
/// planed into a flat shelf at sea level.
const SHORE_BAND: f32 = 0.05;

/// Erosion grid size for a `width` × `height` map.
pub fn grid_size(width: u32, height: u32) -> (u32, u32) {
    let side = |px: u32, min: u32, max: u32| (px / CELL_PX).clamp(min, max).min(px);
    (
        side(width, EROSION_WIDTH, MAX_EROSION_WIDTH),
        side(height, EROSION_HEIGHT, MAX_EROSION_HEIGHT),
    )
}

/// Droplet physics, after Hans Theobald Beyer's "Implementation of a method
/// for hydraulic erosion" (2015).
struct Physics {
    /// Fraction of the previous direction kept at each step (0 = pure gradient).
    inertia: f32,
    /// Sediment a droplet can carry per unit of slope × speed × water.
    capacity: f32,
    /// Minimum slope used for capacity, so flat ground still carries a little.
    min_slope: f32,
    /// Fraction of the excess sediment deposited per step.
    deposit: f32,
    /// Fraction of the spare capacity taken from the ground per step.
    erode: f32,
    /// Fraction of water evaporated per step.
    evaporate: f32,
    gravity: f32,
    max_steps: u32,
    /// Radius of the erosion brush, in erosion-grid pixels.
    radius: i32,
}

const PHYSICS: Physics = Physics {
    inertia: 0.05,
    capacity: 4.0,
    min_slope: 0.01,
    deposit: 0.3,
    erode: 0.3,
    evaporate: 0.02,
    gravity: 4.0,
    max_steps: 48,
    radius: 2,
};

/// A height field on the erosion grid, wrapping in longitude.
struct HeightMap {
    w: usize,
    h: usize,
    z: Vec<f32>,
}

impl HeightMap {
    fn idx(&self, x: i64, y: i64) -> usize {
        let x = x.rem_euclid(self.w as i64) as usize;
        let y = y.clamp(0, self.h as i64 - 1) as usize;
        y * self.w + x
    }

    /// Bilinear height and gradient at a continuous position.
    fn sample(&self, px: f32, py: f32) -> (f32, f32, f32) {
        let (x0, y0) = (px.floor() as i64, py.floor() as i64);
        let (u, v) = (px - x0 as f32, py - y0 as f32);
        let nw = self.z[self.idx(x0, y0)];
        let ne = self.z[self.idx(x0 + 1, y0)];
        let sw = self.z[self.idx(x0, y0 + 1)];
        let se = self.z[self.idx(x0 + 1, y0 + 1)];
        let gx = (ne - nw) * (1.0 - v) + (se - sw) * v;
        let gy = (sw - nw) * (1.0 - u) + (se - ne) * u;
        let height =
            nw * (1.0 - u) * (1.0 - v) + ne * u * (1.0 - v) + sw * (1.0 - u) * v + se * u * v;
        (height, gx, gy)
    }

    /// Adds `amount` at a continuous position, split over the 4 surrounding pixels.
    fn deposit(&mut self, px: f32, py: f32, amount: f32) {
        let (x0, y0) = (px.floor() as i64, py.floor() as i64);
        let (u, v) = (px - x0 as f32, py - y0 as f32);
        for (dx, dy, wgt) in [
            (0, 0, (1.0 - u) * (1.0 - v)),
            (1, 0, u * (1.0 - v)),
            (0, 1, (1.0 - u) * v),
            (1, 1, u * v),
        ] {
            let i = self.idx(x0 + dx, y0 + dy);
            self.z[i] += amount * wgt;
        }
    }
}

/// Brush offsets and normalised weights for erosion around a point.
fn brush(radius: i32) -> Vec<(i64, i64, f32)> {
    let mut cells: Vec<(i64, i64, f32)> = (-radius..=radius)
        .flat_map(|dy| (-radius..=radius).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            (d <= radius as f32).then_some((dx as i64, dy as i64, radius as f32 - d + 1.0))
        })
        .collect();
    let total: f32 = cells.iter().map(|c| c.2).sum();
    cells.iter_mut().for_each(|c| c.2 /= total);
    cells
}

/// Cells a droplet can reach from its start: its longest path, the brush
/// radius and the bilinear neighbour, rounded up.
const REACH: usize = PHYSICS.max_steps as usize + PHYSICS.radius as usize + 2;
/// Smallest tile side for the parallel simulation: two reaches, so tiles
/// running at the same time (one tile apart) never touch the same cells.
const MIN_TILE: usize = 2 * REACH;

/// Runs the droplets on `map`. Droplets only start on land (`z >= waterline`)
/// and stop when they reach water, whose currents carry their sediment
/// offshore. Erosion fades out within `shore_band` above the waterline.
///
/// The grid is cut into tiles at least [`MIN_TILE`] cells wide, run in four
/// checkerboard phases: within a phase, active tiles are a tile apart, so
/// their droplets never meet and they run in parallel, each on a copy of
/// its tile and the cells around it. Each tile gets droplets in proportion
/// to its land and its own random stream, so the result does not depend on
/// the number of threads.
fn simulate(map: &mut HeightMap, waterline: f32, shore_band: f32, droplets: usize, seed: u64) {
    let land_total = map.z.iter().filter(|&&z| z >= waterline).count();
    if land_total == 0 || droplets == 0 {
        return;
    }
    // Columns wrap around, so the checkerboard needs an even count.
    let tiles_x = (map.w / MIN_TILE) & !1;
    let tiles_y = (map.h / MIN_TILE).max(1);
    if tiles_x < 2 {
        let starts: Vec<usize> = (0..map.z.len())
            .filter(|&i| map.z[i] >= waterline)
            .collect();
        let mut rng = StdRng::seed_from_u64(seed);
        run_droplets(map, &starts, droplets, waterline, shore_band, &mut rng);
        return;
    }
    let edge = |k: usize, n: usize, total: usize| k * total / n;
    let h = map.h as i64;
    for phase in 0..4 {
        let (cx, cy) = (phase % 2, phase / 2);
        if cy >= tiles_y {
            continue;
        }
        let tiles: Vec<(usize, usize)> = (0..tiles_y)
            .filter(|ty| ty % 2 == cy)
            .flat_map(|ty| {
                (0..tiles_x)
                    .filter(|tx| tx % 2 == cx)
                    .map(move |tx| (tx, ty))
            })
            .collect();
        let done: Vec<(i64, i64, HeightMap)> = tiles
            .par_iter()
            .map(|&(tx, ty)| {
                let (x0, x1) = (edge(tx, tiles_x, map.w), edge(tx + 1, tiles_x, map.w));
                let (y0, y1) = (edge(ty, tiles_y, map.h), edge(ty + 1, tiles_y, map.h));
                // The tile and its reach around it, clipped at the poles.
                let left = x0 as i64 - REACH as i64;
                let top = (y0 as i64 - REACH as i64).max(0);
                let bottom = (y1 as i64 + REACH as i64).min(h);
                let lw = x1 - x0 + 2 * REACH;
                let lh = (bottom - top) as usize;
                let mut local = HeightMap {
                    w: lw,
                    h: lh,
                    z: (0..lh)
                        .flat_map(|ly| (0..lw).map(move |lx| (lx, ly)))
                        .map(|(lx, ly)| map.z[map.idx(left + lx as i64, top + ly as i64)])
                        .collect(),
                };
                let starts: Vec<usize> = (y0..y1)
                    .flat_map(|y| (x0..x1).map(move |x| (x, y)))
                    .filter(|&(x, y)| map.z[y * map.w + x] >= waterline)
                    .map(|(x, y)| (y - top as usize) * lw + (x - x0 + REACH))
                    .collect();
                let count = (droplets as u64 * starts.len() as u64 / land_total as u64) as usize;
                let tile = (ty * tiles_x + tx) as u64;
                let mut rng =
                    StdRng::seed_from_u64(seed ^ tile.wrapping_mul(0x9E37_79B9_7F4A_7C15));
                run_droplets(&mut local, &starts, count, waterline, shore_band, &mut rng);
                (left, top, local)
            })
            .collect();
        for (left, top, local) in done {
            for ly in 0..local.h {
                for lx in 0..local.w {
                    let i = map.idx(left + lx as i64, top + ly as i64);
                    map.z[i] = local.z[ly * local.w + lx];
                }
            }
        }
    }
}

/// Runs `count` droplets on `map`, each starting in a random cell of
/// `starts`.
fn run_droplets(
    map: &mut HeightMap,
    starts: &[usize],
    count: usize,
    waterline: f32,
    shore_band: f32,
    rng: &mut StdRng,
) {
    let p = &PHYSICS;
    let brush = brush(p.radius);
    if starts.is_empty() {
        return;
    }
    for _ in 0..count {
        let start = starts[rng.random_range(0..starts.len())];
        let mut px = (start % map.w) as f32 + rng.random_range(0.0..1.0);
        let mut py = (start / map.w) as f32 + rng.random_range(0.0..1.0);
        let (mut dx, mut dy) = (0.0f32, 0.0f32);
        let (mut speed, mut water, mut sediment) = (1.0f32, 1.0f32, 0.0f32);

        for _ in 0..p.max_steps {
            let (height, gx, gy) = map.sample(px, py);
            if height < waterline {
                break;
            }
            dx = dx * p.inertia - gx * (1.0 - p.inertia);
            dy = dy * p.inertia - gy * (1.0 - p.inertia);
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1e-9 {
                break;
            }
            dx /= len;
            dy /= len;
            let (old_x, old_y) = (px, py);
            px = (px + dx).rem_euclid(map.w as f32);
            py += dy;
            if py < 0.0 || py >= (map.h - 1) as f32 {
                break;
            }

            let (new_height, _, _) = map.sample(px, py);
            let dh = new_height - height;
            let capacity = (-dh).max(p.min_slope) * speed * water * p.capacity;

            if sediment > capacity || dh > 0.0 {
                // Uphill: fill the pit behind; otherwise drop the excess.
                let amount = if dh > 0.0 {
                    dh.min(sediment)
                } else {
                    (sediment - capacity) * p.deposit
                };
                sediment -= amount;
                map.deposit(old_x, old_y, amount);
            } else {
                // Never dig deeper than the step down, to avoid spikes.
                let steep = smoothstep(FLAT_SLOPE.0, FLAT_SLOPE.1, (gx * gx + gy * gy).sqrt());
                let amount = ((capacity - sediment) * p.erode).min(-dh) * steep;
                let (cx, cy) = (old_x.floor() as i64, old_y.floor() as i64);
                for &(ox, oy, wgt) in &brush {
                    let i = map.idx(cx + ox, cy + oy);
                    let shore = smoothstep(0.0, shore_band, map.z[i] - waterline);
                    let taken = (amount * wgt * shore).min(map.z[i] - waterline).max(0.0);
                    map.z[i] -= taken;
                    sediment += taken;
                }
            }

            speed = (speed * speed - dh * p.gravity).max(0.0).sqrt();
            water *= 1.0 - p.evaporate;
        }
    }
}

/// Weight of erosion on each row of a map: 0 where the sea-level temperature
/// (`row_temperature` plus the planet's `shift`) stays below `polar`, where
/// water is ice and carves no valleys, rising to 1 a little above it.
pub fn thaw(row_temperature: &[f32], shift: f32, polar: f32) -> Vec<f32> {
    row_temperature
        .iter()
        .map(|t| smoothstep(polar - 0.05, polar + 0.1, t + shift))
        .collect()
}

/// Weight of erosion on row `y` of `h`: 1 up to 60° of latitude, fading to 0
/// at 75°. Droplets move in pixel steps, and the projection stretches pixels
/// east-west towards the poles, which would carve artificial north-south grooves.
fn polar_fade(y: usize, h: usize) -> f32 {
    let cos_latitude = (std::f32::consts::PI * (y as f32 + 0.5) / h as f32).sin();
    ((cos_latitude - 0.25) / 0.25).clamp(0.0, 1.0)
}

/// Erodes a full-resolution, row-major `elevation` map in place and returns
/// the height change of each pixel.
///
/// `waterline` is the absolute elevation of the sea surface. `strength` scales
/// the number of droplets (1.0 = one per erosion-grid pixel; 0 disables).
/// `thaw` weighs the change on each map row, 0 where the ground is frozen
/// all year (see [`thaw`]).
pub fn erode(
    elevation: &mut [f32],
    width: u32,
    height: u32,
    waterline: f32,
    strength: f32,
    seed: u32,
    thaw: &[f32],
) -> Vec<f32> {
    let (w, h) = (width as usize, height as usize);
    if strength <= 0.0 {
        return vec![0.0; w * h];
    }
    let (ew, eh) = grid_size(width, height);
    let (ew, eh) = (ew as usize, eh as usize);

    // Box-average the full-resolution map down to the erosion grid.
    let mut sum = vec![0.0f32; ew * eh];
    let mut count = vec![0u32; ew * eh];
    for y in 0..h {
        let ey = y * eh / h;
        for x in 0..w {
            let i = ey * ew + x * ew / w;
            sum[i] += elevation[y * w + x];
            count[i] += 1;
        }
    }
    let original: Vec<f32> = sum
        .iter()
        .zip(&count)
        .map(|(s, &c)| if c > 0 { s / c as f32 } else { 0.0 })
        .collect();
    // Simulate on exaggerated heights, so slopes are in the range the
    // droplet constants expect, and scale the change back.
    let gain = SLOPE_GAIN * ew as f32 / 1024.0;
    let mut map = HeightMap {
        w: ew,
        h: eh,
        z: original.iter().map(|z| z * gain).collect(),
    };
    let droplets = (strength.max(0.0) * (ew * eh) as f32) as usize;
    simulate(
        &mut map,
        waterline * gain,
        SHORE_BAND * gain,
        droplets,
        crate::noise::stream_seed(seed, crate::noise::stream::EROSION) as u64,
    );
    // Keep the change where the ground was steep enough to be carved: on
    // flat cells the deposits of droplets slowing down would only roughen
    // plains and deserts.
    let slope = |i: usize| {
        let (x, y) = ((i % ew) as i64, (i / ew) as i64);
        let at = |dx: i64, dy: i64| {
            original[((y + dy).clamp(0, eh as i64 - 1) as usize) * ew
                + (x + dx).rem_euclid(ew as i64) as usize]
        };
        let (gx, gy) = ((at(1, 0) - at(-1, 0)) / 2.0, (at(0, 1) - at(0, -1)) / 2.0);
        (gx * gx + gy * gy).sqrt() * gain
    };
    let raw: Vec<f32> = map
        .z
        .iter()
        .zip(&original)
        .map(|(a, b)| a / gain - b)
        .collect();
    // On flat ground, blur the change instead: valleys continue as wide,
    // shallow swales (so drainage is kept and no basin is left where they
    // reach the plains), and the deposits of slowing droplets no longer
    // roughen plains and deserts.
    let soft = blur(&raw, ew, eh, FLAT_BLUR);
    let delta: Vec<f32> = (0..ew * eh)
        .into_par_iter()
        .map(|i| {
            let steep = smoothstep(FLAT_SLOPE.0, FLAT_SLOPE.1, slope(i));
            soft[i] + (raw[i] - soft[i]) * steep
        })
        .collect();

    // Spread the height change over every full-resolution pixel with a cubic
    // B-spline: smooth (C2) across cell borders, where a bilinear
    // interpolation leaves creases that draw the grid.
    let columns: Vec<(i64, [f32; 4])> = (0..w)
        .map(|x| spline_taps((x as f32 + 0.5) * ew as f32 / w as f32 - 0.5))
        .collect();
    let mut change = vec![0.0f32; w * h];
    elevation
        .par_chunks_mut(w)
        .zip(change.par_chunks_mut(w))
        .enumerate()
        .for_each_init(
            || vec![0.0f32; ew],
            |row, (y, (elevation, change))| {
                // No rivers carve frozen ground: ice sheets stay smooth.
                let fade = polar_fade(y, h) * thaw.get(y).copied().unwrap_or(1.0);
                if fade == 0.0 {
                    return;
                }
                // Blend the 4 grid rows around this pixel row first.
                let (y0, wy) = spline_taps((y as f32 + 0.5) * eh as f32 / h as f32 - 0.5);
                row.fill(0.0);
                for (k, &wgt) in wy.iter().enumerate() {
                    let gy = (y0 + k as i64).clamp(0, eh as i64 - 1) as usize;
                    for (r, d) in row.iter_mut().zip(&delta[gy * ew..(gy + 1) * ew]) {
                        *r += wgt * d;
                    }
                }
                for ((e, c), (x0, wx)) in elevation.iter_mut().zip(change.iter_mut()).zip(&columns)
                {
                    let mut d = 0.0;
                    for (k, &wgt) in wx.iter().enumerate() {
                        d += wgt * row[(x0 + k as i64).rem_euclid(ew as i64) as usize];
                    }
                    d *= fade;
                    // Fade out towards the shore, pixel by pixel as on the grid.
                    if d < 0.0 {
                        d *= smoothstep(0.0, SHORE_BAND, *e - waterline);
                    }
                    // Keep every pixel on its side of the waterline: the grid
                    // average can stay above the sea while some of its pixels
                    // dip below, and deposits must not raise shoals into islands.
                    d = if *e >= waterline {
                        d.max((waterline - *e) * WATERLINE_MARGIN)
                    } else {
                        d.min((waterline - *e) * WATERLINE_MARGIN)
                    };
                    *c = d;
                    *e = (*e + d).clamp(-1.0, 1.0);
                }
            },
        );
    change
}

/// Approximately Gaussian blur of a grid wrapping in longitude: three box
/// passes of `radius` cells in each direction.
fn blur(field: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    let r = radius as i64;
    let n = (2 * r + 1) as f32;
    let mut a = field.to_vec();
    let mut b = vec![0.0f32; field.len()];
    for _ in 0..3 {
        b.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (x, v) in row.iter_mut().enumerate() {
                let sum: f32 = (-r..=r)
                    .map(|d| a[y * w + (x as i64 + d).rem_euclid(w as i64) as usize])
                    .sum();
                *v = sum / n;
            }
        });
        a.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (x, v) in row.iter_mut().enumerate() {
                let sum: f32 = (-r..=r)
                    .map(|d| b[(y as i64 + d).clamp(0, h as i64 - 1) as usize * w + x])
                    .sum();
                *v = sum / n;
            }
        });
    }
    a
}

/// First grid index and weights of the uniform cubic B-spline at `pos`, in
/// grid coordinates (cell centres on integers).
fn spline_taps(pos: f32) -> (i64, [f32; 4]) {
    let i = pos.floor();
    let t = pos - i;
    let (t2, t3) = (t * t, t * t * t);
    let u = 1.0 - t;
    (
        i as i64 - 1,
        [
            u * u * u / 6.0,
            (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0,
            (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0,
            t3 / 6.0,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiled_erosion_does_not_depend_on_the_thread_count() {
        // Wide enough for several tiles (MIN_TILE cells each).
        let (w, h) = (1024, 512);
        let run = |threads: usize| {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            pool.install(|| {
                let mut elevation = island(w, h);
                erode(&mut elevation, w, h, 0.0, 1.0, 9, &vec![1.0; h as usize]);
                elevation
            })
        };
        let one = run(1);
        assert_eq!(one, run(4));
        assert_ne!(one, island(w, h), "erosion changed nothing");
    }

    /// A cone-shaped island in the middle of the map.
    fn island(w: u32, h: u32) -> Vec<f32> {
        (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| {
                let dx = x as f32 / w as f32 - 0.5;
                let dy = y as f32 / h as f32 - 0.5;
                0.8 - 2.5 * (dx * dx + dy * dy).sqrt()
            })
            .collect()
    }

    #[test]
    fn erosion_is_deterministic() {
        let mut a = island(128, 64);
        let mut b = island(128, 64);
        erode(&mut a, 128, 64, 0.0, 0.5, 1, &[]);
        erode(&mut b, 128, 64, 0.0, 0.5, 1, &[]);
        assert_eq!(a, b);
    }

    #[test]
    fn erosion_lowers_peaks_and_never_digs_below_the_sea() {
        let mut map = island(128, 64);
        let before = map.clone();
        let change = erode(&mut map, 128, 64, 0.0, 1.0, 2, &[]);
        assert!(change.iter().any(|&d| d < 0.0), "nothing eroded");
        for (b, a) in before.iter().zip(&map) {
            if *b >= 0.0 {
                assert!(*a >= -1e-3, "land dug below the waterline: {b} -> {a}");
            }
        }
    }

    #[test]
    fn no_pixel_crosses_the_waterline() {
        // A wavy lowland around the waterline, finer than the erosion grid.
        let (w, h) = (256u32, 128u32);
        let mut map: Vec<f32> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| {
                let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
                0.3 - 1.2 * ((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt()
                    + 0.05 * (x as f32 * 0.9).sin() * (y as f32 * 0.7).cos()
            })
            .collect();
        let before = map.clone();
        erode(&mut map, w, h, 0.0, 2.0, 4, &[]);
        for (b, a) in before.iter().zip(&map) {
            assert_eq!(
                *b >= 0.0,
                *a >= 0.0,
                "pixel crossed the waterline: {b} -> {a}"
            );
        }
    }

    #[test]
    fn grid_follows_the_map_size_within_bounds() {
        assert_eq!(grid_size(1920, 1080), (512, 270));
        assert_eq!(grid_size(4096, 2048), (1024, 512));
        assert_eq!(grid_size(8192, 4096), (2048, 1024));
        assert_eq!(grid_size(16384, 8192), (2048, 1024));
        assert_eq!(grid_size(128, 64), (128, 64));
    }

    #[test]
    fn spline_weights_sum_to_one() {
        for pos in [0.0, 0.25, 3.5, 7.99] {
            let (_, wgt) = spline_taps(pos);
            assert!((wgt.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn zero_strength_changes_nothing() {
        let mut map = island(64, 32);
        let before = map.clone();
        erode(&mut map, 64, 32, 0.0, 0.0, 3, &[]);
        assert_eq!(map, before);
    }
}
