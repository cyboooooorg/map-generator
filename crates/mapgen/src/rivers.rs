//! Vector rivers: the drainage network turned into smooth, meandering
//! polylines whose drawn width grows with their flow.
//!
//! 1. **Tracing.** River pixels are chained along their receivers into
//!    polylines, from head to mouth. At each confluence the main stem follows
//!    the largest tributary; the others become lines whose mouth is a point
//!    of their parent. Lines end at the sea, in a lake or on their parent.
//! 2. **Shaping.** Each line is smoothed (Chaikin corner cutting), resampled,
//!    and displaced sideways by noise along its length: meanders whose
//!    wavelength and amplitude scale with the river width, plus a slower
//!    wander. Guards keep the result hydrologically sound: no offset that
//!    climbs the valley side, no tighter bend than the line allows, a gap to
//!    other rivers, loops cut, and tributaries snapped onto their displaced
//!    parent.
//!
//! Lengths are set in reference pixels, 1/1920 of the planet's circumference
//! (the default map width), so the shapes do not depend on the map
//! resolution. Positions are map pixel coordinates (x east, y south, pixel
//! centres at +0.5), with x unwrapped along a line: it may leave `[0, width)`
//! where a river crosses the longitude seam.
use crate::geo::Grid;
use crate::hydrology::Hydrology;
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize};

pub use crate::hydrology::RIVER_MIN_FLOW_KM2;
/// Map width the reference pixel is defined at.
const REFERENCE_WIDTH: f64 = 1920.0;

/// Where a river ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Mouth {
    Sea,
    Lake,
    /// Joins its parent river.
    River,
}

/// One river polyline, from head to mouth.
#[derive(Clone, Debug)]
pub struct RiverLine {
    /// Map pixel coordinates, x unwrapped (see the module documentation).
    pub points: Vec<[f64; 2]>,
    /// Drainage area at each point, km².
    pub flow_km2: Vec<f32>,
    /// The line this one flows into, for `Mouth::River`.
    pub parent: Option<usize>,
    pub mouth: Mouth,
}

/// Drawn width of a river in reference pixels: hydraulic geometry
/// (width ∝ √flow), exaggerated about 10–20× so rivers read at map scale,
/// capped for the largest ones.
pub fn width_ref_px(flow_km2: f32) -> f64 {
    (0.9 * (flow_km2.max(RIVER_MIN_FLOW_KM2) as f64 / RIVER_MIN_FLOW_KM2 as f64).sqrt()).min(4.0)
}

/// Map pixels per reference pixel for a map `width` pixels wide.
pub fn scale(width: u32) -> f64 {
    width as f64 / REFERENCE_WIDTH
}

// ── Tracing ───────────────────────────────────────────────────────────────────

