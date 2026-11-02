//! River outlets: deltas and estuaries.
//!
//! Large rivers reaching the sea end in one of two ways:
//!
//! - **Delta**, where the coast is low: the river splits some way upstream of
//!   its mouth into distributaries that fan out to the sea, and the land
//!   between them is a flat, wet delta plain.
//! - **Estuary**, where the coast is higher: the river widens into a funnel
//!   towards its mouth (drawn only, see [`crate::rivers::tapers`]).
//!
//! The choice depends on the flow, on how low the land is near the mouth,
//! and on a hash of the seed and the river, so it is deterministic.
//! Distributaries never cross another river, and the delta plain never moves
//! the coastline.
use crate::geo::Grid;
use crate::rivers::{Mouth, RiverLine, SegmentIndex};
use noise::{NoiseFn, Perlin};

/// Smallest mouth flow with a delta or an estuary, km².
const OUTLET_MIN_FLOW_KM2: f32 = 800_000.0;
/// Highest ground (above the waterline) at the delta's apex: a delta needs a
/// low coastal plain.
const DELTA_MAX_APEX: f32 = 0.012;
/// Chance that a low-lying mouth gets a delta, and that another one gets an
/// estuary.
const DELTA_CHANCE: f64 = 0.8;
const ESTUARY_CHANCE: f64 = 0.6;
/// Distance from the apex to the mouth for a 1 M km² river, km; it grows with
/// `√flow`, within the bounds.
const DELTA_KM: f64 = 120.0;
const DELTA_KM_RANGE: (f64, f64) = (60.0, 300.0);
/// Angle between the main channel and a distributary at the apex, degrees.
const FAN_DEGREES: f64 = 40.0;
/// Share of the river's flow taken by each distributary.
const BRANCH_SHARE: f32 = 0.2;
/// Half-width of the delta plain around its channels, in delta lengths.
const PLAIN_SPREAD: f64 = 0.35;
/// Lowest delta plain above the waterline (as the valley beds of
/// [`crate::carve`]).
const PLAIN_FLOOR: f32 = 0.002;
/// Moisture added on the delta plain (weight 1).
const PLAIN_MOISTURE: f32 = 0.3;

/// A delta: its channels, by index in the river lines, and its length.
pub struct Delta {
    /// The main river and its distributaries.
    pub channels: Vec<usize>,
    /// Index of the apex on the main river.
    pub apex: usize,
    pub length_km: f64,
}

/// Flow of a river at its mouth: the largest along it (the very last point,
/// in the sea, only carries the minimum).
pub(crate) fn mouth_flow(line: &RiverLine) -> f32 {
    line.flow_km2.iter().fold(0.0, |m, &f| m.max(f))
}

