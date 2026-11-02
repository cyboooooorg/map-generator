//! Rivers and lakes from rainfall runoff.
//!
//! 1. **Depression filling** (priority-flood, Barnes et al. 2014): starting
//!    from the sea, the terrain is flooded upwards so every land pixel gets a
//!    path down to the sea. Pits are filled to their spill level, plus a tiny
//!    slope so water can cross the filled flats. That slope varies with a
//!    noise, so paths across flats wander instead of running straight to the
//!    spill point; it never changes heights above the flats, so divides and
//!    lakes do not move.
//! 2. **Flow routing**: each land pixel drains to a lower neighbour on the
//!    filled surface, chosen by steepness and by the descent of a
//!    large-scale routing noise, so networks branch instead of running
//!    parallel down even slopes.
//! 3. **Accumulation**: every pixel adds its runoff (area × rain) and passes
//!    the total downstream. The flow of a pixel is the wet-equivalent area, in
//!    km², that drains through it.
//! 4. **Lakes** are filled pits deep enough and fed by enough flow; **rivers**
//!    are pixels whose flow exceeds a threshold.
use crate::geo::Grid;
use noise::{NoiseFn, Perlin};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Slope added across filled flats so every pixel drains somewhere. Each
/// step costs between 1 and 3 times this, following the flat noise.
const FLAT_EPSILON: f32 = 1e-5;
/// Frequency of the flat noise on the unit sphere, at 1920 pixels of map
/// width (scaled with the width so features stay a few pixels wide).
const FLAT_NOISE_FREQUENCY: f64 = 200.0;
/// Wavelength of the routing noise, km.
const ROUTE_WAVELENGTH_KM: f64 = 1200.0;
/// Weight of the routing noise against the slope when choosing between
/// lower neighbours (the steepest scores 1).
const ROUTE_WEIGHT: f32 = 1.0;
/// Filled pits shallower than this (elevation units) stay dry land.
const LAKE_MIN_DEPTH: f32 = 0.004;
/// A filled pit becomes a lake only if this much flow (km²) reaches it.
const LAKE_MIN_FLOW_KM2: f32 = 40_000.0;
/// A pixel is a river when this much flow (km²) passes through it.
pub const RIVER_MIN_FLOW_KM2: f32 = 120_000.0;

/// Rivers and lakes of a world, row-major on its grid.
pub struct Hydrology {
    /// Wet-equivalent drainage area in km² flowing through each pixel
    /// (0 on the sea).
    pub flow_km2: Vec<f32>,
    pub lake: Vec<bool>,
    pub river: Vec<bool>,
    /// The pixel each land pixel drains to (None on the sea); see
    /// [`Hydrology::receiver`]. 32-bit indices keep large maps in memory.
    receivers: Vec<Option<u32>>,
    /// The depression-filled surface the water flows on.
    pub filled: Vec<f32>,
}

impl Hydrology {
    /// The pixel `i` drains to, or None on the sea.
    pub fn receiver(&self, i: usize) -> Option<usize> {
        self.receivers[i].map(|j| j as usize)
    }
}

/// Noise in `[0, 1]` per pixel that sets how costly each step across a
/// filled flat is, so the flood (and the flow paths) meander.
fn flat_noise(grid: &Grid, seed: u32) -> Vec<f32> {
    let perlin = Perlin::new(seed.wrapping_add(800));
    let frequency = FLAT_NOISE_FREQUENCY * grid.width as f64 / 1920.0;
    (0..grid.pixel_count())
        .map(|i| {
            let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
            let lon = (x as f64 + 0.5) / grid.width as f64 * std::f64::consts::TAU;
            let lat = grid.latitude(y);
            let p = [
                lat.cos() * lon.cos() * frequency,
                lat.cos() * lon.sin() * frequency,
                lat.sin() * frequency,
            ];
            ((perlin.get(p) as f32 + 1.0) / 2.0).clamp(0.0, 1.0)
        })
        .collect()
}

/// Large-scale noise guiding the choice between lower neighbours, so river
/// networks branch like trees instead of running parallel down even slopes.
/// Its wavelength is set in km, so networks do not depend on the map size.
fn routing_noise(grid: &Grid, seed: u32) -> Vec<f32> {
    let perlin = Perlin::new(seed.wrapping_add(810));
    let radius_km = grid.circumference_km as f64 / std::f64::consts::TAU;
    let frequency = radius_km / ROUTE_WAVELENGTH_KM;
    (0..grid.pixel_count())
        .map(|i| {
            let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
            let lon = (x as f64 + 0.5) / grid.width as f64 * std::f64::consts::TAU;
            let lat = grid.latitude(y);
            let p = [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()];
            let (a, b) = (
                perlin.get(p.map(|c| c * frequency)),
                perlin.get(p.map(|c| c * frequency * 2.3 + 7.1)),
            );
            (a + 0.4 * b) as f32
        })
        .collect()
}

