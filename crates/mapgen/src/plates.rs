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
//! Nothing here changes a world yet: the generation only exports the plates
//! and their boundaries as diagnostic maps. Relief along the boundaries
//! (ranges, trenches, arcs, ridges, rifts) is the next step.
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
/// Width of the coarse grid the boundaries are found on, pixels at most.
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
}

/// Unit vector of the centre of pixel `i` of `grid`.
fn pixel_point(grid: &Grid, i: usize) -> [f64; 3] {
    let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
    let lon = (f64::from(x) + 0.5) / f64::from(grid.width) * 2.0 * PI;
    let lat = grid.latitude(y);
    [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()]
}

impl PlateMap {
    /// The plates of world `p` and their boundaries, on a grid at most
    /// [`COARSE_WIDTH`] pixels wide.
    pub fn new(p: &WorldParams) -> Self {
        let plates = Plates::new(p);
        let width = p.width.clamp(16, COARSE_WIDTH);
        let grid = Grid {
            width,
            height: (width / 2).max(8),
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
        let seeds: Vec<(usize, f32)> = (0..n)
            .into_par_iter()
            .filter_map(|i| {
                let a = plate[i];
                let (x, y) = (i as u32 % grid.width, i as u32 / grid.width);
                // The other plate: the first neighbour's that is not this one.
                let b = grid
                    .neighbours(x, y)
                    .map(|(nx, ny, _)| plate[grid.index(nx, ny)])
                    .find(|&k| k != a)?;
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
                let (a, b) = (usize::from(a), usize::from(b));
                let (va, vb) = (plates.velocity(a, p), plates.velocity(b, p));
                let closing = dot([va[0] - vb[0], va[1] - vb[1], va[2] - vb[2]], across);
                // km/Ma to cm/yr.
                Some((i, (closing * 0.1) as f32))
            })
            .collect();
        let (boundary_km, convergence) = nearest_boundary(&grid, &seeds);
        PlateMap {
            grid,
            plate,
            boundary_km,
            convergence,
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

/// Distance to the nearest of `seeds` (boundary pixels with their
/// convergence) and that seed's convergence, for every pixel (multi-source
/// Dijkstra over [`Grid::neighbours`]).
fn nearest_boundary(grid: &Grid, seeds: &[(usize, f32)]) -> (Vec<f32>, Vec<f32>) {
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
    let mut convergence = vec![0.0f32; n];
    let mut heap = BinaryHeap::new();
    for &(i, c) in seeds {
        distance[i] = 0.0;
        convergence[i] = c;
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
                convergence[j] = convergence[i];
                heap.push(Node(nd, j));
            }
        }
    }
    for d in &mut distance {
        *d = d.min(MAX_BOUNDARY_KM);
    }
    (distance, convergence)
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