/// Uniform `[0, 1)` from a seed and a river id.
fn unit(seed: u32, id: usize, salt: u64) -> f64 {
    let mut z = (seed as u64) ^ (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
}

/// Gives the large rivers of `lines` their outlet: marks estuaries, and adds
/// the distributaries of deltas (appended to `lines`). `above` is the height
/// above the waterline of every pixel of `grid`.
pub fn outlets(lines: &mut Vec<RiverLine>, grid: &Grid, above: &[f32], seed: u32) -> Vec<Delta> {
    let (w, h) = (grid.width as i64, grid.height as i64);
    let at = |p: [f64; 2]| {
        let (x, y) = (p[0].floor() as i64, p[1].floor() as i64);
        (y >= 0 && y < h).then(|| above[(y * w + x.rem_euclid(w)) as usize])
    };
    let dy_km = grid.dy_km() as f64;
    let noise = Perlin::new(seed.wrapping_add(1200));
    let mut deltas = Vec::new();
    let candidates: Vec<usize> = (0..lines.len())
        .filter(|&i| {
            let l = &lines[i];
            l.mouth == Mouth::Sea
                && l.source.is_none()
                && l.points.len() > 4
                && mouth_flow(l) >= OUTLET_MIN_FLOW_KM2
        })
        .collect();

    // Points never change here, lines are only appended: one index, kept
    // up to date, serves every branch.
    let mut index = SegmentIndex::new(lines);
    for id in candidates {
        let (points, flow) = (lines[id].points.clone(), lines[id].flow_km2.clone());
        let n = points.len();
        let q = mouth_flow(&lines[id]) as f64;
        let length_km = (DELTA_KM * (q / 1e6).sqrt()).clamp(DELTA_KM_RANGE.0, DELTA_KM_RANGE.1);
        let km = |a: [f64; 2], b: [f64; 2]| {
            let row = (a[1].max(0.0) as u32).min(grid.height - 1);
            let dx = (b[0] - a[0]) * grid.dx_km(row) as f64;
            let dy = (b[1] - a[1]) * dy_km;
            (dx * dx + dy * dy).sqrt()
        };
        // Apex: `length_km` upstream of the mouth, along the river.
        let mut apex = n - 1;
        let mut along = 0.0;
        while apex > 1 && along < length_km {
            along += km(points[apex - 1], points[apex]);
            apex -= 1;
        }
        let low = at(points[apex]).is_some_and(|e| e < DELTA_MAX_APEX);
        let roll = unit(seed, id, 1);
        if !low || roll >= DELTA_CHANCE || along < 0.5 * length_km {
            if !low && roll < ESTUARY_CHANCE {
                lines[id].estuary = true;
            }
            continue;
        }

        // Distributaries fan out from the apex, then bend back towards the
        // sea direction, wandering a little.
        let row = (points[apex][1].max(0.0) as u32).min(grid.height - 1);
        let dx_km = grid.dx_km(row) as f64;
        let to_km = |p: [f64; 2]| [p[0] * dx_km, p[1] * dy_km];
        let to_px = |p: [f64; 2]| [p[0] / dx_km, p[1] / dy_km];
        let (s, m) = (to_km(points[apex]), to_km(points[n - 1]));
        let len = ((m[0] - s[0]).powi(2) + (m[1] - s[1]).powi(2))
            .sqrt()
            .max(1e-9);
        let seaward = [(m[0] - s[0]) / len, (m[1] - s[1]) / len];
        let branches = if q >= 3e6 { 2 } else { 1 };
        let side = if unit(seed, id, 2) < 0.5 { 1.0 } else { -1.0 };
        let angles: Vec<f64> = match branches {
            1 => vec![side * FAN_DEGREES],
            _ => vec![-FAN_DEGREES, FAN_DEGREES],
        };
        let step_km = 0.5 * dy_km.min(dx_km);
        let mut added = 0;
        for (b, angle) in angles.into_iter().enumerate() {
            let rad = angle.to_radians();
            let mut heading = [
                seaward[0] * rad.cos() - seaward[1] * rad.sin(),
                seaward[0] * rad.sin() + seaward[1] * rad.cos(),
            ];
            let mut p = s;
            let mut path = vec![points[apex]];
            let mut reached = false;
            let mut travelled = 0.0;
            while travelled < 3.0 * length_km {
                // Bend back towards the sea direction, with a slow wander.
                let wander = 0.15
                    * noise.get([
                        travelled / (0.5 * length_km),
                        id as f64 * 1.7 + b as f64 * 5.3,
                    ]);
                let pull = 0.6 * step_km / length_km;
                heading = [
                    heading[0] * (1.0 - pull) + seaward[0] * pull - heading[1] * wander * 0.1,
                    heading[1] * (1.0 - pull) + seaward[1] * pull + heading[0] * wander * 0.1,
                ];
                let norm = (heading[0].powi(2) + heading[1].powi(2)).sqrt();
                heading = [heading[0] / norm, heading[1] / norm];
                p = [p[0] + heading[0] * step_km, p[1] + heading[1] * step_km];
                travelled += step_km;
                let px = to_px(p);
                path.push(px);
                match at(px) {
                    None => break,
                    Some(e) if e < 0.0 => {
                        reached = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !reached || path.len() < 4 {
                continue;
            }
            // Never cross another river (the apex lies on the main one).
            let crossing = path.windows(2).enumerate().any(|(k, seg)| {
                !index
                    .crossings(lines, usize::MAX, k, seg[0], seg[1])
                    .is_empty()
            });
            if crossing {
                continue;
            }
            let share = flow[apex] * BRANCH_SHARE;
            let branch = RiverLine {
                flow_km2: vec![share; path.len()],
                points: path,
                parent: None,
                mouth: Mouth::Sea,
                source: Some(id),
                estuary: false,
            };
            index.add(lines.len(), &branch);
            lines.push(branch);
            added += 1;
        }
        if added == 0 {
            continue;
        }
        // The main channel carries what the distributaries do not.
        let keep = 1.0 - BRANCH_SHARE * added as f32;
        for f in &mut lines[id].flow_km2[apex + 1..] {
            *f *= keep;
        }
        let first = lines.len() - added;
        deltas.push(Delta {
            channels: std::iter::once(id).chain(first..lines.len()).collect(),
            apex,
            length_km,
        });
    }
    deltas
}

/// Flattens the delta plains into low, wet land, and returns their weight
/// in `[0, 1]` for every pixel. `above` is the height above the waterline,
/// lowered in place but never below [`PLAIN_FLOOR`]; land stays land.
pub fn plains(
    deltas: &[Delta],
    lines: &[RiverLine],
    grid: &Grid,
    above: &mut [f32],
    moisture: &mut [f32],
) -> Vec<f32> {
    let (w, h) = (grid.width as i64, grid.height as i64);
    let dy_km = grid.dy_km() as f64;
    let mut weight = vec![0f32; above.len()];
    for delta in deltas {
        let radius = PLAIN_SPREAD * delta.length_km;
        for (c, &id) in delta.channels.iter().enumerate() {
            // The main river from its apex, and every distributary.
            let from = if c == 0 { delta.apex } else { 0 };
            for p in &lines[id].points[from..] {
                let row = (p[1].max(0.0) as u32).min(grid.height - 1);
                let dx_km = grid.dx_km(row) as f64;
                let (rx, ry) = (
                    (radius / dx_km).ceil() as i64,
                    (radius / dy_km).ceil() as i64,
                );
                let (cx, cy) = (p[0].floor() as i64, p[1].floor() as i64);
                for y in (cy - ry).max(0)..=(cy + ry).min(h - 1) {
                    for x in cx - rx..=cx + rx {
                        let d = (((x as f64 + 0.5 - p[0]) * dx_km).powi(2)
                            + ((y as f64 + 0.5 - p[1]) * dy_km).powi(2))
                        .sqrt();
                        if d >= radius {
                            continue;
                        }
                        let i = (y * w + x.rem_euclid(w)) as usize;
                        let t = ((d / radius - 0.5) / 0.5).clamp(0.0, 1.0);
                        let v = (1.0 - t * t * (3.0 - 2.0 * t)) as f32;
                        weight[i] = weight[i].max(v);
                    }
                }
            }
        }
    }
    for ((a, m), &v) in above.iter_mut().zip(moisture.iter_mut()).zip(&weight) {
        if v > 0.0 && *a >= 0.0 {
            let flat = PLAIN_FLOOR.min(*a);
            *a = *a + (flat - *a) * v;
            *m = (*m + PLAIN_MOISTURE * v).clamp(-1.0, 1.0);
        }
    }
    weight
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A low coastal plain east of x = 300, sea beyond, and a river flowing
    /// east to the coast.
    fn coast() -> (Grid, Vec<f32>, Vec<RiverLine>) {
        let grid = Grid {
            width: 800,
            height: 400,
            circumference_km: 4_000.0,
        };
        let above: Vec<f32> = (0..800 * 400)
            .map(|i| {
                let x = (i % 800) as f32;
                if x < 300.0 {
                    0.005 + (300.0 - x) * 1e-5
                } else {
                    -0.01
                }
            })
            .collect();
        let river = RiverLine {
            points: (100..=300).map(|x| [x as f64 + 0.5, 200.5]).collect(),
            flow_km2: vec![2e6; 201],
            parent: None,
            mouth: Mouth::Sea,
            source: None,
            estuary: false,
        };
        (grid, above, vec![river])
    }

    #[test]
    fn deltas_branch_off_the_river_and_reach_the_sea() {
        let (grid, above, base) = coast();
        // Some seed gives this river a delta.
        let (mut lines, deltas) = (0..20)
            .map(|seed| {
                let mut lines = base.clone();
                let deltas = outlets(&mut lines, &grid, &above, seed);
                (lines, deltas)
            })
            .find(|(_, d)| !d.is_empty())
            .expect("no delta for any seed");
        let delta = &deltas[0];
        assert!(delta.channels.len() >= 2);
        let main = lines[0].clone();
        for &id in &delta.channels[1..] {
            let branch = &lines[id];
            assert_eq!(branch.source, Some(0));
            assert_eq!(branch.points[0], main.points[delta.apex]);
            let end = branch.points[branch.points.len() - 1];
            assert!(above[(end[1] as usize) * 800 + end[0] as usize] < 0.0);
        }
        // The main channel gave part of its flow away.
        assert!(main.flow_km2[main.flow_km2.len() - 1] < main.flow_km2[0]);
        // The plain is flattened, wetter, and still land.
        let mut flat = above.clone();
        let mut moisture = vec![0.0; above.len()];
        let weight = plains(&deltas, &lines, &grid, &mut flat, &mut moisture);
        for i in 0..above.len() {
            assert_eq!(flat[i] >= 0.0, above[i] >= 0.0);
            assert!(flat[i] <= above[i]);
            if weight[i] == 1.0 && above[i] >= 0.0 {
                assert!(flat[i] <= PLAIN_FLOOR + 1e-6 && moisture[i] > 0.0);
            }
        }
        lines.clear();
    }

    #[test]
    fn high_coasts_get_estuaries_not_deltas() {
        let (grid, mut above, base) = coast();
        for a in above.iter_mut().filter(|a| **a >= 0.0) {
            *a += 0.05;
        }
        let mut estuaries = 0;
        for seed in 0..20 {
            let mut lines = base.clone();
            assert!(outlets(&mut lines, &grid, &above, seed).is_empty());
            assert_eq!(lines.len(), 1);
            estuaries += usize::from(lines[0].estuary);
        }
        assert!(estuaries > 0);
    }
}
