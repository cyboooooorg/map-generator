//! Tectonic plates, static: where the plates of a world lie, how they move
//! and where they meet (backlog item 9, step 1; `docs/analysis/08-geology.md`,
//! section 8).
//!
//! **Plates from subplates.** The sphere is cut into about 300 jagged
//! "subplates": the cells of a jittered 3D lattice around the unit sphere,
//! looked up at a position displaced by two octaves of noise, so their
//! borders wander. Each subplate joins the nearest of 10 to 15 plate
//! centres, each with a weight of its own (a power diagram), so plates have
//! varied sizes and follow the jagged borders of their subplates
//! (frozenfractal's *Around The World* builds its plates the same way).
//!
//! **Motion.** Each plate turns about an Euler pole of its own, at 20 to
//! 100 km per Ma at its equator (2 to 10 cm per year, Earth's range):
//! faster the longer the edge along which it sinks under its neighbours
//! (slab pull, see [`pull_by_slabs`]).
//!
//! **Boundaries.** On a coarse cube-sphere grid ([`CubeGrid`], nearly
//! uniform, so the poles are like anywhere else), a cell whose neighbour
//! lies on another plate is on a boundary; its convergence is the speed of
//! its plate against the other one across the boundary (positive: they
//! close, negative: they part), in cm per year. Every cell then gets its distance
//! to the nearest boundary and that boundary's convergence.
//!
//! **Relief.** Each cell of the coarse grid gets what its nearest boundary
//! does to the relief ([`PlateMap::relief`]): ranges, trenches, volcanic
//! arcs, ridges and rifts, from the boundary's convergence and the crust
//! on each side; the generation adds it to the continents of worlds with
//! plate tectonics (`[geology] regime = "plates"`).
use crate::cube::CubeGrid;
use crate::noise::{stream, stream_seed};
use crate::world::WorldParams;
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::f64::consts::PI;

/// Lattice frequency of the subplates on the unit sphere: about 300 cells.
const SUBPLATE_FREQUENCY: f64 = 5.0;
/// Displacement of the subplate lookup, on the unit sphere, and the
/// frequency of its first octave (the second is twice as fine, half as
/// strong).
const SUBPLATE_WARP: f64 = 0.08;
const SUBPLATE_WARP_FREQUENCY: f64 = 3.0;
/// Number of plates: 10 plus up to this many more.
const PLATES: (usize, usize) = (10, 5);
/// Largest weight of a plate centre, radians: a plate of weight `w` reaches
/// `w` farther than an unweighted one.
const PLATE_WEIGHT: f64 = 0.35;
/// Plate speeds at their equator, km per Ma (1 km/Ma = 0.1 cm/yr).
const PLATE_SPEED: (f64, f64) = (20.0, 100.0);
/// Cells along the edge of each face of the coarse grid the boundaries are
/// found on (about 40 km per cell on an Earth-sized planet).
const COARSE_SIZE: u32 = 256;
/// Half-width of the window the direction across a boundary is taken
/// over, coarse cells.
const BOUNDARY_WINDOW: i64 = 3;
/// Distances to a boundary stop growing here, km.
const MAX_BOUNDARY_KM: f32 = 3000.0;
/// Length of a stage of plate motion, Ma: each plate gets a new Euler pole
/// and speed every stage, its rotation vector moving linearly from one to
/// the next.
const STAGE_MA: f64 = 50.0;
/// Farthest epoch from the present, Ma, in the past or the future: beyond,
/// the present relief carried by the plates says little of the world.
pub const DRIFT_LIMIT_MA: f64 = 300.0;
/// Step of the integration of the plates' rotations, Ma.
const DRIFT_STEP_MA: f64 = 1.0;

