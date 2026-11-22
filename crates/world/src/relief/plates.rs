//! Tectonic plates on the grid: where the plates lie, how they turn, and
//! the relief their boundaries raise and sink. Ported from
//! `legacy-final:crates/mapgen/src/plates.rs` for the present only (no
//! drift).
//!
//! **Plates from subplates.** The sphere is cut into about 300 jagged
//! subplates: the cells of a jittered 3D lattice around the unit sphere,
//! looked up at a position displaced by two octaves of noise. Each
//! subplate joins the nearest of the weighted plate centres (a power
//! diagram), so plates have varied sizes and jagged borders.
//!
//! **Motion.** Each plate turns about an Euler pole of its own; its RMS
//! surface speed rises with the share of its edge where it sinks (slab
//! pull, [`science::relief::plates::slab_pull_speed`]).
//!
//! **Boundaries.** A cell next to another plate is on a boundary; what
//! kind of boundary follows from how the two plates move across it and
//! the crust on each side. The relief of each kind is a layer cake over
//! the boundaries' strengths, each layer a function of a distance field
//! ([`crate::stats::distance_to`]), so it fades continuously where one
//! boundary meets another instead of jumping on the line halfway between
//! them (see [`PlateMap::relief`]).

use crate::grid::Grid;
use crate::stats::distance_to;
use ::noise::{NoiseFn, Perlin};
use science::relief::plates::{
    BOUNDARY_WINDOW, Boundary, PLATE_COUNT, PLATE_RMS_SPEED, PLATE_WEIGHT, RELIEF_LEVELS,
    RELIEF_SPEED, SUBPLATE_FREQUENCY, SUBPLATE_WARP, SUBPLATE_WARP_FREQUENCY, arc_weight,
    boundary_height, orogeny_weight, relief_level, relief_reach, relief_strength, slab_pull_speed,
};
use science::relief::spectrum::relief_scale;
use science::units::{Metre, MetrePerSecond, MetrePerSecondSquared};
use timeline::Seed;

/// The plates of a body on its grid.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateMap {
    /// Plate of each cell.
    plate: Vec<u16>,
    /// Whether the crust of each cell is continental.
    continental: Vec<bool>,
    /// Rotation of each plate: its Euler pole times its angular speed
    /// times the body's radius, in m/s, so that the velocity of the plate
    /// at the unit vector `p` is `motion × p` in m/s, whatever the radius.
    motion: Vec<[f64; 3]>,
}

/// A boundary cell: the plates on each side, the direction across, the
/// crust on each side.
struct Edge {
    cell: usize,
    point: [f64; 3],
    /// Towards the other plate, in the tangent plane, unit.
    across: [f64; 3],
    plate: u16,
    other: u16,
    continental: bool,
    other_continental: bool,
}

impl Edge {
    /// Speed at which the two plates close across the boundary, m/s,
    /// negative where they part.
    fn convergence(&self, motion: &[[f64; 3]]) -> f64 {
        let a = cross(motion[usize::from(self.plate)], self.point);
        let b = cross(motion[usize::from(self.other)], self.point);
        dot(sub(a, b), self.across)
    }

    /// Whether the cell's plate sinks under the other one: oceanic crust
    /// under continental crust, or under the other oceanic plate where
    /// that one rides over (the lower index), as the legacy.
    fn sinks(&self) -> bool {
        !self.continental && (self.other_continental || self.plate > self.other)
    }

    /// What the boundary is, from the crust on each side and whether the
    /// plates close (`convergence` positive) or part.
    fn kind(&self, convergence: f64) -> Boundary {
        match (self.continental, self.other_continental, convergence > 0.0) {
            (true, true, true) => Boundary::Collision,
            (false, false, true) => Boundary::IslandArc,
            (_, _, true) => Boundary::Andean,
            (true, true, false) => Boundary::Rift,
            (false, false, false) => Boundary::Ridge,
            _ => Boundary::Transform,
        }
    }
}

/// The kinds of boundary that build relief, and among them the
/// subductions, whose two sides get their own distances.
const KINDS: [Boundary; 5] = [
    Boundary::Collision,
    Boundary::Andean,
    Boundary::IslandArc,
    Boundary::Rift,
    Boundary::Ridge,
];
const SUBDUCTIONS: [Boundary; 2] = [Boundary::Andean, Boundary::IslandArc];

