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
    /// For a delta distributary: the river it branches from (its first point
    /// lies on that river). See [`crate::delta`].
    pub source: Option<usize>,
    /// The river widens into a funnel-shaped estuary towards its mouth.
    pub estuary: bool,
}

/// Nominal width of a river in reference pixels: hydraulic geometry
/// (width ∝ √flow), capped for the largest ones. It sizes the meanders; the
/// drawn width derives from it with [`draw_width_px`].
pub fn width_ref_px(flow_km2: f32) -> f64 {
    (0.9 * (flow_km2.max(RIVER_MIN_FLOW_KM2) as f64 / RIVER_MIN_FLOW_KM2 as f64).sqrt()).min(4.0)
}

/// Share of the nominal width drawn on a raster as wide as the reference map.
pub const INK: f64 = 0.5;
/// How the drawn width follows the raster resolution: `(raster / 1920)^0.67`,
/// so each doubling of resolution narrows a river by 1.26× in km, on the base
/// map and from one detail tile level to the next alike.
pub const RASTER_EXPONENT: f64 = 2.0 / 3.0;
/// Real river width: `REAL_WIDTH_KM × √(flow km²)` (about 5 km at 6 M km²).
pub const REAL_WIDTH_KM: f64 = 0.002;

/// Drawn width in pixels of a raster `raster_width` pixels around the planet
/// (the base map, or every tile of a detail level side by side), whose
/// pixels are `pixel_km` wide. Readable at every scale, it never falls below
/// the real width; `taper` thins the head of a river.
pub fn draw_width_px(flow_km2: f32, raster_width: f64, pixel_km: f64, taper: f64) -> f64 {
    let drawn =
        INK * width_ref_px(flow_km2) * (raster_width / REFERENCE_WIDTH).powf(RASTER_EXPONENT);
    let real = REAL_WIDTH_KM * (flow_km2.max(0.0) as f64).sqrt() / pixel_km;
    (drawn * taper).max(real)
}

/// Exact (box-filtered) coverage of a pixel whose centre is `d` pixels from
/// the axis of a band `width` pixels wide: a band thinner than a pixel fades
/// out instead of keeping a solid one-pixel line.
pub fn band_coverage(d: f64, width: f64) -> f32 {
    let half = width / 2.0;
    ((d + 0.5).min(half) - (d - 0.5).max(-half)).clamp(0.0, 1.0) as f32
}

/// Map pixels per reference pixel for a map `width` pixels wide.
pub fn scale(width: u32) -> f64 {
    width as f64 / REFERENCE_WIDTH
}

// ── Data file ─────────────────────────────────────────────────────────────────

/// Content of `raw_data/rivers.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RiversFile {
    pub version: u32,
    /// Width of the rivers, see [`WidthLaw`].
    pub width_law: WidthLaw,
    pub rivers: Vec<RiverEntry>,
}

/// Parameters of the width of rivers. The nominal width is
/// `w0_ref_px × (flow / q0_km2)^exponent`, capped at `max_ref_px`, in
/// reference pixels (1/`reference_width_px` of the circumference). On a
/// raster `W` pixels around the planet a river is drawn
/// `ink × nominal × (W / reference_width_px)^raster_exponent` pixels wide,
/// and never thinner than `real_km_per_sqrt_km2 × √flow` km.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct WidthLaw {
    pub w0_ref_px: f32,
    pub q0_km2: f32,
    pub exponent: f32,
    pub max_ref_px: f32,
    pub reference_width_px: u32,
    /// Absent from version 1 files, whose rivers were drawn at the nominal width.
    #[serde(default = "one")]
    pub ink: f32,
    #[serde(default = "one")]
    pub raster_exponent: f32,
    #[serde(default)]
    pub real_km_per_sqrt_km2: f32,
}

fn one() -> f32 {
    1.0
}

/// One river polyline of `rivers.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RiverEntry {
    pub id: u32,
    /// The river this one flows into, when `mouth` is `river`.
    pub parent: Option<u32>,
    pub mouth: Mouth,
    /// For a delta distributary, the river it branches from (version 3).
    #[serde(default)]
    pub source: Option<u32>,
    /// Whether the river ends in an estuary (version 3).
    #[serde(default)]
    pub estuary: bool,
    /// `[longitude°, latitude°, flow km²]` from head to mouth. Longitudes are
    /// continuous along a river, so they may leave `[0, 360)` at the seam.
    pub points: Vec<[f32; 3]>,
}