/// SplitMix64 finaliser.
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A uniform draw in `[0, 1)` from a hash.
fn unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// A uniformly random unit vector from two draws.
fn random_direction(u: f64, v: f64) -> [f64; 3] {
    let z = 2.0 * u - 1.0;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = 2.0 * PI * v;
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

fn normalise(a: [f64; 3]) -> [f64; 3] {
    let n = dot(a, a).sqrt().max(1e-12);
    [a[0] / n, a[1] / n, a[2] / n]
}

/// The plates of a world: where they are and how they turn.
#[derive(Clone)]
pub struct Plates {
    seed: u32,
    warp: Perlin,
    centres: Vec<[f64; 3]>,
    weights: Vec<f64>,
    /// Rotation vector of each plate: its Euler pole times its angular
    /// speed, radians per Ma.
    omegas: Vec<[f64; 3]>,
    /// Speeds of the drift's other stages set by slab pull (see
    /// [`pull_stages`]), by plate and stage; the drawn ones elsewhere.
    staged: std::collections::BTreeMap<(usize, i64), [f64; 3]>,
    radius_km: f64,
}

impl Plates {
    /// The plates of the world `p`, from its seed.
    pub fn new(p: &WorldParams) -> Self {
        let seed = stream_seed(p.seed, stream::PLATE_MOTION);
        let mut state = mix(u64::from(seed));
        let mut next = || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            unit(mix(state))
        };
        let count = PLATES.0 + (next() * (PLATES.1 + 1) as f64) as usize;
        let radius_km = f64::from(p.circumference_km) / (2.0 * PI);
        let mut centres = Vec::with_capacity(count);
        let mut weights = Vec::with_capacity(count);
        let mut omegas = Vec::with_capacity(count);
        for _ in 0..count {
            centres.push(random_direction(next(), next()));
            weights.push(PLATE_WEIGHT * next());
            let pole = random_direction(next(), next());
            let speed_km = PLATE_SPEED.0 + (PLATE_SPEED.1 - PLATE_SPEED.0) * next();
            let omega = speed_km / radius_km;
            omegas.push(pole.map(|c| c * omega));
        }
        Plates {
            seed,
            warp: Perlin::new(stream_seed(p.seed, stream::SUBPLATES)),
            centres,
            weights,
            omegas,
            staged: std::collections::BTreeMap::new(),
            radius_km,
        }
    }

    /// Number of plates.
    pub fn count(&self) -> usize {
        self.centres.len()
    }

    /// The jittered lattice point of the subplate holding the unit vector
    /// `p`, on the unit sphere.
    fn subplate(&self, p: [f64; 3]) -> [f64; 3] {
        // Jagged borders: the lookup is displaced by two octaves of noise.
        let f = SUBPLATE_WARP_FREQUENCY;
        let offset = |o: f64| {
            self.warp.get([p[0] * f + o, p[1] * f, p[2] * f])
                + 0.5
                    * self
                        .warp
                        .get([p[0] * 2.0 * f, p[1] * 2.0 * f + o, p[2] * 2.0 * f])
        };
        let q = [
            p[0] + SUBPLATE_WARP * offset(0.0),
            p[1] + SUBPLATE_WARP * offset(17.0),
            p[2] + SUBPLATE_WARP * offset(-31.0),
        ];
        let s = q.map(|c| c * SUBPLATE_FREQUENCY);
        let cell = s.map(|c| c.floor() as i64);
        let mut best = (f64::MAX, [0.0; 3]);
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (i, j, k) = (cell[0] + dx, cell[1] + dy, cell[2] + dz);
                    let mut h = u64::from(self.seed);
                    for v in [i, j, k] {
                        h = mix(h ^ (v as u64).wrapping_add(0x9E37_79B9_7F4A_7C15));
                    }
                    let point = [
                        i as f64 + unit(h),
                        j as f64 + unit(mix(h ^ 1)),
                        k as f64 + unit(mix(h ^ 2)),
                    ];
                    let d = (0..3).map(|a| (point[a] - s[a]).powi(2)).sum::<f64>();
                    if d < best.0 {
                        best = (d, point);
                    }
                }
            }
        }
        normalise(best.1)
    }

    /// The plate holding the unit vector `p`: the plate whose weighted
    /// centre is nearest to `p`'s subplate.
    pub fn plate_at(&self, p: [f64; 3]) -> usize {
        let s = self.subplate(p);
        (0..self.count())
            .map(|k| {
                (
                    k,
                    dot(s, self.centres[k]).clamp(-1.0, 1.0).acos() - self.weights[k],
                )
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(0, |(k, _)| k)
    }

    /// Velocity of plate `plate` at the unit vector `p`, km per Ma, in the
    /// tangent plane.
    pub fn velocity(&self, plate: usize, p: [f64; 3]) -> [f64; 3] {
        cross(self.omegas[plate], p).map(|c| c * self.radius_km)
    }

    /// The rotation vector of plate `plate` at the start of stage `stage`
    /// (`stage × STAGE_MA` Ma from the present): the present one at stage
    /// 0, drawn for the others from the plate and the stage alone.
    fn stage_omega(&self, plate: usize, stage: i64) -> [f64; 3] {
        if stage == 0 {
            return self.omegas[plate];
        }
        if let Some(&omega) = self.staged.get(&(plate, stage)) {
            return omega;
        }
        let mut h = mix(u64::from(self.seed) ^ 0x5354_4147_4553);
        for v in [plate as u64, stage as u64] {
            h = mix(h ^ v.wrapping_add(0x9E37_79B9_7F4A_7C15));
        }
        let pole = random_direction(unit(h), unit(mix(h ^ 1)));
        let speed_km = PLATE_SPEED.0 + (PLATE_SPEED.1 - PLATE_SPEED.0) * unit(mix(h ^ 2));
        pole.map(|c| c * speed_km / self.radius_km)
    }

    /// The rotation vector of plate `plate` at `t_ma` (Ma from the present,
    /// negative in the past), interpolated between its stages.
    pub fn omega_at(&self, plate: usize, t_ma: f64) -> [f64; 3] {
        let u = t_ma / STAGE_MA;
        let stage = u.floor();
        let f = u - stage;
        let (a, b) = (
            self.stage_omega(plate, stage as i64),
            self.stage_omega(plate, stage as i64 + 1),
        );
        [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * f)
    }

    /// The plates at `t_ma` Ma from the present (negative in the past,
    /// clamped to [`DRIFT_LIMIT_MA`]): the rotation carrying each plate from
    /// where it lies today to where it lies then, and its motion then.
    pub fn at_epoch(&self, t_ma: f64) -> Epoch {
        let t_ma = t_ma.clamp(-DRIFT_LIMIT_MA, DRIFT_LIMIT_MA);
        let steps = (t_ma.abs() / DRIFT_STEP_MA).ceil().max(1.0) as usize;
        let dt = t_ma / steps as f64;
        let rotations = (0..self.count())
            .map(|k| {
                let mut r = IDENTITY;
                for i in 0..steps {
                    let omega = self.omega_at(k, (i as f64 + 0.5) * dt);
                    r = multiply(rotation(omega.map(|c| c * dt)), r);
                }
                r
            })
            .collect();
        let omegas = (0..self.count()).map(|k| self.omega_at(k, t_ma)).collect();
        Epoch {
            t_ma,
            rotations,
            omegas,
            radius_km: self.radius_km,
        }
    }
}

/// The plates at an epoch (see [`Plates::at_epoch`]).
pub struct Epoch {
    /// Ma from the present.
    pub t_ma: f64,
    /// Per plate: the rotation from its present position to its position
    /// at the epoch.
    rotations: Vec<Matrix>,
    /// Per plate: its rotation vector at the epoch, radians per Ma.
    omegas: Vec<[f64; 3]>,
    radius_km: f64,
}

impl Epoch {
    /// Where the point of plate `plate` at the unit vector `p` (at the
    /// epoch) lies today.
    pub fn origin(&self, plate: usize, p: [f64; 3]) -> [f64; 3] {
        apply_transposed(&self.rotations[plate], p)
    }

    /// Where the point of plate `plate` lying today at `p` lies at the
    /// epoch.
    pub fn position(&self, plate: usize, p: [f64; 3]) -> [f64; 3] {
        apply(&self.rotations[plate], p)
    }

    /// Velocity of plate `plate` at the unit vector `p` at the epoch, km
    /// per Ma.
    pub fn velocity(&self, plate: usize, p: [f64; 3]) -> [f64; 3] {
        cross(self.omegas[plate], p).map(|c| c * self.radius_km)
    }
}

type Matrix = [[f64; 3]; 3];

const IDENTITY: Matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// The rotation by the vector `v`: about its direction, by its length in
/// radians (Rodrigues' formula).
fn rotation(v: [f64; 3]) -> Matrix {
    let angle = dot(v, v).sqrt();
    if angle < 1e-15 {
        return IDENTITY;
    }
    let [x, y, z] = v.map(|c| c / angle);
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
    ]
}

fn multiply(a: Matrix, b: Matrix) -> Matrix {
    let mut m = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

fn apply(m: &Matrix, p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|i| dot(m[i], p))
}

fn apply_transposed(m: &Matrix, p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|i| m[0][i] * p[0] + m[1][i] * p[1] + m[2][i] * p[2])
}

/// The plates and their boundaries on a coarse cube-sphere grid.
pub struct PlateMap {
    pub grid: CubeGrid,
    /// Plate of each coarse cell.
    pub plate: Vec<u16>,
    /// Distance to the nearest plate boundary, km (capped).
    pub boundary_km: Vec<f32>,
    /// Convergence of that boundary, cm per year: positive where the plates
    /// close (subduction, collision), negative where they part (ridges,
    /// rifts).
    pub convergence: Vec<f32>,
    /// The boundary cells, and the nearest one of each cell.
    boundaries: Vec<Boundary>,
    nearest: Vec<u32>,
    /// What the boundaries do to the relief of each cell (see
    /// [`PlateMap::relief`]).
    reliefs: Vec<Relief>,
    plates: Plates,
    /// The plates' motion up to the map's epoch, for a map away from the
    /// present.
    epoch: Option<Epoch>,
}

/// What a world's present plate map owes to its plates alone, not to its
/// crust (and so not to its sea level, which sets the crust): the plates
/// as drawn, the plate of each cell, the boundary cells and the distance
/// to them. Built once, it gives the map for any crust
/// ([`PlateMap::from_frame`]), as the search of a sea level for a land
/// share needs.
pub struct PlateFrame {
    grid: CubeGrid,
    plates: Plates,
    present: Vec<u16>,
    cells: Vec<BoundaryCell>,
    boundary_km: Vec<f32>,
    nearest: Vec<u32>,
}

