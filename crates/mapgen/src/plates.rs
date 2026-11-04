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
//! 100 km per Ma at its equator (2 to 10 cm per year, Earth's range).
//!
//! **Boundaries.** On a coarse grid, a pixel whose neighbour lies on
//! another plate is on a boundary; its convergence is the speed of its
//! plate against the other one across the boundary (positive: they close,
//! negative: they part), in cm per year. Every pixel then gets its distance
//! to the nearest boundary and that boundary's convergence.
//!
//! **Relief.** Each pixel of the coarse grid gets what its nearest boundary
//! does to the relief ([`PlateMap::relief`]): ranges, trenches, volcanic
//! arcs, ridges and rifts, from the boundary's convergence and the crust
//! on each side; the generation adds it to the continents of worlds with
//! plate tectonics (`[geology] regime = "plates"`).
use crate::geo::Grid;
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
/// Width of the coarse grid the boundaries are found on, pixels (about 40
/// km per pixel on an Earth-sized planet).
const COARSE_WIDTH: u32 = 1024;
/// Half-width of the window the direction across a boundary is taken
/// over, coarse pixels.
const BOUNDARY_WINDOW: i64 = 3;
/// Distances to a boundary stop growing here, km.
const MAX_BOUNDARY_KM: f32 = 3000.0;

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
pub struct Plates {
    seed: u32,
    warp: Perlin,
    centres: Vec<[f64; 3]>,
    weights: Vec<f64>,
    /// Rotation vector of each plate: its Euler pole times its angular
    /// speed, radians per Ma.
    omegas: Vec<[f64; 3]>,
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
}

/// The plates and their boundaries on a coarse latitude-longitude grid.
pub struct PlateMap {
    pub grid: Grid,
    /// Plate of each coarse pixel.
    pub plate: Vec<u16>,
    /// Distance to the nearest plate boundary, km (capped).
    pub boundary_km: Vec<f32>,
    /// Convergence of that boundary, cm per year: positive where the plates
    /// close (subduction, collision), negative where they part (ridges,
    /// rifts).
    pub convergence: Vec<f32>,
    /// The boundary pixels, and the nearest one of each pixel.
    boundaries: Vec<Boundary>,
    nearest: Vec<u32>,
    /// What the boundaries do to the relief of each pixel (see
    /// [`PlateMap::relief`]).
    reliefs: Vec<Relief>,
}

/// A boundary pixel: how its two plates move, and their crust on each side.
#[derive(Clone, Copy, Debug)]
struct Boundary {
    /// Convergence, cm per year (see [`PlateMap::convergence`]).
    convergence: f32,
    /// The plate of the pixel, and the one across.
    plate: u16,
    other: u16,
    /// Whether the crust is continental on the pixel's side, and across.
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
    /// Weight of mountain ridges, in `[0, 1]`.
    pub orogeny: f32,
    /// Weight of a volcanic arc, in `[0, 1]`.
    pub arc: f32,
}

/// Convergence from which a volcanic arc is at full strength, cm per year.
const ARC_SPEED: f32 = 2.0;
/// Latitudes over which boundary relief fades out towards the poles,
/// degrees.
const POLAR_FADE_DEG: (f32, f32) = (70.0, 85.0);
/// Convergence over which boundary relief grows from nothing to its full
/// height, cm per year.
const RELIEF_SPEED: (f32, f32) = (0.5, 5.0);

/// A bump of width `sigma` (km) centred at `mu` km from the boundary.
fn bump(d: f32, mu: f32, sigma: f32) -> f32 {
    let t = (d - mu) / sigma;
    (-t * t).exp()
}

