//! River valleys: the terrain around the vector rivers is lowered into
//! valleys with a flat floodplain, so rivers flow in the relief instead of
//! being drawn over it.
//!
//! For each river line:
//!
//! 1. **Axis.** The line averaged over one meander wavelength: the valley
//!    follows the river's course, not each of its bends, so walls do not
//!    trace every meander.
//! 2. **Bed.** The depression-filled surface along the river, never rising
//!    downstream, cut by an incision that grows with the flow, kept above the
//!    sea and above the lake it ends in.
//! 3. **Cross-section.** A flat floodplain as wide as the meander belt (and
//!    at least `FLOODPLAIN_KM × √(Q / Q0)`), then a gentle wall, blended back
//!    into the terrain towards the valley edge.
//!
//! The new elevation is the minimum of the terrain and every valley, so
//! carving never raises ground, never creates water and keeps the divides.
//! All lengths are in km, so valleys do not depend on the map resolution.
use crate::geo::Grid;
use crate::rivers::{self, Mouth, RIVER_MIN_FLOW_KM2, RiverLine};
use rayon::prelude::*;
use std::collections::HashMap;

/// Floodplain width at the smallest river flow (`RIVER_MIN_FLOW_KM2`), in km;
/// it grows with the square root of the flow (about 80 km at 4 M km²).
const FLOODPLAIN_KM: f64 = 14.0;
/// Slope of the valley walls beyond the floodplain, elevation units per km.
const WALL: f64 = 5e-5;
/// Valley half-width: `VALLEY_SPREAD ×` the floodplain half-width plus
/// `VALLEY_EXTRA_KM`.
const VALLEY_SPREAD: f64 = 4.0;
const VALLEY_EXTRA_KM: f64 = 60.0;
/// Incision of the bed below the filled surface at the smallest river flow;
/// it grows with the fourth root of the flow.
const INCISION: f64 = 6e-4;
/// Lowest bed above the waterline: more than the height detail the deep-zoom
/// tiles add near the shore, so floodplains never turn into sea.
const BED_MARGIN: f64 = 0.002;

/// Distance over which the profiles of the valley pieces near a pixel are
/// blended, km.
const BLEND_KM: f64 = 5.0;

/// The valley profile at a pixel, within one river: the distance to the
/// river's axis, and the profile of the nearest pieces, blended by distance.
struct Nearest {
    distance: f64,
    weight: f64,
    bottom: f64,
    half: f64,
    radius: f64,
    /// Distance from the river's head along its axis, km.
    from_head: f64,
}

impl Nearest {
    fn far() -> Self {
        Self {
            distance: f64::MAX,
            weight: 0.0,
            bottom: 0.0,
            half: 0.0,
            radius: 0.0,
            from_head: 0.0,
        }
    }