impl PlateFrame {
    /// The present frame of the world `p`.
    pub fn new(p: &WorldParams) -> Self {
        let plates = Plates::new(p);
        // A fixed grid, whatever the map's size: the same world gets the
        // same boundaries at every resolution.
        let grid = CubeGrid { size: COARSE_SIZE };
        let present: Vec<u16> = (0..grid.cell_count())
            .into_par_iter()
            .map(|k| plates.plate_at(grid.point(k)) as u16)
            .collect();
        let cells = boundary_cells(&grid, &present, |_| false);
        let radius_km = f64::from(p.circumference_km) / std::f64::consts::TAU;
        let positions: Vec<(usize, Boundary)> = cells
            .iter()
            .map(|c| {
                (
                    c.cell,
                    Boundary {
                        convergence: 0.0,
                        plate: c.plate,
                        other: c.other,
                        continental: false,
                        other_continental: false,
                    },
                )
            })
            .collect();
        let (boundary_km, nearest) = nearest_boundary(&grid, radius_km, &positions);
        PlateFrame {
            grid,
            plates,
            present,
            cells,
            boundary_km,
            nearest,
        }
    }
}

/// Where the crust at a point at the map's epoch lies today (see
/// [`PlateMap::origin`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Origin {
    /// The unit vector of its present position.
    pub point: [f64; 3],
    /// Crust formed since (or to be formed by then), where the plates
    /// part: it lies nowhere today.
    pub new_crust: bool,
}

/// A boundary cell: how its two plates move, and their crust on each side.
#[derive(Clone, Copy, Debug)]
struct Boundary {
    /// Convergence, cm per year (see [`PlateMap::convergence`]).
    convergence: f32,
    /// The plate of the cell, and the one across.
    plate: u16,
    other: u16,
    /// Whether the crust is continental on the cell's side, and across.
    continental: bool,
    other_continental: bool,
}

/// What plate boundaries do to the relief at a point (see
/// [`PlateMap::relief`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Relief {
    /// Height added (negative: lowered), elevation units, at Earth's
    /// gravity.
    pub uplift: f32,
    /// Height of young ocean floor above the old floor, elevation units:
    /// the swell of a mid-ocean ridge, from the age of the floor (see
    /// [`ridge_swell`]). Thermal isostasy: the same at any gravity.
    pub swell: f32,
    /// Weight of mountain ridges, in `[0, 1]`.
    pub orogeny: f32,
    /// Weight of a volcanic arc, in `[0, 1]`.
    pub arc: f32,
}

/// Convergence from which a volcanic arc is at full strength, cm per year.
const ARC_SPEED: f32 = 2.0;
/// Convergence over which boundary relief grows from nothing to its full
/// height, cm per year.
const RELIEF_SPEED: (f32, f32) = (0.5, 5.0);

/// Depth of the ocean floor at a ridge crest, km, and its sinking with
/// the square root of its age, km per √Ma: `d = 2.5 + 0.35 √t` up to
/// about 70 Ma, as the floor cools and contracts (Parsons & Sclater 1977,
/// doi:10.1029/JB082i005p00803).
const RIDGE_CREST_KM: f32 = 2.5;
const SUBSIDENCE_KM: f32 = 0.35;
/// Depth of the generator's old sea floor, km: its median, about 0.16
/// elevation units (see `craters::ELEVATION_KM`). Young floor stands above
/// it until the age at which Parsons & Sclater's floor reaches this depth
/// (18 Ma); older floor stays at the noise's depth, which knows no age.
pub const OLD_FLOOR_KM: f32 = 4.0;

/// The swell of the ocean floor at `d_km` from a ridge spreading at
/// `spreading` cm per year (both plates together), elevation units: the
/// floor there is `d / (v/2)` old, and stands above [`OLD_FLOOR_KM`] by
/// what Parsons & Sclater's age–depth relation gives. 1.5 km at the crest;
/// at 5 cm/yr it fades out 460 km from the ridge, at 2 cm/yr 180 km.
pub fn ridge_swell(d_km: f32, spreading: f32) -> f32 {
    // cm/yr to km/Ma, one plate's half.
    let half_km_per_ma = 5.0 * spreading.max(1e-3);
    let age_ma = d_km.max(0.0) / half_km_per_ma;
    let depth_km = RIDGE_CREST_KM + SUBSIDENCE_KM * age_ma.sqrt();
    (OLD_FLOOR_KM - depth_km).max(0.0) / crate::craters::ELEVATION_KM
}

/// A bump of width `sigma` (km) centred at `mu` km from the boundary.
fn bump(d: f32, mu: f32, sigma: f32) -> f32 {
    let t = (d - mu) / sigma;
    (-t * t).exp()
}