/// Share of the rain that runs off, from moisture in `[-1, 1]`: none in a
/// desert, all of it in the wettest places.
fn runoff(moisture: f32) -> f32 {
    let wet = ((moisture + 1.0) / 2.0).clamp(0.0, 1.0);
    wet * wet
}

#[derive(PartialEq)]
struct Node(f32, u32);
impl Eq for Node {}
impl Ord for Node {
    // Reversed: BinaryHeap is a max-heap, we want the lowest first.
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Computes rivers and lakes.
///
/// `height` is the elevation above the waterline (negative on the sea);
/// `moisture` in `[-1, 1]` sets how much rain runs off.
pub fn hydrology(grid: &Grid, height: &[f32], moisture: &[f32], seed: u32) -> Hydrology {
    let n = grid.pixel_count();
    let noise = flat_noise(grid, seed);
    let sea: Vec<bool> = height.iter().map(|&z| z < 0.0).collect();

    // ── 1. Priority-flood from the sea ────────────────────────────────────────
    let mut filled = height.to_vec();
    let mut done = vec![false; n];
    let mut order = Vec::with_capacity(n); // ascending filled height
    let mut heap = BinaryHeap::new();
    for i in (0..n).filter(|&i| sea[i]) {
        done[i] = true;
        heap.push(Node(filled[i], i as u32));
    }
    if heap.is_empty() {
        // No sea at all: drain everything towards the lowest pixel.
        let lowest = (0..n)
            .min_by(|&a, &b| height[a].total_cmp(&height[b]))
            .unwrap_or(0);
        done[lowest] = true;
        heap.push(Node(filled[lowest], lowest as u32));
    }
    while let Some(Node(z, i)) = heap.pop() {
        order.push(i);
        let i = i as usize;
        let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
        for (nx, ny, _) in grid.neighbours(x, y) {
            let j = grid.index(nx, ny);
            if !done[j] {
                done[j] = true;
                // Only raises pixels below the flood level (flats): real
                // slopes keep their height.
                filled[j] = filled[j].max(z + FLAT_EPSILON * (1.0 + 2.0 * noise[j]));
                heap.push(Node(filled[j], j as u32));
            }
        }
    }

    // ── 2. Receivers on the filled surface ────────────────────────────────────
    // Always a lower neighbour (so drainage stays acyclic and the filled
    // surface valid), chosen by steepness and, as a tie-breaker on even
    // slopes, by the descent of a large-scale routing noise: paths that would
    // run straight and parallel down a regular slope converge into the
    // noise's valleys and join, as river networks do.
    let route = routing_noise(grid, seed);
    let receivers: Vec<Option<u32>> = (0..n)
        .map(|i| {
            if sea[i] {
                return None;
            }
            let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
            let lower: Vec<(usize, f32, f32)> = grid
                .neighbours(x, y)
                .map(|(nx, ny, d)| {
                    let j = grid.index(nx, ny);
                    (j, (filled[i] - filled[j]) / d, (route[i] - route[j]) / d)
                })
                .filter(|&(_, slope, _)| slope > 0.0)
                .collect();
            let steepest = lower.iter().fold(0.0f32, |m, l| m.max(l.1));
            let pull = lower
                .iter()
                .fold(0.0f32, |m, l| m.max(l.2.abs()))
                .max(1e-12);
            lower
                .iter()
                .map(|&(j, slope, down)| (j, slope / steepest + ROUTE_WEIGHT * down / pull))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(j, _)| j as u32)
        })
        .collect();
    drop(route);

    // ── 3. Accumulate runoff downstream (highest pixels first) ────────────────
    let mut flow_km2: Vec<f32> = (0..n)
        .map(|i| {
            if sea[i] {
                return 0.0;
            }
            let y = i as u32 / grid.width;
            grid.dx_km(y) * grid.dy_km() * runoff(moisture[i])
        })
        .collect();
    for &i in order.iter().rev() {
        let i = i as usize;
        if let Some(j) = receivers[i].map(|j| j as usize)
            && !sea[j]
        {
            flow_km2[j] += flow_km2[i];
        }
    }