/// Chains river pixels into polylines following the receivers.
pub fn trace(grid: &Grid, hydro: &Hydrology) -> Vec<RiverLine> {
    let n = grid.pixel_count();
    let w = grid.width as i64;
    let mut donors: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in (0..n).filter(|&i| hydro.river[i]) {
        if let Some(j) = hydro.receiver[i] {
            donors[j].push(i);
        }
    }
    let centre = |i: usize| ((i as i64 % w) as f64 + 0.5, (i as i64 / w) as f64 + 0.5);

    let mut lines: Vec<RiverLine> = Vec::new();
    // (lowest river pixel of the line, parent line, pixel it flows into)
    let mut stack: Vec<(usize, Option<usize>, Option<usize>)> = (0..n)
        .filter(|&i| hydro.river[i])
        .filter(|&i| hydro.receiver[i].is_none_or(|j| !hydro.river[j]))
        .map(|i| (i, None, hydro.receiver[i]))
        .collect();
    // Deterministic order: largest rivers first.
    stack.sort_by(|a, b| hydro.flow_km2[a.0].total_cmp(&hydro.flow_km2[b.0]));

    while let Some((start, parent, end)) = stack.pop() {
        let id = lines.len();
        let mut chain = vec![start];
        let mut p = start;
        loop {
            let mut up: Vec<usize> = donors[p].clone();
            if up.is_empty() {
                break;
            }
            // Main stem: the largest donor; the others start tributaries.
            up.sort_by(|a, b| {
                hydro.flow_km2[*b]
                    .total_cmp(&hydro.flow_km2[*a])
                    .then(a.cmp(b))
            });
            for &d in up[1..].iter().rev() {
                stack.push((d, Some(id), Some(p)));
            }
            p = up[0];
            chain.push(p);
        }
        chain.reverse();
        let mut pixels = chain;
        if let Some(e) = end {
            pixels.push(e);
        }
        // Unwrap x along the line so it never jumps across the seam.
        let mut points = Vec::with_capacity(pixels.len());
        let (mut x, y) = centre(pixels[0]);
        points.push([x, y]);
        for k in 1..pixels.len() {
            let (x1, y1) = centre(pixels[k]);
            let (x0, _) = centre(pixels[k - 1]);
            let mut dx = x1 - x0;
            if dx > w as f64 / 2.0 {
                dx -= w as f64;
            } else if dx < -(w as f64) / 2.0 {
                dx += w as f64;
            }
            x += dx;
            points.push([x, y1]);
        }
        let mouth = match (parent, end) {
            (Some(_), _) => Mouth::River,
            (None, Some(e)) if hydro.lake[e] => Mouth::Lake,
            _ => Mouth::Sea,
        };
        lines.push(RiverLine {
            flow_km2: pixels
                .iter()
                .map(|&i| hydro.flow_km2[i].max(RIVER_MIN_FLOW_KM2))
                .collect(),
            points,
            parent,
            mouth,
        });
    }
    lines
}

// ── Shaping ───────────────────────────────────────────────────────────────────

/// Chaikin corner cutting, endpoints kept.
fn chaikin(points: &[[f64; 2]], flow: &[f32], iterations: usize) -> (Vec<[f64; 2]>, Vec<f32>) {
    let (mut p, mut f) = (points.to_vec(), flow.to_vec());
    for _ in 0..iterations {
        if p.len() < 3 {
            break;
        }
        let mut np = vec![p[0]];
        let mut nf = vec![f[0]];
        for k in 0..p.len() - 1 {
            let (a, b) = (p[k], p[k + 1]);
            if k > 0 {
                np.push([0.75 * a[0] + 0.25 * b[0], 0.75 * a[1] + 0.25 * b[1]]);
                nf.push(0.75 * f[k] + 0.25 * f[k + 1]);
            }
            if k + 2 < p.len() {
                np.push([0.25 * a[0] + 0.75 * b[0], 0.25 * a[1] + 0.75 * b[1]]);
                nf.push(0.25 * f[k] + 0.75 * f[k + 1]);
            }
        }
        np.push(p[p.len() - 1]);
        nf.push(f[f.len() - 1]);
        p = np;
        f = nf;
    }
    (p, f)
}