fn smooth(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Unit vector of the centre of pixel `i` of `grid`.
fn pixel_point(grid: &Grid, i: usize) -> [f64; 3] {
    let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
    let lon = (f64::from(x) + 0.5) / f64::from(grid.width) * 2.0 * PI;
    let lat = grid.latitude(y);
    [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()]
}

impl PlateMap {
    /// The plates of world `p` and their boundaries, on a grid
    /// [`COARSE_WIDTH`] pixels wide, all crust oceanic.
    pub fn new(p: &WorldParams) -> Self {
        Self::with_crust(p, |_| false)
    }

    /// Like [`Self::new`], with `continental` telling the crust at a unit
    /// vector (evaluated on both sides of each boundary only).
    pub fn with_crust(p: &WorldParams, continental: impl Fn([f64; 3]) -> bool + Sync) -> Self {
        let plates = Plates::new(p);
        // A fixed grid, whatever the map's size: the same world gets the
        // same boundaries at every resolution.
        let grid = Grid {
            width: COARSE_WIDTH,
            height: COARSE_WIDTH / 2,
            circumference_km: p.circumference_km,
        };
        let n = grid.pixel_count();
        let plate: Vec<u16> = (0..n)
            .into_par_iter()
            .map(|i| plates.plate_at(pixel_point(&grid, i)) as u16)
            .collect();
        // Boundary pixels and their convergence: the speed of this plate
        // against the other one across the boundary. The direction across
        // is taken over a window of pixels, from this pixel towards those
        // of the other plate: one neighbour only gives one of 8 directions,
        // which jump from pixel to pixel along a stepped boundary.
        let seeds: Vec<(usize, Boundary)> = (0..n)
            .into_par_iter()
            .filter_map(|i| {
                let a = plate[i];
                let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
                // The other plate: the first neighbour's that is not this one.
                let (across_pixel, b) = grid
                    .neighbours(x, y)
                    .map(|(nx, ny, _)| grid.index(nx, ny))
                    .map(|j| (j, plate[j]))
                    .find(|&(_, k)| k != a)?;
                let p = pixel_point(&grid, i);
                let mut towards = [0.0f64; 3];
                for oy in -BOUNDARY_WINDOW..=BOUNDARY_WINDOW {
                    let ny = i64::from(y) + oy;
                    if ny < 0 || ny >= i64::from(grid.height) {
                        continue;
                    }
                    for ox in -BOUNDARY_WINDOW..=BOUNDARY_WINDOW {
                        let nx = (i64::from(x) + ox).rem_euclid(i64::from(grid.width));
                        let j = grid.index(nx as u32, ny as u32);
                        if plate[j] == b {
                            let q = pixel_point(&grid, j);
                            towards = [
                                towards[0] + q[0] - p[0],
                                towards[1] + q[1] - p[1],
                                towards[2] + q[2] - p[2],
                            ];
                        }
                    }
                }
                // In the tangent plane.
                let t = dot(towards, p);
                let across = normalise([
                    towards[0] - t * p[0],
                    towards[1] - t * p[1],
                    towards[2] - t * p[2],
                ]);
                let (va, vb) = (
                    plates.velocity(usize::from(a), p),
                    plates.velocity(usize::from(b), p),
                );
                let closing = dot([va[0] - vb[0], va[1] - vb[1], va[2] - vb[2]], across);
                Some((
                    i,
                    Boundary {
                        // km/Ma to cm/yr.
                        convergence: (closing * 0.1) as f32,
                        plate: a,
                        other: b,
                        continental: continental(p),
                        other_continental: continental(pixel_point(&grid, across_pixel)),
                    },
                ))
            })
            .collect();
        let (boundary_km, nearest) = nearest_boundary(&grid, &seeds);
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
        };
        map.reliefs = (0..n).into_par_iter().map(|k| map.relief_of(k)).collect();
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
    /// pixels: which side of a boundary a point is on switches the relief
    /// from one pixel to the next, which would draw steps.
    pub fn relief(&self, lon: f64, lat: f64) -> Relief {
        let g = &self.grid;
        let (w, h) = (i64::from(g.width), i64::from(g.height));
        let x = lon.rem_euclid(2.0 * PI) / (2.0 * PI) * w as f64 - 0.5;
        let y = ((PI / 2.0 - lat) / PI * h as f64 - 0.5).clamp(0.0, (h - 1) as f64);
        let (x0, y0) = (x.floor() as i64, y.floor() as i64);
        let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
        let at =
            |xx: i64, yy: i64| self.reliefs[(yy.clamp(0, h - 1) * w + xx.rem_euclid(w)) as usize];
        let mix = |a: Relief, b: Relief, t: f32| Relief {
            uplift: a.uplift + (b.uplift - a.uplift) * t,
            orogeny: a.orogeny + (b.orogeny - a.orogeny) * t,
            arc: a.arc + (b.arc - a.arc) * t,
        };
        let top = mix(at(x0, y0), at(x0 + 1, y0), fx);
        let bottom = mix(at(x0, y0 + 1), at(x0 + 1, y0 + 1), fx);
        mix(top, bottom, fy)
    }

    /// The relief of coarse pixel `k` (see [`Self::relief`]).
    fn relief_of(&self, k: usize) -> Relief {
        let Some(b) = self.boundaries.get(self.nearest[k] as usize) else {
            return Relief::default();
        };
        let d = self.boundary_km[k];
        let own = self.plate[k];
        // The latitude-longitude grid stretches boundaries near the poles
        // into streaks: relief fades out there, as erosion does.
        let lat = self
            .grid
            .latitude(k as u32 / self.grid.width)
            .abs()
            .to_degrees() as f32;
        let polar = 1.0 - smooth(POLAR_FADE_DEG.0, POLAR_FADE_DEG.1, lat);
        if polar <= 0.0 {
            return Relief::default();
        }
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
                (false, false) => r.uplift = 0.1 * parting * bump(d, 0.0, 450.0),
                (true, true) => {
                    r.uplift = parting * (0.02 * bump(d, 120.0, 60.0) - 0.06 * bump(d, 0.0, 60.0));
                }
                _ => {}
            }
        }
        Relief {
            uplift: r.uplift * polar,
            orogeny: r.orogeny * polar,
            arc: r.arc * polar,
        }
    }

    /// The coarse pixel holding the unit vector `p`.
    pub fn index_at(&self, lon: f64, lat: f64) -> usize {
        let g = &self.grid;
        let x =
            ((lon.rem_euclid(2.0 * PI) / (2.0 * PI) * f64::from(g.width)) as u32).min(g.width - 1);
        let y = (((PI / 2.0 - lat) / PI * f64::from(g.height)) as u32).min(g.height - 1);
        g.index(x, y)
    }
}

