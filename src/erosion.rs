//! Hydraulic erosion by simulated rain droplets.
//!
//! Each droplet starts on land, rolls downhill with some inertia, picks up
//! sediment while it speeds up on steep ground and drops it when it slows
//! down, evaporates, or reaches the sea. Thousands of droplets carve valleys
//! into the ridges and leave fans and deltas at their feet.
//!
//! The simulation runs on a fixed-size grid ([`EROSION_WIDTH`] ×
//! [`EROSION_HEIGHT`], or the map size if smaller), whatever the output
//! resolution, so a world keeps the
//! same valleys at every size and the cost stays bounded. The resulting height
//! change is then interpolated back onto the full-resolution elevation.
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

/// Width of the erosion grid in pixels.
pub const EROSION_WIDTH: u32 = 512;
/// Height of the erosion grid in pixels.
pub const EROSION_HEIGHT: u32 = 256;

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

/// Runs the droplets on `map`. Droplets only start on land (`z >= waterline`)
/// and drop everything they carry when they reach water.
fn simulate(map: &mut HeightMap, waterline: f32, droplets: usize, seed: u64) {
    let p = &PHYSICS;
    let brush = brush(p.radius);
    let mut rng = StdRng::seed_from_u64(seed);
    let land: Vec<usize> = (0..map.z.len())
        .filter(|&i| map.z[i] >= waterline)
        .collect();
    if land.is_empty() {
        return;
    }

    for _ in 0..droplets {
        let start = land[rng.random_range(0..land.len())];
        let mut px = (start % map.w) as f32 + rng.random_range(0.0..1.0);
        let mut py = (start / map.w) as f32 + rng.random_range(0.0..1.0);
        let (mut dx, mut dy) = (0.0f32, 0.0f32);
        let (mut speed, mut water, mut sediment) = (1.0f32, 1.0f32, 0.0f32);

        for _ in 0..p.max_steps {
            let (height, gx, gy) = map.sample(px, py);
            if height < waterline {
                map.deposit(px, py, sediment);
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
                let amount = ((capacity - sediment) * p.erode).min(-dh);
                let (cx, cy) = (old_x.floor() as i64, old_y.floor() as i64);
                for &(ox, oy, wgt) in &brush {
                    let i = map.idx(cx + ox, cy + oy);
                    let taken = (amount * wgt).min(map.z[i] - waterline).max(0.0);
                    map.z[i] -= taken;
                    sediment += taken;
                }
            }

            speed = (speed * speed - dh * p.gravity).max(0.0).sqrt();
            water *= 1.0 - p.evaporate;
        }
    }
}

/// Erodes a full-resolution, row-major `elevation` map in place and returns
/// the height change of each pixel.
///
/// `waterline` is the absolute elevation of the sea surface. `strength` scales
/// the number of droplets (1.0 = one per erosion-grid pixel; 0 disables).
pub fn erode(
    elevation: &mut [f32],
    width: u32,
    height: u32,
    waterline: f32,
    strength: f32,
    seed: u32,
) -> Vec<f32> {
    let (w, h) = (width as usize, height as usize);
    // Never upsample: small maps erode at their own resolution.
    let (ew, eh) = (
        EROSION_WIDTH.min(width) as usize,
        EROSION_HEIGHT.min(height) as usize,
    );

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
    let mut map = HeightMap {
        w: ew,
        h: eh,
        z: original.clone(),
    };
    let droplets = (strength.max(0.0) * (ew * eh) as f32) as usize;
    simulate(&mut map, waterline, droplets, seed as u64);

    // Interpolate the height change back onto every full-resolution pixel.
    let delta = HeightMap {
        w: ew,
        h: eh,
        z: map.z.iter().zip(&original).map(|(a, b)| a - b).collect(),
    };
    let mut change = vec![0.0f32; w * h];
    for y in 0..h {
        // Pixel centre in erosion-grid coordinates (cell centres at +0.5).
        let ey = (y as f32 + 0.5) * eh as f32 / h as f32 - 0.5;
        for x in 0..w {
            let ex = (x as f32 + 0.5) * ew as f32 / w as f32 - 0.5;
            let (d, _, _) = delta.sample(ex, ey);
            change[y * w + x] = d;
            elevation[y * w + x] = (elevation[y * w + x] + d).clamp(-1.0, 1.0);
        }
    }
    change
}

#[cfg(test)]
mod tests {
    use super::*;

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
        erode(&mut a, 128, 64, 0.0, 0.5, 1);
        erode(&mut b, 128, 64, 0.0, 0.5, 1);
        assert_eq!(a, b);
    }

    #[test]
    fn erosion_lowers_peaks_and_never_digs_below_the_sea() {
        let mut map = island(128, 64);
        let before = map.clone();
        let change = erode(&mut map, 128, 64, 0.0, 1.0, 2);
        assert!(change.iter().any(|&d| d < 0.0), "nothing eroded");
        for (b, a) in before.iter().zip(&map) {
            if *b >= 0.0 {
                assert!(*a >= -1e-3, "land dug below the waterline: {b} -> {a}");
            }
        }
    }

    #[test]
    fn zero_strength_changes_nothing() {
        let mut map = island(64, 32);
        let before = map.clone();
        erode(&mut map, 64, 32, 0.0, 0.0, 3);
        assert_eq!(map, before);
    }
}
