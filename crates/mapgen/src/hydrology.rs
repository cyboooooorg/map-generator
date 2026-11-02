//! Rivers and lakes from rainfall runoff.
//!
//! 1. **Depression filling** (priority-flood, Barnes et al. 2014): starting
//!    from the sea, the terrain is flooded upwards so every land pixel gets a
//!    path down to the sea. Pits are filled to their spill level, plus a tiny
//!    slope so water can cross the filled flats.
//! 2. **Flow routing**: each land pixel drains to its steepest lower
//!    neighbour on the filled surface.
//! 3. **Accumulation**: every pixel adds its runoff (area × rain) and passes
//!    the total downstream. The flow of a pixel is the wet-equivalent area, in
//!    km², that drains through it.
//! 4. **Lakes** are filled pits deep enough and fed by enough flow; **rivers**
//!    are pixels whose flow exceeds a threshold.
use crate::geo::Grid;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Slope added across filled flats so every pixel drains somewhere.
const FLAT_EPSILON: f32 = 1e-5;
/// Filled pits shallower than this (elevation units) stay dry land.
const LAKE_MIN_DEPTH: f32 = 0.004;
/// A filled pit becomes a lake only if this much flow (km²) reaches it.
const LAKE_MIN_FLOW_KM2: f32 = 40_000.0;
/// A pixel is a river when this much flow (km²) passes through it.
const RIVER_MIN_FLOW_KM2: f32 = 120_000.0;

/// Rivers and lakes of a world, row-major on its grid.
pub struct Hydrology {
    /// Wet-equivalent drainage area in km² flowing through each pixel
    /// (0 on the sea).
    pub flow_km2: Vec<f32>,
    pub lake: Vec<bool>,
    pub river: Vec<bool>,
}

/// Share of the rain that runs off, from moisture in `[-1, 1]`: none in a
/// desert, all of it in the wettest places.
fn runoff(moisture: f32) -> f32 {
    let wet = ((moisture + 1.0) / 2.0).clamp(0.0, 1.0);
    wet * wet
}

#[derive(PartialEq)]
struct Node(f32, usize);
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
pub fn hydrology(grid: &Grid, height: &[f32], moisture: &[f32]) -> Hydrology {
    let n = grid.pixel_count();
    let sea: Vec<bool> = height.iter().map(|&z| z < 0.0).collect();

    // ── 1. Priority-flood from the sea ────────────────────────────────────────
    let mut filled = height.to_vec();
    let mut done = vec![false; n];
    let mut order = Vec::with_capacity(n); // ascending filled height
    let mut heap = BinaryHeap::new();
    for i in (0..n).filter(|&i| sea[i]) {
        done[i] = true;
        heap.push(Node(filled[i], i));
    }
    if heap.is_empty() {
        // No sea at all: drain everything towards the lowest pixel.
        let lowest = (0..n)
            .min_by(|&a, &b| height[a].total_cmp(&height[b]))
            .unwrap_or(0);
        done[lowest] = true;
        heap.push(Node(filled[lowest], lowest));
    }
    while let Some(Node(z, i)) = heap.pop() {
        order.push(i);
        let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
        for (nx, ny, _) in grid.neighbours(x, y) {
            let j = grid.index(nx, ny);
            if !done[j] {
                done[j] = true;
                filled[j] = filled[j].max(z + FLAT_EPSILON);
                heap.push(Node(filled[j], j));
            }
        }
    }

    // ── 2. Steepest-descent receivers on the filled surface ───────────────────
    let receiver: Vec<Option<usize>> = (0..n)
        .map(|i| {
            if sea[i] {
                return None;
            }
            let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
            grid.neighbours(x, y)
                .map(|(nx, ny, d)| {
                    let j = grid.index(nx, ny);
                    (j, (filled[i] - filled[j]) / d)
                })
                .filter(|&(_, slope)| slope > 0.0)
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(j, _)| j)
        })
        .collect();

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
        if let Some(j) = receiver[i]
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
        let h = hydrology(&g, &ramp(&g), &vec![1.0; g.pixel_count()]);
        let row = 16 * 64;
        assert_eq!(h.flow_km2[row + 5], 0.0, "sea has no flow");
        assert!(h.flow_km2[row + 17] > h.flow_km2[row + 40]);
    }

    #[test]
    fn deserts_have_no_runoff() {
        let g = grid();
        let h = hydrology(&g, &ramp(&g), &vec![-1.0; g.pixel_count()]);
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
        let h = hydrology(&g, &height, &vec![1.0; g.pixel_count()]);
        assert!(h.lake[16 * 64 + 40]);
        assert!(!h.lake[16 * 64 + 20]);
    }
}
