//! Area-weighted statistics and distances over the grid.

use crate::grid::Grid;
use science::units::Metre;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// The value below which `share` of the sphere's area lies: `share` 0 is
/// the minimum, 1 the maximum. Cells are weighted by their solid angle.
///
/// # Panics
///
/// Panics when `values` does not hold one value per cell.
pub fn area_quantile(grid: &Grid, values: &[f32], share: f64) -> f32 {
    assert_eq!(values.len(), grid.cell_count(), "one value per cell");
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_unstable_by(|&a, &b| values[a].total_cmp(&values[b]));
    let total: f64 = (0..values.len()).map(|k| grid.area(k)).sum();
    let target = share.clamp(0.0, 1.0) * total;
    let mut below = 0.0;
    for &k in &order {
        below += grid.area(k);
        if below >= target {
            return values[k];
        }
    }
    values[*order.last().expect("a grid has cells")]
}

/// The mean of `values` weighted by cell area.
///
/// # Panics
///
/// Panics when `values` does not hold one value per cell.
pub fn area_mean(grid: &Grid, values: &[f32]) -> f32 {
    assert_eq!(values.len(), grid.cell_count(), "one value per cell");
    let (sum, total) = values
        .iter()
        .enumerate()
        .fold((0.0, 0.0), |(s, t), (k, &v)| {
            let a = grid.area(k);
            (s + a * f64::from(v), t + a)
        });
    (sum / total) as f32
}

/// A distance as a totally ordered key for the priority queue.
#[derive(PartialEq)]
struct Key(f64);

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Great-circle distance, in metres, from each cell to the nearest source
/// cell: a multi-source Dijkstra over the neighbours, each edge being the
/// arc between the two cell centres on a sphere of `radius`. Cells that no
/// source reaches (no source at all, or farther than `reach` when one is
/// given: the pass stops expanding there) are infinite.
///
/// # Panics
///
/// Panics when `sources` does not hold one flag per cell.
pub fn distance_to(grid: &Grid, radius: Metre, sources: &[bool], reach: Option<Metre>) -> Vec<f32> {
    let reach = reach.map_or(f64::INFINITY, |r| r.0);
    assert_eq!(sources.len(), grid.cell_count(), "one flag per cell");
    let mut best = vec![f64::INFINITY; sources.len()];
    let mut queue = BinaryHeap::new();
    for (k, _) in sources.iter().enumerate().filter(|(_, s)| **s) {
        best[k] = 0.0;
        queue.push(Reverse((Key(0.0), k)));
    }
    while let Some(Reverse((Key(d), k))) = queue.pop() {
        if d > best[k] {
            continue;
        }
        let p = grid.point(k);
        for j in grid.neighbours(k) {
            let q = grid.point(j);
            let cos: f64 = p.iter().zip(q).map(|(a, b)| a * b).sum();
            let edge = radius.0 * cos.clamp(-1.0, 1.0).acos();
            if d + edge <= reach && d + edge < best[j] {
                best[j] = d + edge;
                queue.push(Reverse((Key(d + edge), j)));
            }
        }
    }
    best.into_iter().map(|d| d as f32).collect()
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