    /// Turns the weighted sums into the blended profile.
    fn normalise(&mut self) {
        let w = self.weight.max(1e-12);
        self.bottom /= w;
        self.half /= w;
        self.radius /= w;
        self.from_head /= w;
    }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Lowers `above` (height above the waterline, row-major on `grid`) into
/// valleys around `lines`, and returns the floodplain weight in `[0, 1]` of
/// every pixel. `filled` is the depression-filled surface in the same units;
/// pixels under the sea or in a lake are left alone.
pub fn carve(
    lines: &[RiverLine],
    grid: &Grid,
    filled: &[f32],
    lake: &[bool],
    above: &mut [f32],
) -> Vec<f32> {
    let original = above.to_vec();
    let mut floodplain = vec![0f32; above.len()];
    // Lines are carved independently and merged with min / max, which do
    // not depend on the order: the result is the same as carving them one
    // after another.
    let carved: Vec<Vec<(u32, f32, f32)>> = lines
        .par_iter()
        .map(|line| carve_line(line, grid, filled, lake, &original))
        .collect();
    for (i, height, weight) in carved.into_iter().flatten() {
        let i = i as usize;
        if height < above[i] {
            above[i] = height;
        }
        if weight > floodplain[i] {
            floodplain[i] = weight;
        }
    }
    floodplain
}

/// The valley of one river: for each pixel it reaches, its carved height
/// (never above the terrain) and its floodplain weight.
fn carve_line(
    line: &RiverLine,
    grid: &Grid,
    filled: &[f32],
    lake: &[bool],
    original: &[f32],
) -> Vec<(u32, f32, f32)> {
    let (w, h) = (grid.width as i64, grid.height as i64);
    let index = |x: i64, y: i64| (y.clamp(0, h - 1) * w + x.rem_euclid(w)) as usize;
    let at = |p: [f64; 2]| index(p[0].floor() as i64, p[1].floor() as i64);
    let dy_km = grid.dy_km() as f64;
    let mut out = Vec::new();
    {
        let n = line.points.len();
        if n < 2 {
            return out;
        }
        let row = |p: [f64; 2]| (p[1].max(0.0) as u32).min(grid.height - 1);
        let km = |a: [f64; 2], b: [f64; 2]| {
            let dx = (b[0] - a[0]) * grid.dx_km(row(a)) as f64;
            let dy = (b[1] - a[1]) * dy_km;
            (dx * dx + dy * dy).sqrt()
        };
        let mut arc = vec![0.0; n];
        for i in 1..n {
            arc[i] = arc[i - 1] + km(line.points[i - 1], line.points[i]);
        }

        // Axis: the line averaged over one meander wavelength (as in
        // `rivers::shape`), and how far the river strays from it.
        let ref_km = grid.circumference_km as f64 / 1920.0;
        let wavelength =
            |i: usize| (7.0 * rivers::width_ref_px(line.flow_km2[i])).clamp(6.0, 40.0) * ref_km;
        let window = |i: usize| {
            let half = wavelength(i) / 2.0;
            let lo = arc.partition_point(|&a| a < arc[i] - half);
            let hi = arc.partition_point(|&a| a <= arc[i] + half);
            (lo, hi.max(lo + 1))
        };
        let axis: Vec<[f64; 2]> = (0..n)
            .map(|i| {
                let (lo, hi) = window(i);
                let count = (hi - lo) as f64;
                let sum = line.points[lo..hi]
                    .iter()
                    .fold([0.0, 0.0], |s, p| [s[0] + p[0], s[1] + p[1]]);
                [sum[0] / count, sum[1] / count]
            })
            .collect();
        let stray: Vec<f64> = (0..n).map(|i| km(axis[i], line.points[i])).collect();

        // Floodplain half-width: the meander belt, at least the flow's own.
        let half_width: Vec<f64> = (0..n)
            .map(|i| {
                let (lo, hi) = window(i);
                let belt = stray[lo..hi].iter().fold(0.0f64, |m, &s| m.max(s));
                let q = line.flow_km2[i].max(RIVER_MIN_FLOW_KM2) as f64 / RIVER_MIN_FLOW_KM2 as f64;
                (0.5 * FLOODPLAIN_KM * q.sqrt()).max(1.2 * belt)
            })
            .collect();

        // Bed: the lowest of the filled surface under the river and of the
        // ground across the floodplain (the drawn river strays from the
        // drainage path by up to its meander belt), never rising downstream,
        // incised, above the sea and above the lake the river ends in.
        let floor = match line.mouth {
            Mouth::Lake => filled[at(line.points[n - 1])] as f64,
            _ => 0.0,
        }
        .max(0.0)
            + BED_MARGIN;
        let mut lowest = f64::MAX;
        let bed: Vec<f64> = (0..n)
            .map(|i| {
                let (prev, next) = (axis[i.saturating_sub(1)], axis[(i + 1).min(n - 1)]);
                let (tx, ty) = (
                    (next[0] - prev[0]) * grid.dx_km(row(axis[i])) as f64,
                    (next[1] - prev[1]) * dy_km,
                );
                let len = (tx * tx + ty * ty).sqrt().max(1e-9);
                let across = (-4..=4)
                    .map(|j| {
                        let d = half_width[i] * j as f64 / 4.0;
                        let p = [
                            axis[i][0] - ty / len * d / grid.dx_km(row(axis[i])) as f64,
                            axis[i][1] + tx / len * d / dy_km,
                        ];
                        original[at(p)] as f64
                    })
                    // The sea beside a coastal river is not its floodplain.
                    .filter(|&e| e >= 0.0)
                    .fold(f64::MAX, f64::min);
                lowest = lowest.min(filled[at(line.points[i])] as f64).min(across);
                let q = line.flow_km2[i].max(RIVER_MIN_FLOW_KM2) as f64 / RIVER_MIN_FLOW_KM2 as f64;
                lowest - INCISION * q.powf(0.25)
            })
            .collect();
        // Smooth along the river over the floodplain width: the running
        // minimum drops in steps that would draw lines across the valley.
        // A moving average of a sequence that never rises never rises.
        let bed: Vec<f64> = (0..n)
            .map(|i| {
                let lo = arc.partition_point(|&a| a < arc[i] - half_width[i]);
                let hi = arc
                    .partition_point(|&a| a <= arc[i] + half_width[i])
                    .max(lo + 1);
                let mean = bed[lo..hi].iter().sum::<f64>() / (hi - lo) as f64;
                mean.max(floor)
            })
            .collect();

        // Carve along the axis, in straight pieces short enough (half the
        // floodplain) to follow its bends. Each pixel takes the profile of
        // its nearest pieces, blended by distance: the minimum over all
        // pieces would let the round ends of lower pieces downstream bite
        // into the slope upstream (concentric arcs), and the single nearest
        // one would jump where two pieces are equally near (straight hatches).
        let mut pieces = Vec::new();
        let mut s = 0;
        while s + 1 < n {
            let mut e = s + 1;
            while e + 1 < n && arc[e] - arc[s] < 0.5 * half_width[s] {
                e += 1;
            }
            pieces.push((s, e));
            s = e;
        }
        // Pass 1: distance to the axis. Pass 2: blended profile.
        let mut nearest: HashMap<usize, Nearest> = HashMap::new();
        for pass in 0..2 {
            for &(s, e) in &pieces {
                let (a, b) = (axis[s], axis[e]);
                let radius = VALLEY_SPREAD * half_width[s].max(half_width[e]) + VALLEY_EXTRA_KM;
                let dx_km = grid.dx_km(row(a)) as f64;
                let (rx, ry) = (radius / dx_km, radius / dy_km);
                let (x0, x1) = (
                    (a[0].min(b[0]) - rx).floor() as i64,
                    (a[0].max(b[0]) + rx).ceil() as i64,
                );
                let (y0, y1) = (
                    ((a[1].min(b[1]) - ry).floor() as i64).max(0),
                    ((a[1].max(b[1]) + ry).ceil() as i64).min(h - 1),
                );
                let (sx, sy) = ((b[0] - a[0]) * dx_km, (b[1] - a[1]) * dy_km);
                let len2 = (sx * sx + sy * sy).max(1e-12);
                for py in y0..=y1 {
                    for px in x0..=x1 {
                        let i = index(px, py);
                        if original[i] < 0.0 || lake[i] {
                            continue;
                        }
                        let (cx, cy) = (
                            (px as f64 + 0.5 - a[0]) * dx_km,
                            (py as f64 + 0.5 - a[1]) * dy_km,
                        );
                        let t = ((cx * sx + cy * sy) / len2).clamp(0.0, 1.0);
                        let d = ((cx - t * sx).powi(2) + (cy - t * sy).powi(2)).sqrt();
                        if pass == 0 {
                            if d < radius {
                                let entry = nearest.entry(i).or_insert(Nearest::far());
                                entry.distance = entry.distance.min(d);
                            }
                            continue;
                        }
                        let Some(entry) = nearest.get_mut(&i) else {
                            continue;
                        };
                        // Falls to exactly 0 at 4 × BLEND_KM beyond the
                        // nearest piece, and at the piece's own radius (its
                        // box): no step anywhere.
                        let weight = ((-(d - entry.distance) / BLEND_KM).exp() - (-4.0f64).exp())
                            * (1.0 - smoothstep(0.8 * radius, radius, d));
                        if weight <= 0.0 {
                            continue;
                        }
                        let lerp = |v: &[f64]| v[s] + (v[e] - v[s]) * t;
                        entry.weight += weight;
                        entry.bottom += weight * lerp(&bed);
                        entry.half += weight * lerp(&half_width);
                        entry.radius += weight * radius;
                        entry.from_head += weight * lerp(&arc);
                    }
                }
            }
        }
        for (i, mut v) in nearest {
            v.normalise();
            let e0 = original[i] as f64;
            let valley = (v.bottom + WALL * (v.distance - v.half).max(0.0)).min(e0);
            // Blend back into the terrain towards the valley edge, and fade
            // the valley in from the river's head instead of a round bowl.
            let keep = smoothstep(0.6 * v.radius, v.radius, v.distance)
                .max(1.0 - smoothstep(0.0, v.radius, v.from_head));
            let carved = valley + (e0 - valley) * keep;
            let weight = (1.0 - smoothstep(0.6 * v.half, 1.6 * v.half, v.distance)) as f32;
            out.push((i as u32, carved as f32, weight));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::{GenerateOptions, generate_with_progress};
    use crate::request::WorldRequest;

    fn world() -> crate::world::World {
        let request = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(3),
            sea_level: Some(0.2),
            land_fraction: None,
            pressure_bar: Some(1.0),
            volcanic: Some(0.3),
            circumference_km: Some(40_000.0),
            axial_tilt_deg: Some(23.44),
            erosion: Some(1.0),
            width: Some(480),
            height: Some(240),
        };
        let params = request.resolve(&mut rand::rng()).unwrap().params;
        generate_with_progress(&params, GenerateOptions::default(), |_| {}).world
    }

    #[test]
    fn valleys_have_a_flat_floor_walls_and_untouched_surroundings() {
        // 5 km pixels; ground rising away from a straight east-flowing river
        // on row 200, and falling gently downstream.
        let grid = Grid {
            width: 800,
            height: 400,
            circumference_km: 4_000.0,
        };
        let (w, h) = (800usize, 400usize);
        let mut above: Vec<f32> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as f32, (i / w) as f32);
                0.05 - 1e-5 * x + 4e-4 * (y - 200.0).abs()
            })
            .collect();
        let filled = above.clone();
        let river = RiverLine {
            points: (100..700).map(|x| [x as f64 + 0.5, 200.5]).collect(),
            flow_km2: vec![1e6; 600],
            parent: None,
            mouth: Mouth::Sea,
            source: None,
            estuary: false,
        };
        let fp = carve(&[river], &grid, &filled, &vec![false; w * h], &mut above);
        let at = |x: usize, y: usize| above[y * w + x];
        let orig = |x: usize, y: usize| filled[y * w + x];
        // Floodplain half-width: 7 km × √(1e6 / 120 000) ≈ 20 km = 4 pixels.
        for x in [200, 400, 600] {
            assert!(at(x, 200) < orig(x, 200), "the bed is incised");
            for d in 1..=3 {
                // Flat across, up to the downstream tilt (1e-5 per pixel)
                // over the floodplain's reach.
                assert!(
                    (at(x, 200 + d) - at(x, 200)).abs() < 5e-5,
                    "flat floor at {d}"
                );
                assert!(fp[(200 + d) * w + x] > 0.9);
            }
            // Walls rise away from the floor, below the original ground.
            assert!(at(x, 210) > at(x, 204) && at(x, 210) < orig(x, 210));
            // Beyond the valley (4 × 20 + 60 km = 28 pixels), nothing changes.
            assert_eq!(at(x, 235), orig(x, 235));
            assert_eq!(fp[235 * w + x], 0.0);
        }
        // The floor never rises downstream.
        assert!(at(600, 200) <= at(400, 200) && at(400, 200) <= at(200, 200));
    }

    #[test]
    fn carving_never_raises_ground_or_creates_water() {
        let w = world();
        let waterline = w.params.sea_level + w.params.planet.rules.waterline;
        let grid = w.params.grid();
        let mut above: Vec<f32> = w.elevation.iter().map(|e| e - waterline).collect();
        let before = above.clone();
        let lake = vec![false; above.len()];
        let fp = carve(&w.rivers, &grid, &before, &lake, &mut above);
        for ((b, a), f) in before.iter().zip(&above).zip(&fp) {
            assert!(a <= b);
            assert_eq!(*a >= 0.0, *b >= 0.0);
            assert!((0.0..=1.0).contains(f));
        }
    }
}