/// What the plate boundaries do to the surface, per cell.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateRelief {
    /// Height added (negative: lowered), metres.
    pub height: Vec<f32>,
    /// Weight of a volcanic arc, in `[0, 1]`: where arc cones stand.
    pub arc: Vec<f32>,
    /// Weight of mountain ridges, in `[0, 1]`: collision ranges and
    /// Andean plateaus.
    pub orogeny: Vec<f32>,
}

impl PlateMap {
    /// The plates of a body drawn from its pass seed `seed` on `grid`,
    /// `continental` telling the crust of each cell (it sets which plates
    /// sink, and so their speeds). Draws from `seed`'s children only.
    ///
    /// # Panics
    ///
    /// Panics when `continental` does not hold one flag per cell.
    pub fn draw(grid: &Grid, seed: Seed, continental: &[bool]) -> PlateMap {
        assert_eq!(continental.len(), grid.cell_count(), "one flag per cell");
        let draw = |label: &str| unit(seed.derive(label).get());
        let span = f64::from(PLATE_COUNT.1 - PLATE_COUNT.0 + 1);
        let count = (PLATE_COUNT.0 + (draw("count") * span) as u32).min(PLATE_COUNT.1) as usize;
        let mut centres = Vec::with_capacity(count);
        let mut weights = Vec::with_capacity(count);
        let mut poles = Vec::with_capacity(count);
        let mut speeds = Vec::with_capacity(count);
        let (slow, fast) = PLATE_RMS_SPEED;
        for i in 0..count {
            let at = |what: &str| draw(&format!("{what}/{i}"));
            centres.push(random_direction(at("centre/u"), at("centre/v")));
            weights.push(PLATE_WEIGHT.0 * at("weight"));
            poles.push(random_direction(at("pole/u"), at("pole/v")));
            speeds.push(slow.0 + (fast.0 - slow.0) * at("speed"));
        }
        let subplates = Subplates::new(seed);
        let plate: Vec<u16> = (0..grid.cell_count())
            .map(|k| {
                let s = subplates.at(grid.point(k));
                let nearest = (0..count)
                    .map(|i| (i, dot(s, centres[i]).clamp(-1.0, 1.0).acos() - weights[i]))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map_or(0, |(i, _)| i);
                u16::try_from(nearest).expect("at most PLATE_COUNT plates")
            })
            .collect();
        let geometry = rms_factors(grid, &plate, &poles);
        let drawn: Vec<[f64; 3]> = (0..count)
            .map(|i| scale(poles[i], speeds[i] / geometry[i]))
            .collect();
        // Slab pull: the share of each plate's edge where it sinks, at the
        // drawn speeds, sets its speed (legacy `pull_by_slabs`).
        let mut edge = vec![0usize; count];
        let mut sinking = vec![0usize; count];
        for e in edges(grid, &plate, continental) {
            let own = usize::from(e.plate);
            edge[own] += 1;
            if e.convergence(&drawn) > RELIEF_SPEED.0.0 && e.sinks() {
                sinking[own] += 1;
            }
        }
        let motion = (0..count)
            .map(|i| {
                let share = if edge[i] > 0 {
                    sinking[i] as f64 / edge[i] as f64
                } else {
                    0.0
                };
                scale(poles[i], slab_pull_speed(share).0 / geometry[i])
            })
            .collect();
        PlateMap {
            plate,
            continental: continental.to_vec(),
            motion,
        }
    }

    /// Number of plates.
    pub fn count(&self) -> usize {
        self.motion.len()
    }

    /// Plate of each cell.
    pub fn plate(&self) -> &[u16] {
        &self.plate
    }

    /// Rotation of each plate, m/s: see [`PlateMap::velocity`].
    pub fn motion(&self) -> &[[f64; 3]] {
        &self.motion
    }

    /// Velocity of plate `plate` at the unit vector `p`, m/s, in the
    /// tangent plane.
    pub fn velocity(&self, plate: usize, p: [f64; 3]) -> [f64; 3] {
        cross(self.motion[plate], p)
    }