fn smooth(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl PlateMap {
    /// The plates of world `p` and their boundaries, on a grid of
    /// [`COARSE_SIZE`] cells per face edge, all crust oceanic.
    pub fn new(p: &WorldParams) -> Self {
        Self::with_crust(p, |_| false)
    }

    /// Like [`Self::new`], with `continental` telling the crust at a unit
    /// vector (evaluated on both sides of each boundary only).
    pub fn with_crust(p: &WorldParams, continental: impl Fn([f64; 3]) -> bool + Sync) -> Self {
        Self::at_epoch(p, 0.0, continental)
    }

    /// Like [`Self::with_crust`], with the plates as they lie `t_ma` Ma
    /// from the present (negative in the past, within
    /// [`DRIFT_LIMIT_MA`]): each turned by its stage rotations. Where
    /// several plates come to cover a cell, continental crust overrides
    /// oceanic crust (which dives under it), then the lower plate index
    /// wins; where none does, the plates have parted and new oceanic crust
    /// fills the gap, joining the nearest plate.
    pub fn at_epoch(
        p: &WorldParams,
        t_ma: f64,
        continental: impl Fn([f64; 3]) -> bool + Sync,
    ) -> Self {
        if t_ma == 0.0 {
            return Self::from_frame(&PlateFrame::new(p), continental);
        }
        let mut plates = Plates::new(p);
        // A fixed grid, whatever the map's size: the same world gets the
        // same boundaries at every resolution.
        let grid = CubeGrid { size: COARSE_SIZE };
        let radius_km = f64::from(p.circumference_km) / std::f64::consts::TAU;
        let n = grid.cell_count();
        let present: Vec<u16> = (0..n)
            .into_par_iter()
            .map(|k| plates.plate_at(grid.point(k)) as u16)
            .collect();
        // Slab pull: the present speed of each plate from its subducting
        // edge, found on the present boundaries.
        let present_cells = boundary_cells(&grid, &present, |k| continental(grid.point(k)));
        pull_by_slabs(&mut plates, &present_cells);
        pull_stages(&mut plates, &present_cells, t_ma);
        let epoch = (t_ma != 0.0).then(|| plates.at_epoch(t_ma));
        // The plate and crust of each cell at the epoch (the crust of the
        // present map is only sampled on its boundaries, below).
        let (plate, crust) = match &epoch {
            None => (present, None),
            Some(epoch) => {
                let (plate, crust) = epoch_cells(&grid, &plates, epoch, &present, &continental);
                (plate, Some(crust))
            }
        };
        let velocity = |plate: u16, q: [f64; 3]| match &epoch {
            Some(epoch) => epoch.velocity(usize::from(plate), q),
            None => plates.velocity(usize::from(plate), q),
        };
        let cells = match &crust {
            // At the present, the boundaries already found.
            None => present_cells,
            Some(crust) => boundary_cells(&grid, &plate, |k| crust[k]),
        };
        let seeds: Vec<(usize, Boundary)> = cells
            .into_iter()
            .map(|c| {
                let (va, vb) = (velocity(c.plate, c.point), velocity(c.other, c.point));
                let closing = dot([va[0] - vb[0], va[1] - vb[1], va[2] - vb[2]], c.across);
                (
                    c.cell,
                    Boundary {
                        // km/Ma to cm/yr.
                        convergence: (closing * 0.1) as f32,
                        plate: c.plate,
                        other: c.other,
                        continental: c.continental,
                        other_continental: c.other_continental,
                    },
                )
            })
            .collect();
        let (boundary_km, nearest) = nearest_boundary(&grid, radius_km, &seeds);
        let boundaries: Vec<Boundary> = seeds.into_iter().map(|(_, b)| b).collect();
        let convergence = nearest
            .iter()
            .map(|&k| boundaries.get(k as usize).map_or(0.0, |b| b.convergence))
            .collect();
        let mut map = PlateMap {
            grid,
            plate,
            boundary_km,
            convergence,
            boundaries,
            nearest,
            reliefs: Vec::new(),
            plates,
            epoch,
        };
        map.reliefs = (0..n).into_par_iter().map(|k| map.relief_of(k)).collect();
        map.soften_reliefs();
        map
    }

    /// The present plate map of a world from its `frame`, with
    /// `continental` telling the crust on each side of its boundaries:
    /// the same map as [`Self::with_crust`], the crust's part only
    /// computed (see [`PlateFrame`]).
    pub fn from_frame(frame: &PlateFrame, continental: impl Fn([f64; 3]) -> bool + Sync) -> Self {
        let grid = frame.grid;
        let mut plates = frame.plates.clone();
        let cells: Vec<BoundaryCell> = frame
            .cells
            .par_iter()
            .map(|c| BoundaryCell {
                continental: continental(grid.point(c.cell)),
                other_continental: continental(grid.point(c.across_cell)),
                ..c.clone()
            })
            .collect();
        // Slab pull: the present speed of each plate from its subducting
        // edge.
        pull_by_slabs(&mut plates, &cells);
        let seeds: Vec<Boundary> = cells
            .into_iter()
            .map(|c| {
                let (va, vb) = (
                    plates.velocity(usize::from(c.plate), c.point),
                    plates.velocity(usize::from(c.other), c.point),
                );
                let closing = dot([va[0] - vb[0], va[1] - vb[1], va[2] - vb[2]], c.across);
                Boundary {
                    // km/Ma to cm/yr.
                    convergence: (closing * 0.1) as f32,
                    plate: c.plate,
                    other: c.other,
                    continental: c.continental,
                    other_continental: c.other_continental,
                }
            })
            .collect();
        let convergence = frame
            .nearest
            .iter()
            .map(|&k| seeds.get(k as usize).map_or(0.0, |b| b.convergence))
            .collect();
        let mut map = PlateMap {
            grid,
            plate: frame.present.clone(),
            boundary_km: frame.boundary_km.clone(),
            convergence,
            boundaries: seeds,
            nearest: frame.nearest.clone(),
            reliefs: Vec::new(),
            plates,
            epoch: None,
        };
        map.reliefs = (0..grid.cell_count())
            .into_par_iter()
            .map(|k| map.relief_of(k))
            .collect();
        map.soften_reliefs();
        map
    }

    /// What the plate boundaries do to the relief at a longitude and
    /// latitude, growing with the boundary's speed (full from 5 cm/yr):
    ///
    /// | Boundary                        | Relief                                              |
    /// | ------------------------------- | --------------------------------------------------- |
    /// | Continent–continent, closing    | A range on both sides (plateau and ridges)          |
    /// | Continent over ocean, closing   | Coastal range and volcanic arc, 150–220 km inland   |
    /// | Ocean under continent or ocean  | A trench along the boundary                         |
    /// | Ocean over ocean, closing       | An island arc, 120 km from the boundary             |
    /// | Ocean–ocean, parting            | A mid-ocean ridge, broad                            |
    /// | Continent–continent, parting    | A rift valley and its shoulders                     |
    ///
    /// Between two oceanic plates, the one with the lower index overrides
    /// the other. Computed on the coarse grid and interpolated between its
    /// cells: which side of a boundary a point is on switches the relief
    /// from one cell to the next, which would draw steps.
    pub fn relief(&self, p: [f64; 3]) -> Relief {
        let (face, x, y) = self.grid.locate(p);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = ((x - x0) as f32, (y - y0) as f32);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let at = |dx: i64, dy: i64| self.reliefs[self.grid.cell(face, x0 + dx, y0 + dy)];
        let mix = |a: Relief, b: Relief, t: f32| Relief {
            uplift: a.uplift + (b.uplift - a.uplift) * t,
            swell: a.swell + (b.swell - a.swell) * t,
            orogeny: a.orogeny + (b.orogeny - a.orogeny) * t,
            arc: a.arc + (b.arc - a.arc) * t,
        };
        let top = mix(at(0, 0), at(1, 0), fx);
        let bottom = mix(at(0, 1), at(1, 1), fx);
        mix(top, bottom, fy)
    }

    /// Blurs the relief of the coarse cells with their eight neighbours
    /// (weights 1-2-1 across, 1-2-1 down). A cell takes its relief from its
    /// nearest boundary: where the nearest one changes (halfway between two
    /// boundaries, or two stretches of one that close at different speeds
    /// or border different crust) the relief jumps by up to several km
    /// from one cell to the next, a straight cliff a pixel or two wide
    /// along the line of equal distance.
    fn soften_reliefs(&mut self) {
        let sum = |a: Relief, b: Relief, w: f32| Relief {
            uplift: a.uplift + w * b.uplift,
            swell: a.swell + w * b.swell,
            orogeny: a.orogeny + w * b.orogeny,
            arc: a.arc + w * b.arc,
        };
        // The ridges' swell stays as it is: it is the same on both sides of
        // a boundary (no cliff), and a crest keeps its full height.
        let softened: Vec<Relief> = (0..self.reliefs.len())
            .into_par_iter()
            .map(|k| {
                let mut r = sum(Relief::default(), self.reliefs[k], 4.0);
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        if (dx, dy) != (0, 0) {
                            let w = if dx == 0 || dy == 0 { 2.0 } else { 1.0 };
                            r = sum(r, self.reliefs[self.grid.offset(k, dx, dy)], w);
                        }
                    }
                }
                Relief {
                    swell: self.reliefs[k].swell,
                    ..sum(Relief::default(), r, 1.0 / 16.0)
                }
            })
            .collect();
        self.reliefs = softened;
    }

    /// The relief of coarse cell `k` (see [`Self::relief`]).
    fn relief_of(&self, k: usize) -> Relief {
        let Some(b) = self.boundaries.get(self.nearest[k] as usize) else {
            return Relief::default();
        };
        let d = self.boundary_km[k];
        let own = self.plate[k];
        let (mine, theirs, other) = if own == b.plate {
            (b.continental, b.other_continental, b.other)
        } else {
            (b.other_continental, b.continental, b.plate)
        };
        let closing = smooth(RELIEF_SPEED.0, RELIEF_SPEED.1, b.convergence);
        // Arcs need subduction, hardly its speed: full from 2 cm/yr.
        let arcing = smooth(RELIEF_SPEED.0, ARC_SPEED, b.convergence);
        let parting = smooth(RELIEF_SPEED.0, RELIEF_SPEED.1, -b.convergence);
        let trench = || -0.16 * closing * bump(d, 40.0, 45.0);
        let mut r = Relief::default();
        // A mid-ocean ridge: young floor, the younger the higher, its crest
        // at Parsons & Sclater's depth at any spreading rate (slow ridges
        // are narrow by their age alone); it fades in only as the plates
        // start to part, under 0.5 cm/yr.
        if !mine && !theirs && b.convergence < 0.0 {
            r.swell = smooth(0.0, RELIEF_SPEED.0, -b.convergence) * ridge_swell(d, -b.convergence);
        }
        if closing > 0.0 {
            match (mine, theirs) {
                (true, true) => {
                    r.orogeny = closing * bump(d, 0.0, 350.0);
                    r.uplift = 0.12 * r.orogeny;
                }
                (true, false) => {
                    r.orogeny = closing * bump(d, 150.0, 150.0);
                    r.uplift = 0.08 * closing * bump(d, 150.0, 120.0);
                    r.arc = arcing * bump(d, 220.0, 80.0);
                }
                (false, true) => r.uplift = trench(),
                (false, false) if own < other => {
                    r.uplift = 0.12 * closing * bump(d, 120.0, 60.0);
                    r.arc = arcing * bump(d, 120.0, 60.0);
                }
                (false, false) => r.uplift = trench(),
            }
        } else if parting > 0.0 {
            match (mine, theirs) {
                (false, false) => {}
                (true, true) => {
                    r.uplift = parting * (0.02 * bump(d, 120.0, 60.0) - 0.06 * bump(d, 0.0, 60.0));
                }
                _ => {}
            }
        }
        r
    }

    /// The coarse cell holding the unit vector `p`.
    pub fn index_at(&self, p: [f64; 3]) -> usize {
        self.grid.index_of(p)
    }

    /// Where the crust at the unit vector `p`, at the map's epoch, lies
    /// today: `p` itself at the present. The plate is found to the point,
    /// among those of the four cells around `p` (`continental` tells the
    /// crust where two of them overlap), so coasts carried by the plates
    /// stay sharp.
    pub fn origin(&self, p: [f64; 3], continental: impl Fn([f64; 3]) -> bool) -> Origin {
        let Some(epoch) = &self.epoch else {
            return Origin {
                point: p,
                new_crust: false,
            };
        };
        let (face, x, y) = self.grid.locate(p);
        let (x0, y0) = (x.floor() as i64, y.floor() as i64);
        let mut candidates: Vec<u16> = Vec::with_capacity(4);
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let k = self.plate[self.grid.cell(face, x0 + dx, y0 + dy)];
            if !candidates.contains(&k) {
                candidates.push(k);
            }
        }
        candidates.sort_unstable();
        let claims: Vec<(u16, [f64; 3])> = candidates
            .into_iter()
            .map(|k| (k, epoch.origin(usize::from(k), p)))
            .filter(|&(k, o)| self.plates.plate_at(o) == usize::from(k))
            .collect();
        let point = match claims.as_slice() {
            [] => {
                let k = self.plate[self.grid.index_of(p)];
                return Origin {
                    point: epoch.origin(usize::from(k), p),
                    new_crust: true,
                };
            }
            [(_, o)] => *o,
            // Continental crust first, then the lower index.
            [(_, first), ..] => claims
                .iter()
                .find(|(_, o)| continental(*o))
                .map_or(*first, |(_, o)| *o),
        };
        Origin {
            point,
            new_crust: false,
        }
    }
}