/// Resamples a polyline every `step` pixels; returns points, flows and arc lengths.
fn resample(points: &[[f64; 2]], flow: &[f32], step: f64) -> (Vec<[f64; 2]>, Vec<f32>, Vec<f64>) {
    let mut out = vec![points[0]];
    let mut out_flow = vec![flow[0]];
    let mut arc = vec![0.0];
    let (mut total, mut next) = (0.0, step);
    for k in 1..points.len() {
        let (a, b) = (points[k - 1], points[k]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        while len > 0.0 && next <= total + len {
            let t = (next - total) / len;
            out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            out_flow.push(flow[k - 1] + (flow[k] - flow[k - 1]) * t as f32);
            arc.push(next);
            next += step;
        }
        total += len;
    }
    let last = points[points.len() - 1];
    if out
        .last()
        .is_none_or(|p| (p[0] - last[0]).abs() + (p[1] - last[1]).abs() > 1e-6)
    {
        out.push(last);
        out_flow.push(flow[flow.len() - 1]);
        arc.push(total);
    }
    (out, out_flow, arc)
}

/// Bilinear sample of a row-major field at a map pixel position (x wraps).
fn sample(field: &[f32], width: u32, height: u32, p: [f64; 2]) -> f32 {
    let (w, h) = (width as f64, height as f64);
    let x = p[0] - 0.5;
    let y = (p[1] - 0.5).clamp(0.0, h - 1.001);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
    let at = |xx: f64, yy: f64| field[yy as usize * width as usize + xx.rem_euclid(w) as usize];
    let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1.0, y0) * fx;
    let bottom = at(x0, y0 + 1.0) * (1.0 - fx) + at(x0 + 1.0, y0 + 1.0) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// Do segments ab and cd cross (strictly)?
fn crosses(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    orient(c, d, a) * orient(c, d, b) < 0.0 && orient(a, b, c) * orient(a, b, d) < 0.0
}

/// Where two river segments cross: (line, segment index, crossing point).
type Crossing = (usize, usize, [f64; 2]);

/// Spatial index of the segments of a set of lines, by integer cell.
struct SegmentIndex {
    cells: std::collections::HashMap<(i64, i64), Vec<(usize, usize)>>,
}

impl SegmentIndex {
    fn new(lines: &[RiverLine]) -> Self {
        let mut cells: std::collections::HashMap<(i64, i64), Vec<(usize, usize)>> =
            std::collections::HashMap::new();
        for (id, line) in lines.iter().enumerate() {
            for (s, pair) in line.points.windows(2).enumerate() {
                for cell in Self::cells_of(pair[0], pair[1]) {
                    cells.entry(cell).or_default().push((id, s));
                }
            }
        }
        Self { cells }
    }

    fn cells_of(a: [f64; 2], b: [f64; 2]) -> impl Iterator<Item = (i64, i64)> {
        let (x0, x1) = (a[0].min(b[0]).floor() as i64, a[0].max(b[0]).floor() as i64);
        let (y0, y1) = (a[1].min(b[1]).floor() as i64, a[1].max(b[1]).floor() as i64);
        (x0..=x1).flat_map(move |x| (y0..=y1).map(move |y| (x, y)))
    }

    /// Segments of other lines (or of the same line, not adjacent) crossing ab.
    fn crossings(
        &self,
        lines: &[RiverLine],
        id: usize,
        s: usize,
        a: [f64; 2],
        b: [f64; 2],
    ) -> Vec<Crossing> {
        let mut found = Vec::new();
        for cell in Self::cells_of(a, b) {
            for &(other, t) in self.cells.get(&cell).into_iter().flatten() {
                // Skip adjacent segments, and entries left stale by truncation.
                if (other == id && t.abs_diff(s) < 2) || t + 1 >= lines[other].points.len() {
                    continue;
                }
                let (c, d) = (lines[other].points[t], lines[other].points[t + 1]);
                if crosses(a, b, c, d) && !found.iter().any(|f: &Crossing| f.0 == other && f.1 == t)
                {
                    found.push((other, t, intersection(a, b, c, d)));
                }
            }
        }
        found
    }
}

/// Intersection point of two crossing segments.
fn intersection(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> [f64; 2] {
    let (r, q) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let denominator = r[0] * q[1] - r[1] * q[0];
    let t = ((c[0] - a[0]) * q[1] - (c[1] - a[1]) * q[0]) / denominator;
    [a[0] + t * r[0], a[1] + t * r[1]]
}

/// Ends tributaries at the first river they cross before their own mouth:
/// where two tributaries join their parent side by side, their snapped ends
/// can cross. The truncated line then flows into the river it met.
fn end_at_first_crossing(lines: &mut [RiverLine], tail: f64) {
    let index = SegmentIndex::new(lines);
    // Lines that receive tributaries keep their length, except for a short
    // overshoot across their own parent (a few samples at the very end).
    let mut has_children = vec![false; lines.len()];
    for line in lines.iter() {
        if let Some(p) = line.parent {
            has_children[p] = true;
        }
    }
    for id in 0..lines.len() {
        let Some(parent) = lines[id].parent else {
            continue;
        };
        let reach = if has_children[id] { tail / 12.0 } else { tail };
        // Only the confluence tail, where the snapping moved the line.
        let points = &lines[id].points;
        let mut from_mouth = 0.0;
        let mut first = points.len().saturating_sub(1);
        while first > 0 && from_mouth < reach {
            let (a, b) = (points[first - 1], points[first]);
            from_mouth += ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            first -= 1;
        }
        let mut cut = None;
        for s in first..lines[id].points.len().saturating_sub(1) {
            let (a, b) = (lines[id].points[s], lines[id].points[s + 1]);
            let hit = index
                .crossings(lines, id, s, a, b)
                .into_iter()
                .find(|&(other, _, _)| other != id && (!has_children[id] || other == parent));
            if let Some((other, _, point)) = hit {
                cut = Some((s, other, point));
                break;
            }
        }
        if let Some((s, other, point)) = cut {
            let line = &mut lines[id];
            line.points.truncate(s + 1);
            line.flow_km2.truncate(s + 1);
            line.points.push(point);
            line.flow_km2.push(line.flow_km2[s]);
            line.parent = Some(other);
        }
    }
}

/// Smooths the traced lines and adds meanders.
///
/// `filled` is the depression-filled surface the water flows on: offsets that
/// would climb it are shrunk, so rivers stay in their valleys.
pub fn shape(lines: &[RiverLine], grid: &Grid, filled: &[f32], seed: u32) -> Vec<RiverLine> {
    let (width, height) = (grid.width, grid.height);
    let k = scale(width);
    let perlin = Perlin::new(seed.wrapping_add(900));
    let step = 0.4 * k;
    let half = 6usize;
    let valley_tolerance = 0.001;

    // Which line owns each pixel, for the gap to other rivers.
    let (wi, hi) = (width as i64, height as i64);
    let mut owner = vec![-1i32; (wi * hi) as usize];
    for (id, line) in lines.iter().enumerate() {
        for p in &line.points[..line.points.len().saturating_sub(1)] {
            let (x, y) = (
                (p[0].floor() as i64).rem_euclid(wi),
                (p[1].floor() as i64).clamp(0, hi - 1),
            );
            owner[(y * wi + x) as usize] = id as i32;
        }
    }

    let mut out: Vec<RiverLine> = Vec::with_capacity(lines.len());
    // Undisplaced resampled centrelines, aligned with `out`, for snapping.
    let mut bases: Vec<Vec<[f64; 2]>> = Vec::with_capacity(lines.len());
    for (id, line) in lines.iter().enumerate() {
        let (smooth, smooth_flow) = chaikin(&line.points, &line.flow_km2, 3);
        let (base, flow, arc) = resample(&smooth, &smooth_flow, step);
        let m = base.len();
        let total = arc[m - 1];
        // Normals of a strongly averaged copy, so offsets do not follow pixel jitter.
        let normals: Vec<[f64; 2]> = (0..m)
            .map(|i| {
                let (a, b) = (base[i.saturating_sub(half)], base[(i + half).min(m - 1)]);
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let len = (dx * dx + dy * dy).sqrt().max(1e-9);
                [-dy / len, dx / len]
            })
            .collect();
        // Confluence zone: near its mouth a tributary may approach its parent.
        let parent_lambda = line.parent.map_or(0.0, |p| {
            (7.0 * width_ref_px(*lines[p].flow_km2.last().unwrap_or(&0.0))).clamp(6.0, 40.0) * k
        });

        let mut offsets = Vec::with_capacity(m);
        for i in 0..m {
            let w = width_ref_px(flow[i]);
            let lambda = (7.0 * w).clamp(6.0, 40.0) * k;
            let amplitude = (2.5 * w).clamp(1.0, 3.0) * k;
            let wander = 1.5 * k;
            // Phase along the arc from the mouth: stable when the head changes.
            let from_mouth = total - arc[i];
            let meander = perlin.get([from_mouth / lambda, id as f64 * 3.17 + 0.5]) * 2.0;
            let slow =
                perlin.get([from_mouth / (3.5 * lambda) + 11.3, id as f64 * 1.73 + 0.25]) * 2.0;
            let edge = (arc[i].min(from_mouth) / (0.5 * lambda)).clamp(0.0, 1.0);
            let taper = edge * edge * (3.0 - 2.0 * edge);
            let mut o = (amplitude * meander + wander * slow) * taper;

            // Never offset more than half the local bend radius.
            let (a, b) = (i.saturating_sub(2 * half), (i + 2 * half).min(m - 1));
            let (na, nb) = (normals[a], normals[b]);
            let turn = (na[0] * nb[0] + na[1] * nb[1]).clamp(-1.0, 1.0).acos();
            if turn > 1e-3 {
                let limit = 0.5 * (arc[b] - arc[a]) / turn;
                o = o.clamp(-limit, limit);
            }
            // Valley constraint: shrink the offset while it climbs.
            let z0 = sample(filled, width, height, base[i]);
            for _ in 0..6 {
                let q = [
                    base[i][0] + normals[i][0] * o,
                    base[i][1] + normals[i][1] * o,
                ];
                if sample(filled, width, height, q) - z0 <= valley_tolerance {
                    break;
                }
                o *= 0.6;
            }
            // Keep a gap to other rivers (the parent too, outside the confluence).
            if o != 0.0 {
                let in_confluence = from_mouth < 2.0 * parent_lambda;
                let (px, py) = (base[i][0].floor() as i64, base[i][1].floor() as i64);
                let reach = (6.0 * k).ceil() as i64;
                let mut nearest = f64::MAX;
                for dy in -reach..=reach {
                    for dx in -reach..=reach {
                        let (x, y) = ((px + dx).rem_euclid(wi), py + dy);
                        if y < 0 || y >= hi {
                            continue;
                        }
                        let other = owner[(y * wi + x) as usize];
                        let is_parent = line.parent == Some(other as usize);
                        if other >= 0 && other as usize != id && !(is_parent && in_confluence) {
                            nearest = nearest.min(((dx * dx + dy * dy) as f64).sqrt());
                        }
                    }
                }
                if nearest < f64::MAX {
                    let limit = (0.35 * (nearest - 1.0)).max(0.0);
                    o = o.clamp(-limit, limit);
                }
            }
            offsets.push(o);
        }
        // Smooth the offsets so the guards leave no kinks.
        let r = 4usize;
        let smoothed: Vec<f64> = (0..m)
            .map(|i| {
                let (a, b) = (i.saturating_sub(r), (i + r).min(m - 1));
                offsets[a..=b].iter().sum::<f64>() / (b - a + 1) as f64
            })
            .collect();
        let mut points: Vec<[f64; 2]> = (0..m)
            .map(|i| {
                [
                    base[i][0] + normals[i][0] * smoothed[i],
                    base[i][1] + normals[i][1] * smoothed[i],
                ]
            })
            .collect();

        // Snap the mouth of a tributary onto its displaced parent.
        if let Some(pid) = line.parent {
            let end = base[m - 1];
            let wf = width as f64;
            let (mut best, mut best_k, mut best_shift) = (f64::MAX, 0usize, 0.0);
            for (j, b) in bases[pid].iter().enumerate() {
                let mut dx = b[0] - end[0];
                let shift = (dx / wf).round() * wf;
                dx -= shift;
                let d = dx * dx + (b[1] - end[1]).powi(2);
                if d < best {
                    (best, best_k, best_shift) = (d, j, shift);
                }
            }
            let target = out[pid].points[best_k];
            let delta = [target[0] - best_shift - end[0], target[1] - end[1]];
            let blend = parent_lambda.max(8.0 * k);
            for (p, s) in points.iter_mut().zip(&arc) {
                let t = ((s - (total - blend)) / blend).clamp(0.0, 1.0);
                let t = t * t * (3.0 - 2.0 * t);
                p[0] += delta[0] * t;
                p[1] += delta[1] * t;
            }
        }

        // Cut any remaining local loop.
        let mut flow = flow;
        // 80 samples: 32 reference pixels, whatever the map size (the step scales).
        let window = 80;
        let mut i = 0;
        while i + 3 < points.len() {
            let cut = ((i + 2)..(i + window).min(points.len() - 1))
                .rev()
                .find(|&j| crosses(points[i], points[i + 1], points[j], points[j + 1]));
            if let Some(j) = cut {
                points.drain(i + 1..=j);
                flow.drain(i + 1..=j);
            }
            i += 1;
        }
        bases.push(if points.len() == m {
            base
        } else {
            points.clone()
        });
        out.push(RiverLine {
            points,
            flow_km2: flow,
            parent: line.parent,
            mouth: line.mouth,
        });
    }
    end_at_first_crossing(&mut out, 48.0 * k);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::generate;
    use crate::request::WorldRequest;

    fn world(seed: u32) -> crate::world::World {
        let request = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(seed),
            sea_level: Some(0.2),
            volcanic: Some(0.3),
            circumference_km: Some(40_000.0),
            axial_tilt_deg: Some(23.44),
            width: Some(480),
            height: Some(240),
            ..Default::default()
        };
        generate(&request.resolve(&mut rand::rng()).unwrap().params).world
    }

    #[test]
    fn width_grows_with_flow_and_is_capped() {
        assert!((width_ref_px(RIVER_MIN_FLOW_KM2) - 0.9).abs() < 1e-9);
        assert!(width_ref_px(1e6) > width_ref_px(5e5));
        assert_eq!(width_ref_px(1e9), 4.0);
    }

    #[test]
    fn chaikin_keeps_the_endpoints() {
        let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [2.0, 1.0]];
        let (p, f) = chaikin(&points, &[1.0, 2.0, 3.0, 4.0], 3);
        assert_eq!(p[0], points[0]);
        assert_eq!(*p.last().unwrap(), points[3]);
        assert_eq!(p.len(), f.len());
    }

    #[test]
    fn rivers_end_at_the_sea_a_lake_or_their_parent() {
        let w = world(3);
        assert!(!w.rivers.is_empty());
        for line in &w.rivers {
            assert_eq!(line.points.len(), line.flow_km2.len());
            assert_eq!(line.parent.is_some(), line.mouth == Mouth::River);
            if let Some(p) = line.parent {
                // The mouth lies on the parent line.
                let end = line.points[line.points.len() - 1];
                let parent = &w.rivers[p];
                let gap = parent
                    .points
                    .iter()
                    .map(|q| {
                        let dx = (q[0] - end[0]).rem_euclid(w.width as f64);
                        let dx = dx.min(w.width as f64 - dx);
                        (dx * dx + (q[1] - end[1]).powi(2)).sqrt()
                    })
                    .fold(f64::MAX, f64::min);
                assert!(gap < 0.75, "tributary mouth {gap} px from its parent");
            }
        }
    }

    #[test]
    fn rivers_do_not_cross() {
        for seed in [3, 7, 42] {
            let w = world(seed);
            let index = SegmentIndex::new(&w.rivers);
            let mut crossings = 0;
            for (id, line) in w.rivers.iter().enumerate() {
                for (s, pair) in line.points.windows(2).enumerate() {
                    crossings += index
                        .crossings(&w.rivers, id, s, pair[0], pair[1])
                        .iter()
                        // A tributary's last segment ends on its parent.
                        .filter(|&&(other, t, _)| {
                            let end = |l: usize, k: usize| k + 2 >= w.rivers[l].points.len();
                            !(w.rivers[id].parent == Some(other) && end(id, s))
                                && !(w.rivers[other].parent == Some(id) && end(other, t))
                        })
                        .count();
                }
            }
            assert_eq!(crossings, 0, "seed {seed}");
        }
    }

    #[test]
    fn shaping_is_deterministic() {
        let (a, b) = (world(11), world(11));
        assert_eq!(a.rivers.len(), b.rivers.len());
        for (x, y) in a.rivers.iter().zip(&b.rivers) {
            assert_eq!(x.points, y.points);
        }
    }
}