    /// The relief the plate boundaries build on a body of radius `radius`
    /// and surface gravity `gravity`.
    ///
    /// Each boundary cell is classified by its convergence (from the two
    /// plates' rotations) and the crust on each side
    /// ([`science::relief::plates::Boundary`]), and given its strength
    /// ([`relief_strength`]) rounded to one of [`RELIEF_LEVELS`] levels.
    /// The relief of a kind is the layer cake
    /// `Σ_ℓ profile(d_ℓ, v_ℓ) − profile(d_ℓ, v_{ℓ−1})`, `d_ℓ` the distance
    /// to its boundary cells at least `ℓ / L` strong and `v_ℓ` the
    /// convergence of that strength ([`relief_level`]): the discretised
    /// `∫₀¹ profile(d_{s≥t}) dt`, continuous in every distance, so the
    /// relief never jumps where the nearest boundary cell changes. A
    /// subduction's distance is positive on its overriding side: for an
    /// Andean margin the continental cells (plateau and arc), for an island
    /// arc the plates that mostly override (whole plates, so the side never
    /// switches inside one); the trench is on both. The relief therefore
    /// steps only across plate boundaries and coastlines. Heights of
    /// all kinds add up; the arc and orogeny weights take the strongest
    /// kind. Heights are [`boundary_height`] scaled by [`relief_scale`],
    /// except the ridges' (thermal isostasy, the same at any gravity).
    pub fn relief(
        &self,
        grid: &Grid,
        radius: Metre,
        gravity: MetrePerSecondSquared,
    ) -> PlateRelief {
        self.relief_of(grid, radius, gravity, &KINDS)
    }

    /// [`PlateMap::relief`] of the boundaries of `kinds` only.
    pub(crate) fn relief_of(
        &self,
        grid: &Grid,
        radius: Metre,
        gravity: MetrePerSecondSquared,
        kinds: &[Boundary],
    ) -> PlateRelief {
        let n = grid.cell_count();
        let levels = RELIEF_LEVELS;
        let edges: Vec<(Edge, Boundary, u32)> = edges(grid, &self.plate, &self.continental)
            .into_iter()
            .map(|e| {
                let v = MetrePerSecond(e.convergence(&self.motion));
                let kind = e.kind(v.0);
                let level = (relief_strength(kind, v) * f64::from(levels)).round() as u32;
                (e, kind, level)
            })
            .collect();
        // Convergence of strength level `l` of `kind`; level 0 is no
        // motion, where every profile is flat.
        let speed = |kind: Boundary, l: u32| {
            if l == 0 {
                MetrePerSecond(0.0)
            } else {
                relief_level(kind, f64::from(l) / f64::from(levels))
            }
        };
        // Distance to the cells of `kind` on `side` at least `l` strong,
        // within the reach of the profile at that strength.
        let distance = |kind: Boundary, side: Option<bool>, l: u32| {
            let mut sources = vec![false; n];
            for (e, k, level) in &edges {
                if *k == kind && *level >= l && side.is_none_or(|over| over != e.sinks()) {
                    sources[e.cell] = true;
                }
            }
            let reach = relief_reach(kind, speed(kind, l));
            sources
                .contains(&true)
                .then(|| distance_to(grid, radius, &sources, Some(reach)))
        };
        let scale = relief_scale(gravity);
        let mut out = PlateRelief {
            height: vec![0.0; n],
            arc: vec![0.0; n],
            orogeny: vec![0.0; n],
        };
        for &kind in kinds {
            let gain = if kind == Boundary::Ridge { 1.0 } else { scale };
            let mut arc = vec![0.0f64; n];
            let mut orogeny = vec![0.0f64; n];
            for l in 1..=levels {
                let (lo, hi) = (speed(kind, l - 1), speed(kind, l));
                let fields = if SUBDUCTIONS.contains(&kind) {
                    [
                        distance(kind, Some(true), l),
                        distance(kind, Some(false), l),
                    ]
                } else {
                    [distance(kind, None, l), None]
                };
                if fields.iter().all(Option::is_none) {
                    continue;
                }
                // The side of each cell. An Andean margin is ocean sinking
                // under a continent: its overriding side is continental by
                // definition (`Boundary::Andean`), so continental cells take
                // it, with its plateau and arc, and oceanic cells the
                // sinking side, with the trench. An island arc stands on
                // the overriding plate: a whole plate takes one side, the
                // one most of its edge cells of this level are on, so the
                // side never switches inside a plate; plates with no such
                // edge take the sinking side (the trench's symmetric
                // flank, no arc).
                let mut role = vec![(0usize, 0usize); self.count()];
                for (e, k, level) in &edges {
                    if *k == kind && *level >= l {
                        let r = &mut role[usize::from(e.plate)];
                        if e.sinks() {
                            r.1 += 1;
                        } else {
                            r.0 += 1;
                        }
                    }
                }
                for k in 0..n {
                    let side = match kind {
                        Boundary::Andean => self.continental[k],
                        Boundary::IslandArc => {
                            let (over, under) = role[usize::from(self.plate[k])];
                            over > 0 && over >= under
                        }
                        // Symmetric kinds: their one field.
                        _ => true,
                    };
                    let Some(d) = signed(&fields, k, side) else {
                        continue;
                    };
                    let d = Metre(d);
                    let h = boundary_height(kind, d, hi).0 - boundary_height(kind, d, lo).0;
                    out.height[k] += (h * gain) as f32;
                    arc[k] += arc_weight(kind, d, hi).get() - arc_weight(kind, d, lo).get();
                    orogeny[k] +=
                        orogeny_weight(kind, d, hi).get() - orogeny_weight(kind, d, lo).get();
                }
            }
            for k in 0..n {
                out.arc[k] = out.arc[k].max(arc[k] as f32);
                out.orogeny[k] = out.orogeny[k].max(orogeny[k] as f32);
            }
        }
        out
    }
}