/// A boundary cell of a plate map, before its plates' motion is known.
#[derive(Clone)]
struct BoundaryCell {
    cell: usize,
    /// The neighbour across the boundary.
    across_cell: usize,
    point: [f64; 3],
    /// The direction across the boundary, towards the other plate, in the
    /// tangent plane.
    across: [f64; 3],
    plate: u16,
    other: u16,
    continental: bool,
    other_continental: bool,
}

/// The cells of `grid` next to another plate than their own (`plate`),
/// with the direction across the boundary and the crust on each side
/// (`crust_of`). The direction is taken over a window of cells, from the
/// cell towards those of the other plate: one neighbour only gives one of
/// 8 directions, which jump from cell to cell along a stepped boundary.
fn boundary_cells(
    grid: &CubeGrid,
    plate: &[u16],
    crust_of: impl Fn(usize) -> bool + Sync,
) -> Vec<BoundaryCell> {
    (0..grid.cell_count())
        .into_par_iter()
        .filter_map(|k| {
            let a = plate[k];
            // The other plate: the first neighbour's that is not this one.
            let (across_cell, b) = grid
                .neighbours(k)
                .map(|j| (j, plate[j]))
                .find(|&(_, other)| other != a)?;
            let p = grid.point(k);
            let mut towards = [0.0f64; 3];
            for oy in -BOUNDARY_WINDOW..=BOUNDARY_WINDOW {
                for ox in -BOUNDARY_WINDOW..=BOUNDARY_WINDOW {
                    let j = grid.offset(k, ox, oy);
                    if plate[j] == b {
                        let q = grid.point(j);
                        towards = [
                            towards[0] + q[0] - p[0],
                            towards[1] + q[1] - p[1],
                            towards[2] + q[2] - p[2],
                        ];
                    }
                }
            }
            let t = dot(towards, p);
            let across = normalise([
                towards[0] - t * p[0],
                towards[1] - t * p[1],
                towards[2] - t * p[2],
            ]);
            Some(BoundaryCell {
                cell: k,
                across_cell,
                point: p,
                across,
                plate: a,
                other: b,
                continental: crust_of(k),
                other_continental: crust_of(across_cell),
            })
        })
        .collect()
}

/// Share of its boundary a plate must sink along (subduct) to reach the
/// top of [`PLATE_SPEED`]. Calibration, no source: Forsyth & Uyeda
/// (1975, doi:10.1111/j.1365-246X.1975.tb00631.x) find the plates
/// attached to downgoing slabs the fast ones, "the forces acting on the
/// downgoing slab" an order of magnitude stronger than any other.
const SLAB_FULL_SHARE: f64 = 0.3;