/// Distance to the nearest of `seeds` (boundary pixels) and that seed's
/// index in `seeds` (`u32::MAX` beyond [`MAX_BOUNDARY_KM`]), for every pixel
/// (multi-source Dijkstra over [`Grid::neighbours`]).
fn nearest_boundary(grid: &Grid, seeds: &[(usize, Boundary)]) -> (Vec<f32>, Vec<u32>) {
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
    let n = grid.pixel_count();
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
        let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
        for (nx, ny, step) in grid.neighbours(x, y) {
            let j = grid.index(nx, ny);
            let nd = d + step;
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
    use crate::preset::Preset;

    fn params(seed: u32) -> WorldParams {
        WorldParams {
            width: 512,
            height: 256,
            seed,
            planet: Preset::load("terran").unwrap(),
            sea_level: 0.2,
            volcanic_intensity: 0.5,
            circumference_km: 40_075.0,
            axial_tilt_deg: 23.4,
            erosion: 1.0,
            pressure_bar: 1.0,
            surface_age_ga: 0.4,
            star: crate::star::SUN,
            orbit_au: 1.0,
        }
    }

    #[test]
    fn plates_cover_the_sphere_with_varied_sizes() {
        let map = PlateMap::new(&params(3));
        let plates = Plates::new(&params(3));
        assert!((PLATES.0..=PLATES.0 + PLATES.1).contains(&plates.count()));
        let mut area = vec![0.0f64; plates.count()];
        for (i, &k) in map.plate.iter().enumerate() {
            area[usize::from(k)] += map.grid.latitude(i as u32 / map.grid.width).cos();
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
    fn boundaries_raise_ranges_and_arcs_and_sink_trenches() {
        // Continents in one hemisphere (x > 0), ocean in the other.
        let map = PlateMap::with_crust(&params(5), |q| q[0] > 0.0);
        let g = &map.grid;
        let (mut range, mut trench, mut ridge, mut arc) = (false, false, false, false);
        for k in 0..g.pixel_count() {
            let r = map.reliefs[k];
            assert!(
                r.uplift.abs() <= 0.2
                    && (0.0..=1.0).contains(&r.orogeny)
                    && (0.0..=1.0).contains(&r.arc)
            );
            range |= r.orogeny > 0.3 && r.uplift > 0.03;
            trench |= r.uplift < -0.05;
            ridge |= r.uplift > 0.03 && r.orogeny == 0.0 && r.arc < 0.1;
            arc |= r.arc > 0.5;
        }
        assert!(
            range && trench && ridge && arc,
            "{range} {trench} {ridge} {arc}"
        );
        // Nothing near the poles, where the grid stretches.
        for x in 0..g.width {
            assert_eq!(map.reliefs[g.index(x, 0)], Relief::default());
        }
        // Interpolated between pixels: no relief beyond them.
        let r = map.relief(1.0, 0.3);
        assert!(r.uplift.is_finite());
    }

    #[test]
    fn boundaries_close_on_one_side_and_part_on_the_other() {
        let map = PlateMap::new(&params(5));
        let g = &map.grid;
        // Boundary pixels are at distance 0, the others farther.
        let boundary = map.boundary_km.iter().filter(|&&d| d == 0.0).count();
        assert!(boundary > 0 && boundary < g.pixel_count() / 5, "{boundary}");
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
}