/// The distance at cell `k` from the boundary cells of one level, the
/// nearer of the fields' (for a subduction, both its sides'), counted
/// positive on the overriding side (`over`) and negative on the sinking
/// side. The nearer of both, not the side's own, so a cell at a boundary
/// is at distance 0 on either side and the trench is whole whatever the
/// grid's size. `None` beyond the reach of both.
fn signed(fields: &[Option<Vec<f32>>; 2], k: usize, over: bool) -> Option<f64> {
    let d = fields
        .iter()
        .flatten()
        .map(|f| f64::from(f[k]))
        .filter(|d| d.is_finite())
        .reduce(f64::min)?;
    Some(if over { d } else { -d })
}

/// The boundary cells of `grid`: those next to another plate than their
/// own, with the direction across the boundary taken over the cells
/// within [`BOUNDARY_WINDOW`] neighbour steps that lie on the other plate
/// (one neighbour only gives one of 8 directions).
fn edges(grid: &Grid, plate: &[u16], continental: &[bool]) -> Vec<Edge> {
    (0..grid.cell_count())
        .filter_map(|k| {
            let a = plate[k];
            let (across_cell, b) = grid
                .neighbours(k)
                .map(|j| (j, plate[j]))
                .find(|&(_, other)| other != a)?;
            let p = grid.point(k);
            let mut window = vec![k];
            let mut frontier = vec![k];
            for _ in 0..BOUNDARY_WINDOW {
                let mut next = Vec::new();
                for &c in &frontier {
                    for j in grid.neighbours(c) {
                        if !window.contains(&j) {
                            window.push(j);
                            next.push(j);
                        }
                    }
                }
                frontier = next;
            }
            let towards = window
                .iter()
                .filter(|&&j| plate[j] == b)
                .fold([0.0; 3], |t, &j| add(t, sub(grid.point(j), p)));
            let across = normalise(sub(towards, scale(p, dot(towards, p))));
            Some(Edge {
                cell: k,
                point: p,
                across,
                plate: a,
                other: b,
                continental: continental[k],
                other_continental: continental[across_cell],
            })
        })
        .collect()
}

/// Per plate, `√mean(1 − (pole · p)²)` over its cells, weighted by area:
/// the RMS of `|pole × p|`, which turns an RMS surface speed into the
/// rotation's magnitude. 1 for a plate holding no cell.
fn rms_factors(grid: &Grid, plate: &[u16], poles: &[[f64; 3]]) -> Vec<f64> {
    let mut sums = vec![(0.0f64, 0.0f64); poles.len()];
    for (k, &i) in plate.iter().enumerate() {
        let i = usize::from(i);
        let c = dot(poles[i], grid.point(k));
        sums[i].0 += grid.area(k) * (1.0 - c * c);
        sums[i].1 += grid.area(k);
    }
    sums.into_iter()
        .map(|(s, a)| if s > 0.0 { (s / a).sqrt() } else { 1.0 })
        .collect()
}