/// Slab pull: sets the speed of every plate from the share of its
/// boundary where it sinks under its neighbour (oceanic crust under
/// continental crust, or under the other oceanic plate where that one
/// rides over: the lower index), keeping its Euler pole. A plate with no
/// sinking edge turns at the bottom of [`PLATE_SPEED`], one sinking along
/// [`SLAB_FULL_SHARE`] of its boundary or more at the top. This is the
/// present stage; [`pull_stages`] does the others.
fn pull_by_slabs(plates: &mut Plates, cells: &[BoundaryCell]) {
    let speeds = sinking_speeds(plates.count(), cells, |c| {
        (
            plates.velocity(usize::from(c.plate), c.point),
            plates.velocity(usize::from(c.other), c.point),
            c.across,
        )
    });
    for (k, speed_km) in speeds.into_iter().enumerate() {
        plates.omegas[k] = at_speed(plates.omegas[k], speed_km, plates.radius_km);
    }
}

/// Slab pull for the drift's other stages, out to `t_ma` from the present
/// (stages [`STAGE_MA`] apart, at most six each way). Calibration of the
/// geometry, no source: the present boundary cells are carried rigidly
/// by each plate's rotation to the stage (gaps and overlaps ignored, the
/// crust labels kept), the closing speed is taken from that stage's drawn
/// Euler poles, and each plate's speed is then set from its sinking share
/// as [`pull_by_slabs`] does. Stages go outward from the present, each
/// reading the rotations built from the stages before it. One pass over
/// the boundary cells per stage (a few thousand), not over the grid.
fn pull_stages(plates: &mut Plates, cells: &[BoundaryCell], t_ma: f64) {
    let last = ((t_ma.abs() / STAGE_MA).ceil() as i64).min(6);
    let sign = if t_ma < 0.0 { -1 } else { 1 };
    for i in 1..=last {
        let stage = sign * i;
        let epoch = plates.at_epoch(stage as f64 * STAGE_MA);
        let speeds = sinking_speeds(plates.count(), cells, |c| {
            let (a, b) = (usize::from(c.plate), usize::from(c.other));
            let q = epoch.position(a, c.point);
            (
                epoch.velocity(a, q),
                epoch.velocity(b, q),
                epoch.position(a, c.across),
            )
        });
        for (k, speed_km) in speeds.into_iter().enumerate() {
            let omega = plates.stage_omega(k, stage);
            plates
                .staged
                .insert((k, stage), at_speed(omega, speed_km, plates.radius_km));
        }
    }
}

/// `omega` turned to a speed of `speed_km` at the equator, same pole.
fn at_speed(omega: [f64; 3], speed_km: f64, radius_km: f64) -> [f64; 3] {
    let len = dot(omega, omega).sqrt();
    if len > 0.0 {
        omega.map(|c| c * speed_km / radius_km / len)
    } else {
        omega
    }
}

/// The speed slab pull gives each plate, km per Ma, from the share of its
/// boundary cells where it sinks; `motion` gives, for a cell, the
/// velocities of its plate and the other at it and the direction across.
fn sinking_speeds(
    count: usize,
    cells: &[BoundaryCell],
    motion: impl Fn(&BoundaryCell) -> ([f64; 3], [f64; 3], [f64; 3]),
) -> Vec<f64> {
    let mut edge = vec![0usize; count];
    let mut sinking = vec![0usize; count];
    for c in cells {
        let own = usize::from(c.plate);
        let (va, vb, across) = motion(c);
        let closing = dot([va[0] - vb[0], va[1] - vb[1], va[2] - vb[2]], across) * 0.1;
        edge[own] += 1;
        let under = !c.continental && (c.other_continental || c.plate > c.other);
        if closing > f64::from(RELIEF_SPEED.0) && under {
            sinking[own] += 1;
        }
    }
    (0..count)
        .map(|k| {
            let share = if edge[k] > 0 {
                sinking[k] as f64 / edge[k] as f64
            } else {
                0.0
            };
            PLATE_SPEED.0 + (PLATE_SPEED.1 - PLATE_SPEED.0) * (share / SLAB_FULL_SHARE).min(1.0)
        })
        .collect()
}

/// The plate and the crust (continental or not) of each cell of `grid` at
/// `epoch` (see [`PlateMap::at_epoch`]), from the plates of the present
/// map `present`.
fn epoch_cells(
    grid: &CubeGrid,
    plates: &Plates,
    epoch: &Epoch,
    present: &[u16],
    continental: &(impl Fn([f64; 3]) -> bool + Sync),
) -> (Vec<u16>, Vec<bool>) {
    let n = grid.cell_count();
    // The plates covering each cell, by priority; none in the gaps.
    let claims: Vec<Option<(u16, bool)>> = (0..n)
        .into_par_iter()
        .map(|c| {
            let p = grid.point(c);
            let mut best: Option<(u16, bool)> = None;
            for k in 0..plates.count() {
                let o = epoch.origin(k, p);
                if usize::from(present[grid.index_of(o)]) != k {
                    continue;
                }
                let crust = continental(o);
                if best.is_none_or(|(_, b)| crust && !b) {
                    best = Some((k as u16, crust));
                }
            }
            best
        })
        .collect();
    // New crust in the gaps: each cell joins the nearest covered cell's
    // plate (breadth first), as oceanic crust.
    let mut plate = vec![u16::MAX; n];
    let mut crust = vec![false; n];
    let mut queue = std::collections::VecDeque::new();
    for (c, claim) in claims.iter().enumerate() {
        if let Some((k, continental)) = claim {
            plate[c] = *k;
            crust[c] = *continental;
            queue.push_back(c);
        }
    }
    while let Some(c) = queue.pop_front() {
        for j in grid.neighbours(c) {
            if plate[j] == u16::MAX {
                plate[j] = plate[c];
                queue.push_back(j);
            }
        }
    }
    (plate, crust)
}