    // ── 4. Lakes and rivers ───────────────────────────────────────────────────
    let lake = lakes(grid, height, &filled, &flow_km2, &sea);
    let river = (0..n)
        .map(|i| !sea[i] && !lake[i] && flow_km2[i] >= RIVER_MIN_FLOW_KM2)
        .collect();
    Hydrology {
        flow_km2,
        lake,
        river,
        receivers,
        filled,
    }
}

/// Filled pits (connected pixels flooded deeper than [`LAKE_MIN_DEPTH`]) that
/// receive at least [`LAKE_MIN_FLOW_KM2`] at their deepest inflow point.
fn lakes(grid: &Grid, height: &[f32], filled: &[f32], flow: &[f32], sea: &[bool]) -> Vec<bool> {
    let n = grid.pixel_count();
    let flooded: Vec<bool> = (0..n)
        .map(|i| !sea[i] && filled[i] - height[i] > LAKE_MIN_DEPTH)
        .collect();
    let mut lake = vec![false; n];
    let mut seen = vec![false; n];
    for start in 0..n {
        if !flooded[start] || seen[start] {
            continue;
        }
        // Collect the connected flooded region.
        let mut region = vec![start];
        seen[start] = true;
        let mut k = 0;
        while k < region.len() {
            let i = region[k];
            k += 1;
            let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
            for (nx, ny, _) in grid.neighbours(x, y) {
                let j = grid.index(nx, ny);
                if flooded[j] && !seen[j] {
                    seen[j] = true;
                    region.push(j);
                }
            }
        }
        let inflow = region.iter().map(|&i| flow[i]).fold(0.0f32, f32::max);
        if inflow >= LAKE_MIN_FLOW_KM2 {
            region.iter().for_each(|&i| lake[i] = true);
        }
    }
    lake
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid {
            width: 64,
            height: 32,
            circumference_km: 40_000.0,
        }
    }

    /// Sea in the west quarter, land rising eastwards.
    fn ramp(g: &Grid) -> Vec<f32> {
        (0..g.pixel_count())
            .map(|i| {
                let x = (i % g.width as usize) as f32;
                if x < 16.0 { -0.5 } else { (x - 16.0) * 0.01 }
            })
            .collect()
    }

    #[test]
    fn flow_grows_downstream() {
        let g = grid();
        let h = hydrology(&g, &ramp(&g), &vec![1.0; g.pixel_count()], 1);
        let row = 16 * 64;
        assert_eq!(h.flow_km2[row + 5], 0.0, "sea has no flow");
        assert!(h.flow_km2[row + 17] > h.flow_km2[row + 40]);
    }

    #[test]
    fn deserts_have_no_runoff() {
        let g = grid();
        let h = hydrology(&g, &ramp(&g), &vec![-1.0; g.pixel_count()], 1);
        assert!(h.flow_km2.iter().all(|&f| f == 0.0));
        assert!(!h.river.iter().any(|&r| r));
    }

    #[test]
    fn a_wet_pit_becomes_a_lake() {
        let g = grid();
        let mut height = ramp(&g);
        // Dig a basin in the middle of the land.
        for y in 12..20 {
            for x in 36..44 {
                height[y * 64 + x] = 0.05;
            }
        }
        let h = hydrology(&g, &height, &vec![1.0; g.pixel_count()], 1);
        assert!(h.lake[16 * 64 + 40]);
        assert!(!h.lake[16 * 64 + 20]);
    }

    #[test]
    fn every_land_pixel_drains_to_the_sea() {
        let g = grid();
        let mut height = ramp(&g);
        // A closed basin and a long flat plateau.
        for y in 4..28 {
            for x in 30..60 {
                height[y * 64 + x] = 0.2;
            }
        }
        let h = hydrology(&g, &height, &vec![1.0; g.pixel_count()], 7);
        for start in (0..g.pixel_count()).filter(|&i| height[i] >= 0.0) {
            let mut i = start;
            let mut steps = 0;
            while let Some(j) = h.receiver(i) {
                assert!(
                    h.filled[j] < h.filled[i],
                    "flow must go down the filled surface"
                );
                i = j;
                steps += 1;
                assert!(steps <= g.pixel_count(), "cycle from {start}");
            }
            assert!(height[i] < 0.0, "pixel {start} ends on land at {i}");
        }
    }

    #[test]
    fn flat_noise_only_raises_flats() {
        let g = grid();
        let height = ramp(&g);
        let h = hydrology(&g, &height, &vec![1.0; g.pixel_count()], 3);
        // On a clean slope nothing is filled: heights are kept exactly.
        for (filled, height) in h.filled.iter().zip(&height) {
            if *height >= 0.0 {
                assert_eq!(filled, height);
            }
        }
    }
}