/// The jittered lattice the subplates are the cells of, and the noise
/// that warps its lookup.
struct Subplates {
    /// Jittered point of each lattice cell, in lattice units, indexed by
    /// [`Subplates::index`].
    points: Vec<[f64; 3]>,
    /// Lattice cells from `-reach` to `reach` on each axis.
    reach: i64,
    warp: [Perlin; 3],
}

impl Subplates {
    /// The lattice and warp of the pass seed `seed`: each lattice point
    /// jittered by `seed.derive("subplates")`'s children, each axis of the
    /// warp a Perlin noise of its own (the legacy offsets one noise per
    /// axis instead).
    fn new(seed: Seed) -> Subplates {
        let lattice = seed.derive("subplates");
        // The warped lookup stays within 1 + 2 × SUBPLATE_WARP of the
        // centre (two octaves of noise in [-1, 1], the second halved),
        // and the search looks one cell beyond.
        let reach = (SUBPLATE_FREQUENCY * (1.0 + 2.0 * SUBPLATE_WARP)).ceil() as i64 + 1;
        let side = 2 * reach + 1;
        let mut points = Vec::with_capacity((side * side * side) as usize);
        for i in -reach..=reach {
            for j in -reach..=reach {
                for k in -reach..=reach {
                    let cell = lattice.derive(&format!("{i}/{j}/{k}"));
                    let jitter = |axis: &str| unit(cell.derive(axis).get());
                    points.push([
                        i as f64 + jitter("x"),
                        j as f64 + jitter("y"),
                        k as f64 + jitter("z"),
                    ]);
                }
            }
        }
        let warp = ["warp/x", "warp/y", "warp/z"].map(|l| Perlin::new(seed.derive(l).get() as u32));
        Subplates {
            points,
            reach,
            warp,
        }
    }

    fn index(&self, c: [i64; 3]) -> usize {
        let side = 2 * self.reach + 1;
        let [i, j, k] = c.map(|v| v.clamp(-self.reach, self.reach) + self.reach);
        ((i * side + j) * side + k) as usize
    }

    /// The jittered lattice point of the subplate holding the unit vector
    /// `p`, on the unit sphere: the nearest lattice point to `p` displaced
    /// by two octaves of noise.
    fn at(&self, p: [f64; 3]) -> [f64; 3] {
        let f = SUBPLATE_WARP_FREQUENCY;
        let offset = |perlin: &Perlin| {
            perlin.get(p.map(|c| c * f)) + 0.5 * perlin.get(p.map(|c| c * 2.0 * f))
        };
        let q = [0, 1, 2].map(|a| p[a] + SUBPLATE_WARP * offset(&self.warp[a]));
        let s = q.map(|c| c * SUBPLATE_FREQUENCY);
        let cell = s.map(|c| c.floor() as i64);
        let mut best = (f64::MAX, [0.0; 3]);
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let point = self.points[self.index([cell[0] + dx, cell[1] + dy, cell[2] + dz])];
                    let d = dot(sub(point, s), sub(point, s));
                    if d < best.0 {
                        best = (d, point);
                    }
                }
            }
        }
        normalise(best.1)
    }
}

/// A uniform draw in `[0, 1)` from the top 53 bits of `h`.
pub(crate) fn unit(h: u64) -> f64 {
    let bits = f64::MANTISSA_DIGITS;
    (h >> (u64::BITS - bits)) as f64 / (1u64 << bits) as f64
}

/// A uniformly random unit vector from two uniform draws.
fn random_direction(u: f64, v: f64) -> [f64; 3] {
    let z = 2.0 * u - 1.0;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = std::f64::consts::TAU * v;
    [r * phi.cos(), r * phi.sin(), z]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    a.map(|c| c * s)
}

fn normalise(a: [f64; 3]) -> [f64; 3] {
    let n = dot(a, a).sqrt();
    if n > 0.0 { scale(a, 1.0 / n) } else { a }
}

#[cfg(test)]
#[path = "plates_tests.rs"]
mod tests;