impl RiversFile {
    /// Converts polylines in map pixels to the file format.
    pub fn from_lines(lines: &[RiverLine], width: u32, height: u32) -> Self {
        let (w, h) = (width as f64, height as f64);
        Self {
            version: 3,
            width_law: WidthLaw {
                w0_ref_px: 0.9,
                q0_km2: RIVER_MIN_FLOW_KM2,
                exponent: 0.5,
                max_ref_px: 4.0,
                reference_width_px: REFERENCE_WIDTH as u32,
                ink: INK as f32,
                raster_exponent: RASTER_EXPONENT as f32,
                real_km_per_sqrt_km2: REAL_WIDTH_KM as f32,
            },
            rivers: lines
                .iter()
                .enumerate()
                .map(|(id, line)| RiverEntry {
                    id: id as u32,
                    parent: line.parent.map(|p| p as u32),
                    mouth: line.mouth,
                    source: line.source.map(|p| p as u32),
                    estuary: line.estuary,
                    points: line
                        .points
                        .iter()
                        .zip(&line.flow_km2)
                        .map(|(p, &f)| {
                            [
                                (p[0] / w * 360.0) as f32,
                                (90.0 - p[1] / h * 180.0) as f32,
                                f.round(),
                            ]
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// Converts back to polylines in the pixels of a `width` × `height` map.
    pub fn to_lines(&self, width: u32, height: u32) -> Vec<RiverLine> {
        let (w, h) = (width as f64, height as f64);
        self.rivers
            .iter()
            .map(|r| RiverLine {
                points: r
                    .points
                    .iter()
                    .map(|p| [p[0] as f64 / 360.0 * w, (90.0 - p[1] as f64) / 180.0 * h])
                    .collect(),
                flow_km2: r.points.iter().map(|p| p[2]).collect(),
                parent: r.parent.map(|p| p as usize),
                mouth: r.mouth,
                source: r.source.map(|p| p as usize),
                estuary: r.estuary,
            })
            .collect()
    }
}

// ── Tracing ───────────────────────────────────────────────────────────────────

/// Chains river pixels into polylines following the receivers.
pub fn trace(grid: &Grid, hydro: &Hydrology) -> Vec<RiverLine> {
    let n = grid.pixel_count();
    let w = grid.width as i64;
    // Only river pixels have donors that matter: keyed by pixel rather
    // than one (mostly empty) list per pixel of the map.
    let mut donors: std::collections::HashMap<usize, Vec<usize>> = Default::default();
    for i in (0..n).filter(|&i| hydro.river[i]) {
        if let Some(j) = hydro.receiver(i) {
            donors.entry(j).or_default().push(i);
        }
    }
    let centre = |i: usize| ((i as i64 % w) as f64 + 0.5, (i as i64 / w) as f64 + 0.5);

    let mut lines: Vec<RiverLine> = Vec::new();
    // (lowest river pixel of the line, parent line, pixel it flows into)
    let mut stack: Vec<(usize, Option<usize>, Option<usize>)> = (0..n)
        .filter(|&i| hydro.river[i])
        .filter(|&i| hydro.receiver(i).is_none_or(|j| !hydro.river[j]))
        .map(|i| (i, None, hydro.receiver(i)))
        .collect();
    // Deterministic order: largest rivers first.
    stack.sort_by(|a, b| hydro.flow_km2[a.0].total_cmp(&hydro.flow_km2[b.0]));

    while let Some((start, parent, end)) = stack.pop() {
        let id = lines.len();
        let mut chain = vec![start];
        let mut p = start;
        loop {
            let mut up: Vec<usize> = donors.get(&p).cloned().unwrap_or_default();
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
            source: None,
            estuary: false,
        });
    }
    lines
}

// ── Drawing ───────────────────────────────────────────────────────────────────

/// Width factor of a line at each point: thinning from 0.55 at its head to 1
/// eight reference pixels downstream, so rivers do not start abruptly, and
/// for an estuary a funnel widening to [`ESTUARY_WIDENING`] at the mouth.
pub fn tapers(line: &RiverLine, map_width: u32) -> Vec<f64> {
    let k = scale(map_width);
    let n = line.points.len();
    let mut arc = vec![0.0; n];
    for i in 1..n {
        let (p, q) = (line.points[i], line.points[i - 1]);
        arc[i] = arc[i - 1] + ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
    }
    let total = arc.last().copied().unwrap_or(0.0);
    let mouth_flow = crate::delta::mouth_flow(line) as f64;
    let funnel = ESTUARY_REF_PX * k * (mouth_flow / 1e6).sqrt().clamp(0.5, 2.0);
    (0..n)
        .map(|i| {
            let t = (arc[i] / (8.0 * k)).clamp(0.0, 1.0);
            let head = 0.55 + 0.45 * t * t * (3.0 - 2.0 * t);
            let mouth = if line.estuary {
                let u = (1.0 - (total - arc[i]) / funnel).clamp(0.0, 1.0);
                1.0 + (ESTUARY_WIDENING - 1.0) * u * u
            } else {
                1.0
            };
            head * mouth
        })
        .collect()
}

/// Width of an estuary at the mouth, in river widths.
pub const ESTUARY_WIDENING: f64 = 3.0;
/// Length of the estuary funnel for a 1 M km² river, in reference pixels
/// (about 60 km on an Earth-sized planet); it grows with `√flow`.
const ESTUARY_REF_PX: f64 = 3.0;

/// Drawn width in map pixels at each point of a line, on a map `map_width`
/// pixels around a planet of `circumference_km`.
pub fn widths_px(line: &RiverLine, map_width: u32, circumference_km: f64) -> Vec<f64> {
    let pixel_km = circumference_km / map_width as f64;
    tapers(line, map_width)
        .into_iter()
        .zip(&line.flow_km2)
        .map(|(taper, &flow)| draw_width_px(flow, map_width as f64, pixel_km, taper))
        .collect()
}

/// Anti-aliased coverage in `[0, 1]` of the rivers drawn on a `width` ×
/// `height` map of a planet of `circumference_km`, row-major. Overlapping
/// rivers take the maximum, so confluences do not darken. Lines crossing the
/// longitude seam wrap.
pub fn coverage(lines: &[RiverLine], width: u32, height: u32, circumference_km: f64) -> Vec<f32> {
    let mut cov = vec![0f32; (width * height) as usize];
    let wrap = width as f64;
    for line in lines {
        let widths = widths_px(line, width, circumference_km);
        for (k, s) in line.points.windows(2).enumerate() {
            let (wa, wb) = (widths[k], widths[k + 1]);
            for shift in [-wrap, 0.0, wrap] {
                let a = [s[0][0] + shift, s[0][1]];
                let b = [s[1][0] + shift, s[1][1]];
                let r = wa.max(wb) / 2.0 + 1.0;
                let (x0, x1) = ((a[0].min(b[0]) - r).floor(), (a[0].max(b[0]) + r).ceil());
                let (y0, y1) = ((a[1].min(b[1]) - r).floor(), (a[1].max(b[1]) + r).ceil());
                if x1 < 0.0 || y1 < 0.0 || x0 >= wrap || y0 >= height as f64 {
                    continue;
                }
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let len2 = (dx * dx + dy * dy).max(1e-12);
                for py in (y0.max(0.0) as u32)..(y1.min(height as f64) as u32) {
                    for px in (x0.max(0.0) as u32)..(x1.min(wrap) as u32) {
                        let c = [px as f64 + 0.5, py as f64 + 0.5];
                        let t = (((c[0] - a[0]) * dx + (c[1] - a[1]) * dy) / len2).clamp(0.0, 1.0);
                        let q = [a[0] + t * dx, a[1] + t * dy];
                        let d = ((c[0] - q[0]).powi(2) + (c[1] - q[1]).powi(2)).sqrt();
                        let v = band_coverage(d, wa + (wb - wa) * t);
                        let i = (py * width + px) as usize;
                        cov[i] = cov[i].max(v);
                    }
                }
            }
        }
    }
    cov
}

/// Coverage from which a pixel counts as a river in the biome layer.
pub const RIVER_COVERAGE: f32 = 0.5;

// ── Shaping ───────────────────────────────────────────────────────────────────

/// Moving average of a polyline over `radius` points on each side, the
/// endpoints kept (the mouth must stay where it joins).
fn moving_average(points: &[[f64; 2]], radius: usize) -> Vec<[f64; 2]> {
    let n = points.len();
    (0..n)
        .map(|i| {
            if i == 0 || i + 1 == n {
                return points[i];
            }
            let r = radius.min(i).min(n - 1 - i);
            let window = &points[i - r..=i + r];
            let sum = window
                .iter()
                .fold([0.0, 0.0], |s, p| [s[0] + p[0], s[1] + p[1]]);
            [sum[0] / window.len() as f64, sum[1] / window.len() as f64]
        })
        .collect()
}

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
pub(crate) fn crosses(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    orient(c, d, a) * orient(c, d, b) < 0.0 && orient(a, b, c) * orient(a, b, d) < 0.0
}

/// Where two river segments cross: (line, segment index, crossing point).
pub(crate) type Crossing = (usize, usize, [f64; 2]);

/// Spatial index of the segments of a set of lines, by integer cell.
pub(crate) struct SegmentIndex {
    cells: std::collections::HashMap<(i64, i64), Vec<(usize, usize)>>,
}

impl SegmentIndex {
    pub(crate) fn new(lines: &[RiverLine]) -> Self {
        let mut index = Self {
            cells: std::collections::HashMap::new(),
        };
        for (id, line) in lines.iter().enumerate() {
            index.add(id, line);
        }
        index
    }

    /// Indexes the segments of `line`, numbered `id`.
    pub(crate) fn add(&mut self, id: usize, line: &RiverLine) {
        for (s, pair) in line.points.windows(2).enumerate() {
            for cell in Self::cells_of(pair[0], pair[1]) {
                self.cells.entry(cell).or_default().push((id, s));
            }
        }
    }

    fn cells_of(a: [f64; 2], b: [f64; 2]) -> impl Iterator<Item = (i64, i64)> {
        let (x0, x1) = (a[0].min(b[0]).floor() as i64, a[0].max(b[0]).floor() as i64);
        let (y0, y1) = (a[1].min(b[1]).floor() as i64, a[1].max(b[1]).floor() as i64);
        (x0..=x1).flat_map(move |x| (y0..=y1).map(move |y| (x, y)))
    }

    /// Segments of other lines (or of the same line, not adjacent) crossing ab.
    pub(crate) fn crossings(
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

/// Ends every river at the first other river it crosses, which it then
/// joins: two tributaries snapped side by side onto their parent, a meander
/// swinging across the parent upstream of the confluence, or two rivers
/// reaching the coast at the same place.
///
/// A line is only cut downstream of its own tributaries' mouths, so none of
/// them is left dangling; a line joined by a cut protects the joining point in
/// turn. The very end of a tributary, which lies on its parent, is not a
/// crossing.
fn end_at_first_crossing(lines: &mut [RiverLine]) {
    let index = SegmentIndex::new(lines);
    // First segment of each line that may be cut: past its tributaries' mouths.
    let mut protected = vec![0usize; lines.len()];
    for line in lines.iter() {
        if let Some(p) = line.parent {
            let k = nearest_point(&lines[p].points, line.points[line.points.len() - 1]);
            protected[p] = protected[p].max(k);
        }
    }
    for id in 0..lines.len() {
        let last = lines[id].points.len().saturating_sub(1);
        let mut cut = None;
        for s in protected[id]..last {
            let (a, b) = (lines[id].points[s], lines[id].points[s + 1]);
            let hit = index
                .crossings(lines, id, s, a, b)
                .into_iter()
                .find(|&(other, _, _)| {
                    other != id
                        && !(lines[id].parent == Some(other) && s + 1 == last)
                        && !flows_into(lines, other, id)
                });
            if let Some((other, t, point)) = hit {
                cut = Some((s, other, t, point));
                break;
            }
        }
        if let Some((s, other, t, point)) = cut {
            let line = &mut lines[id];
            line.points.truncate(s + 1);
            line.flow_km2.truncate(s + 1);
            line.points.push(point);
            line.flow_km2.push(line.flow_km2[s]);
            line.parent = Some(other);
            line.mouth = Mouth::River;
            protected[other] = protected[other].max(t + 1);
        }
    }
}

/// Does line `from` reach line `to` through its chain of parents? Joining
/// such a line would make the two rivers flow into each other.
fn flows_into(lines: &[RiverLine], from: usize, to: usize) -> bool {
    let mut line = from;
    for _ in 0..lines.len() {
        match lines[line].parent {
            Some(p) if p == to => return true,
            Some(p) => line = p,
            None => return false,
        }
    }
    false
}

/// Index of the point of `points` closest to `p`.
fn nearest_point(points: &[[f64; 2]], p: [f64; 2]) -> usize {
    let distance = |q: &[f64; 2]| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2);
    (0..points.len())
        .min_by(|&i, &j| distance(&points[i]).total_cmp(&distance(&points[j])))
        .unwrap_or(0)
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
        // Average out the pixel staircase first: on small maps a pixel is up
        // to two reference pixels, and corner cutting alone leaves zigzags.
        let unstepped = moving_average(&line.points, 2);
        let (smooth, smooth_flow) = chaikin(&unstepped, &line.flow_km2, 3);
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
            source: line.source,
            estuary: line.estuary,
        });
    }
    end_at_first_crossing(&mut out);
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
    fn coverage_is_widest_on_large_rivers() {
        let line = |flow: f32| RiverLine {
            points: (0..40).map(|x| [x as f64 + 0.5, 10.5]).collect(),
            flow_km2: vec![flow; 40],
            parent: None,
            mouth: Mouth::Sea,
            source: None,
            estuary: false,
        };
        let covered = |l: RiverLine| {
            coverage(&[l], 1920, 20, 40_000.0)
                .iter()
                .filter(|&&c| c >= RIVER_COVERAGE)
                .count()
        };
        assert!(covered(line(5e6)) > covered(line(RIVER_MIN_FLOW_KM2)));
        // The seam wraps: a line just west of x = 0 covers the east edge.
        let west = RiverLine {
            points: vec![[-3.0, 10.5], [0.5, 10.5]],
            flow_km2: vec![5e6; 2],
            parent: None,
            mouth: Mouth::Sea,
            source: None,
            estuary: false,
        };
        assert!(coverage(&[west], 1920, 20, 40_000.0)[10 * 1920 + 1918] > 0.5);
    }

    #[test]
    fn drawn_width_barely_jumps_between_rasters() {
        // From a base map to the next raster twice as wide (the first detail
        // level), a river narrows by 2^(2/3) = 1.26× in km, whatever the base.
        let km = |flow: f32, raster: f64| {
            let pixel_km = 40_000.0 / raster;
            draw_width_px(flow, raster, pixel_km, 1.0) * pixel_km
        };
        for base in [1920.0, 4096.0, 8192.0] {
            for flow in [2e5, 1e6, 4e6] {
                let ratio = km(flow, base) / km(flow, 2.0 * base);
                assert!(
                    (1.0..=1.3).contains(&ratio),
                    "{base} px, {flow} km²: {ratio}"
                );
            }
        }
        // Close up, the real width takes over.
        assert!((km(4e6, 1e7) - REAL_WIDTH_KM * 2000.0).abs() < 1e-9);
    }

    #[test]
    fn sub_pixel_coverage_adds_up_to_the_width() {
        for width in [0.2, 0.6, 1.0, 2.5] {
            let total: f32 = (-5..=5).map(|d| band_coverage(d as f64 + 0.3, width)).sum();
            assert!((total as f64 - width).abs() < 1e-6, "{width}: {total}");
        }
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
    fn the_data_file_round_trips() {
        let w = world(5);
        let file = RiversFile::from_lines(&w.rivers, w.width, w.height);
        let json = serde_json::to_string(&file).unwrap();
        let back: RiversFile = serde_json::from_str(&json).unwrap();
        let lines = back.to_lines(w.width, w.height);
        assert_eq!(lines.len(), w.rivers.len());
        for (a, b) in lines.iter().zip(&w.rivers) {
            assert_eq!(a.parent, b.parent);
            assert_eq!(a.mouth, b.mouth);
            for (p, q) in a.points.iter().zip(&b.points) {
                assert!((p[0] - q[0]).abs() < 1e-3 && (p[1] - q[1]).abs() < 1e-3);
            }
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