/// Distance to the nearest of `seeds` (boundary cells), km on a sphere of
/// radius `radius_km`, and that seed's index in `seeds` (`u32::MAX` beyond
/// [`MAX_BOUNDARY_KM`]), for every cell (multi-source Dijkstra over
/// [`CubeGrid::neighbours`]).
fn nearest_boundary(
    grid: &CubeGrid,
    radius_km: f64,
    seeds: &[(usize, Boundary)],
) -> (Vec<f32>, Vec<u32>) {
    #[derive(PartialEq)]
    struct Node(f32, usize);
    impl Eq for Node {}
    impl Ord for Node {
        fn cmp(&self, other: &Self) -> Ordering {
            other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    let n = grid.cell_count();
    let mut distance = vec![f32::INFINITY; n];
    let mut nearest = vec![u32::MAX; n];
    let mut heap = BinaryHeap::new();
    for (k, &(i, _)) in seeds.iter().enumerate() {
        distance[i] = 0.0;
        nearest[i] = k as u32;
        heap.push(Node(0.0, i));
    }
    while let Some(Node(d, i)) = heap.pop() {
        if d > distance[i] || d >= MAX_BOUNDARY_KM {
            continue;
        }
        let p = grid.point(i);
        for j in grid.neighbours(i) {
            let q = grid.point(j);
            let chord =
                ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
            let nd = d + (2.0 * (chord / 2.0).asin() * radius_km) as f32;
            if nd < distance[j] {
                distance[j] = nd;
                nearest[j] = nearest[i];
                heap.push(Node(nd, j));
            }
        }
    }
    for d in &mut distance {
        *d = d.min(MAX_BOUNDARY_KM);
    }
    (distance, nearest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::Look;

    fn params(seed: u32) -> WorldParams {
        WorldParams {
            age_gyr: crate::star::DEFAULT_AGE_GYR,
            width: 512,
            height: 256,
            seed,
            class: "rocky".into(),
            archetype: None,
            model: None,
            look: Look::load("terran"),
            sea_level: 0.2,
            volcanic_intensity: 0.5,
            circumference_km: 40_075.0,
            axial_tilt_deg: 23.4,
            erosion: 1.0,
            pressure_bar: 1.0,
            surface_age_ga: 0.4,
            star: crate::star::SUN,
            orbit_au: 1.0,
            eccentricity: 0.0,
            drift_ma: 0.0,
            moons: crate::moons::MoonsMode::Auto,
            shape: None,
            metallicity: 0.0,
            giants: crate::giants::GiantsMode::None,
            giant_mass_earth: None,
            host: crate::exomoon::HostMode::None,
            given_rings: None,
            impact_factor: None,
            locked_to_star: None,
            ridge: None,
            perihelion_longitude_deg: None,
            resonance_to_star: None,
            gravity: None,
        }
    }

    #[test]
    fn plates_cover_the_sphere_with_varied_sizes() {
        let map = PlateMap::new(&params(3));
        let plates = Plates::new(&params(3));
        assert!((PLATES.0..=PLATES.0 + PLATES.1).contains(&plates.count()));
        let mut area = vec![0.0f64; plates.count()];
        // Cells are nearly uniform: count them.
        for &k in &map.plate {
            area[usize::from(k)] += 1.0;
        }
        let total: f64 = area.iter().sum();
        let shares: Vec<f64> = area.iter().map(|a| a / total).collect();
        // Most plates hold some ground, and they are not all alike.
        assert!(
            shares.iter().filter(|&&s| s > 0.005).count() >= plates.count() - 2,
            "{shares:?}"
        );
        let (small, large) = shares
            .iter()
            .fold((1.0f64, 0.0f64), |(a, b), &s| (a.min(s), b.max(s)));
        assert!(large > 3.0 * small.max(0.005), "{shares:?}");
        // Deterministic per seed.
        assert_eq!(map.plate, PlateMap::new(&params(3)).plate);
        assert_ne!(map.plate, PlateMap::new(&params(4)).plate);
    }

    #[test]
    fn ridges_follow_the_age_depth_relation_of_the_ocean_floor() {
        // Parsons & Sclater: 2.5 km deep at the crest, 4.3 km at 25 Ma.
        let km = |d, v| OLD_FLOOR_KM - ridge_swell(d, v) * crate::craters::ELEVATION_KM;
        assert!((km(0.0, 5.0) - 2.5).abs() < 1e-4);
        // 25 Ma old at 5 cm/yr: 625 km out, below the generator's floor.
        assert_eq!(ridge_swell(625.0, 5.0), 0.0);
        // 9 Ma: 3.55 km deep.
        assert!((km(225.0, 5.0) - 3.55).abs() < 1e-3);
        // Faster ridges are broader.
        assert!(ridge_swell(200.0, 8.0) > ridge_swell(200.0, 2.0));
        // A slow ridge, as the Mid-Atlantic's 2.5 cm/yr (USGS), keeps its
        // full crest: 1.5 km over the old floor.
        let map = PlateMap::with_crust(&params(5), |_| false);
        let slow_crests: Vec<f32> = (0..map.grid.cell_count())
            .filter(|&k| {
                map.boundaries
                    .get(map.nearest[k] as usize)
                    .is_some_and(|b| {
                        map.boundary_km[k] < 1.0 && (-3.0..-1.0).contains(&b.convergence)
                    })
            })
            .map(|k| map.reliefs[k].swell * crate::craters::ELEVATION_KM)
            .collect();
        assert!(!slow_crests.is_empty());
        assert!(
            slow_crests.iter().all(|&h| (h - 1.5).abs() < 1e-3),
            "{slow_crests:?}"
        );
    }

    #[test]
    fn boundary_relief_does_not_step_from_cell_to_cell() {
        // Continents in one hemisphere, ocean in the other: boundaries of
        // every kind. The mountains' weight and the uplift (0.12 is 3 km)
        // change by a few tenths at most from one coarse cell to the next,
        // not by their full height along the line where the nearest
        // boundary changes.
        for seed in [3, 5, 7] {
            let map = PlateMap::with_crust(&params(seed), |q| q[0] > 0.0);
            let (mut orogeny, mut uplift) = (0.0f32, 0.0f32);
            for k in 0..map.grid.cell_count() {
                for (dx, dy) in [(1, 0), (0, 1)] {
                    let (a, b) = (map.reliefs[k], map.reliefs[map.grid.offset(k, dx, dy)]);
                    orogeny = orogeny.max((a.orogeny - b.orogeny).abs());
                    uplift = uplift.max((a.uplift - b.uplift).abs());
                }
            }
            // Before the blur: a full step of 1.0 and 0.28 (7 km).
            assert!(
                orogeny < 0.6 && uplift < 0.15,
                "seed {seed}: {orogeny} {uplift}"
            );
        }
    }

    #[test]
    fn plates_sinking_along_their_edges_are_the_fast_ones() {
        // Continents in one hemisphere (x > 0), ocean in the other.
        let p = params(5);
        let continental = |q: [f64; 3]| q[0] > 0.0;
        let mut plates = Plates::new(&p);
        let grid = CubeGrid { size: COARSE_SIZE };
        let present: Vec<u16> = (0..grid.cell_count())
            .map(|k| plates.plate_at(grid.point(k)) as u16)
            .collect();
        let cells = boundary_cells(&grid, &present, |k| continental(grid.point(k)));
        let drawn = plates.omegas.clone();
        pull_by_slabs(&mut plates, &cells);
        let speed = |o: [f64; 3]| dot(o, o).sqrt() * plates.radius_km;
        let speeds: Vec<f64> = plates.omegas.iter().map(|&o| speed(o)).collect();
        for (k, &v) in speeds.iter().enumerate() {
            assert!(
                (PLATE_SPEED.0 - 1e-6..=PLATE_SPEED.1 + 1e-6).contains(&v),
                "{v}"
            );
            // The pole is kept.
            let (a, b) = (normalise(drawn[k]), normalise(plates.omegas[k]));
            assert!(dot(a, b) > 1.0 - 1e-9);
        }
        // Some sink and are fast, continental ones without a slab are slow.
        assert!(speeds.iter().any(|&v| v > 60.0), "{speeds:?}");
        assert!(speeds.iter().any(|&v| v < 30.0), "{speeds:?}");
        // The map uses them.
        let map = PlateMap::with_crust(&p, continental);
        assert_eq!(map.plates.omegas, plates.omegas);
    }

    #[test]
    fn other_stages_are_pulled_by_slabs_too() {
        let p = params(5);
        let continental = |q: [f64; 3]| q[0] > 0.0;
        let mut plates = Plates::new(&p);
        let grid = CubeGrid { size: COARSE_SIZE };
        let present: Vec<u16> = (0..grid.cell_count())
            .map(|k| plates.plate_at(grid.point(k)) as u16)
            .collect();
        let cells = boundary_cells(&grid, &present, |k| continental(grid.point(k)));
        pull_by_slabs(&mut plates, &cells);
        let drawn = plates.clone();
        pull_stages(&mut plates, &cells, -120.0);
        let speed = |o: [f64; 3]| dot(o, o).sqrt() * plates.radius_km;
        let mut changed = 0;
        for k in 0..plates.count() {
            for stage in [-1, -2, -3] {
                let (new, old) = (plates.stage_omega(k, stage), drawn.stage_omega(k, stage));
                assert!(
                    (PLATE_SPEED.0 - 1e-6..=PLATE_SPEED.1 + 1e-6).contains(&speed(new)),
                    "{}",
                    speed(new)
                );
                // The pole is kept.
                assert!(dot(normalise(new), normalise(old)) > 1.0 - 1e-9);
                changed += usize::from((speed(new) - speed(old)).abs() > 1.0);
            }
            // Stages beyond the epoch's keep their drawn speeds.
            assert_eq!(plates.stage_omega(k, -4), drawn.stage_omega(k, -4));
        }
        assert!(changed > 0);
    }

    #[test]
    fn boundaries_raise_ranges_and_arcs_and_sink_trenches() {
        // Continents in one hemisphere (x > 0), ocean in the other.
        let map = PlateMap::with_crust(&params(5), |q| q[0] > 0.0);
        let g = &map.grid;
        let (mut range, mut trench, mut ridge, mut arc) = (false, false, false, false);
        let mut polar = false;
        for k in 0..g.cell_count() {
            let r = map.reliefs[k];
            assert!(
                r.uplift.abs() <= 0.2
                    && (0.0..=1.0).contains(&r.orogeny)
                    && (0.0..=1.0).contains(&r.arc)
            );
            range |= r.orogeny > 0.3 && r.uplift > 0.03;
            trench |= r.uplift < -0.05;
            ridge |= r.swell > 0.03 && r.orogeny == 0.0 && r.arc < 0.1;
            arc |= r.arc > 0.5;
            polar |= g.point(k)[2].abs() > 0.98 && r != Relief::default();
        }
        assert!(
            range && trench && ridge && arc,
            "{range} {trench} {ridge} {arc}"
        );
        // The grid is uniform: boundaries shape the poles too.
        assert!(polar);
        // Interpolated between cells, across face edges too.
        for p in [[1.0, 0.0, 0.0], [0.6, 0.0, 0.8], [0.577, 0.577, 0.577]] {
            assert!(map.relief(normalise(p)).uplift.is_finite());
        }
    }

    #[test]
    fn boundaries_close_on_one_side_and_part_on_the_other() {
        let map = PlateMap::new(&params(5));
        let g = &map.grid;
        // Boundary cells are at distance 0, the others farther.
        let boundary = map.boundary_km.iter().filter(|&&d| d == 0.0).count();
        assert!(boundary > 0 && boundary < g.cell_count() / 5, "{boundary}");
        // Converging and diverging boundaries both exist, at Earth's speeds.
        let (closing, parting) = map
            .convergence
            .iter()
            .zip(&map.boundary_km)
            .filter(|(_, d)| **d == 0.0)
            .fold((0, 0), |(c, p), (v, _)| {
                (c + usize::from(*v > 0.5), p + usize::from(*v < -0.5))
            });
        assert!(closing > 0 && parting > 0, "{closing} {parting}");
        assert!(map.convergence.iter().all(|c| c.abs() <= 25.0));
        // Across a boundary the two sides see the same speed, opposite.
        let plates = Plates::new(&params(5));
        let p = random_direction(0.3, 0.7);
        let q = random_direction(0.31, 0.7);
        let (a, b) = (0, 1);
        let rel = |x: usize, y: usize| {
            let (vx, vy) = (plates.velocity(x, p), plates.velocity(y, p));
            dot(
                [vx[0] - vy[0], vx[1] - vy[1], vx[2] - vy[2]],
                normalise(cross(p, q)),
            )
        };
        assert!((rel(a, b) + rel(b, a)).abs() < 1e-9);
    }

    #[test]
    fn epochs_turn_the_plates_rigidly() {
        let plates = Plates::new(&params(5));
        let present = plates.at_epoch(0.0);
        let p = normalise([0.3, -0.5, 0.8]);
        for k in 0..plates.count() {
            assert_eq!(present.position(k, p), p);
        }
        let mut moved = 0.0;
        for t in [-100.0, 100.0] {
            let epoch = plates.at_epoch(t);
            for k in 0..plates.count() {
                let q = epoch.position(k, p);
                // Rotations: unit vectors stay unit, and come back.
                assert!((dot(q, q) - 1.0).abs() < 1e-9);
                let back = epoch.origin(k, q);
                assert!((0..3).all(|a| (back[a] - p[a]).abs() < 1e-9));
                moved += dot(p, q).clamp(-1.0, 1.0).acos() * plates.radius_km;
            }
        }
        // 20 to 100 km per Ma, over 100 Ma: thousands of km on average.
        let mean = moved / (2 * plates.count()) as f64;
        assert!((500.0..12_000.0).contains(&mean), "{mean}");
    }

    #[test]
    fn a_map_at_an_epoch_covers_the_sphere_and_opens_new_crust() {
        let p = params(5);
        let present = PlateMap::with_crust(&p, |q| q[0] > 0.0);
        assert_eq!(
            present.plate,
            PlateMap::at_epoch(&p, 0.0, |q| q[0] > 0.0).plate
        );
        let map = PlateMap::at_epoch(&p, -150.0, |q| q[0] > 0.0);
        assert!(
            map.plate
                .iter()
                .all(|&k| usize::from(k) < map.plates.count())
        );
        assert_ne!(map.plate, present.plate);
        let boundary = map.boundary_km.iter().filter(|&&d| d == 0.0).count();
        assert!(
            boundary > 0 && boundary < map.grid.cell_count() / 5,
            "{boundary}"
        );
        // Most points come from somewhere on their plate; some, where the
        // plates have parted, from nowhere (a quarter at 150 Ma: Earth renews
        // its ocean floor in about 180 Ma).
        let (mut old, mut new) = (0, 0);
        for k in 0..2000 {
            let q = random_direction((k as f64 * 0.618_034) % 1.0, (k as f64 * 0.414_213) % 1.0);
            let origin = map.origin(q, |o| o[0] > 0.0);
            if origin.new_crust {
                new += 1;
            } else {
                old += 1;
                assert!((dot(origin.point, origin.point) - 1.0).abs() < 1e-9);
            }
        }
        assert!(new > 0 && old > 2 * new, "{old} {new}");
        // At the present, every point is its own origin.
        let q = normalise([0.2, 0.9, -0.1]);
        assert_eq!(present.origin(q, |_| false).point, q);
    }
}
