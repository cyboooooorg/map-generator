//! A nearly uniform grid on the sphere: the six faces of a cube, each cut
//! into `n × n` cells and blown up onto the sphere.
//!
//! Cells are spaced by equal angles across each face (an equi-angular cube
//! map), so their size varies by about 30 % over the sphere, against the
//! latitude-longitude grid whose cells shrink to nothing at the poles.
//! Cells across a face edge are found through the sphere: a cell index past
//! the edge of a face is a point on the plane of that face extended, which
//! falls in a cell of the next face.

use science::units::Metre;

/// Each face: its normal and the two axes of its plane (orthonormal).
const FACES: [[[f64; 3]; 3]; 6] = [
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
    [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
    [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
    [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
    [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
];

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Definition: the default cell size of the simulation grid on solid
/// worlds (architecture rewrite design, Simulation grid).
pub const DEFAULT_CELL: Metre = Metre(15_000.0);

/// Definition: the cap on the cell count (architecture rewrite design,
/// Simulation grid).
pub const MAX_CELLS: usize = 8_000_000;

/// A cube-sphere grid of `6 × n²` cells. Cell `k` is at column `x`, row
/// `y` of face `f`, with `k = (f × n + y) × n + x`.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    n: u32,
    /// Solid angle of each of the `n²` cells of one face, in steradians;
    /// the same on every face.
    face_areas: Vec<f64>,
}

impl Grid {
    /// A grid of `n × n` cells per face.
    ///
    /// # Panics
    ///
    /// Panics when `n` is zero.
    pub fn new(n: u32) -> Grid {
        assert!(n > 0, "a grid needs at least one cell per face side");
        // Gnomonic coordinates of the cell edges, equal steps of angle.
        let edge = |i: u32| {
            ((f64::from(i) / f64::from(n) * 2.0 - 1.0) * std::f64::consts::FRAC_PI_4).tan()
        };
        let edges: Vec<f64> = (0..=n).map(edge).collect();
        // Solid angle of [0, s] × [0, t] on the unit face plane.
        let omega = |s: f64, t: f64| (s * t / (1.0 + s * s + t * t).sqrt()).atan();
        let mut face_areas = Vec::with_capacity((n as usize).pow(2));
        for y in 0..n as usize {
            for x in 0..n as usize {
                let (s0, s1, t0, t1) = (edges[x], edges[x + 1], edges[y], edges[y + 1]);
                face_areas.push(omega(s1, t1) - omega(s0, t1) - omega(s1, t0) + omega(s0, t0));
            }
        }
        Grid { n, face_areas }
    }

    /// The grid of a body of radius `radius` with cells of about `cell`:
    /// `n = max(1, round(π/2 × radius / cell))` cells per face side, held
    /// so that the cell count stays within [`MAX_CELLS`].
    pub fn for_radius(radius: Metre, cell: Metre) -> Grid {
        let wanted = (std::f64::consts::FRAC_PI_2 * radius.0 / cell.0).round();
        let cap = (MAX_CELLS as f64 / 6.0).sqrt().floor();
        Grid::new(wanted.clamp(1.0, cap) as u32)
    }

    /// Cells per face side.
    pub fn n(&self) -> u32 {
        self.n
    }

    /// The solid angle of cell `k`, in steradians.
    pub fn area(&self, k: usize) -> f64 {
        self.face_areas[k % self.face_areas.len()]
    }

    pub fn cell_count(&self) -> usize {
        6 * (self.n as usize).pow(2)
    }

    /// Face, column and row of cell `k`.
    fn split(&self, k: usize) -> (usize, i64, i64) {
        let n = self.n as usize;
        (k / (n * n), (k % n) as i64, (k / n % n) as i64)
    }

    /// The unit vector at column `x`, row `y` of face `face`, in cells (the
    /// centre of a cell at whole values); past the face's edges, on its
    /// plane extended.
    pub fn point_at(&self, face: usize, x: f64, y: f64) -> [f64; 3] {
        let n = f64::from(self.n);
        let plane = |c: f64| ((c + 0.5) / n * 2.0 - 1.0) * std::f64::consts::FRAC_PI_4;
        let (s, t) = (plane(x).tan(), plane(y).tan());
        let [normal, e1, e2] = FACES[face];
        let v = [0, 1, 2].map(|a| normal[a] + s * e1[a] + t * e2[a]);
        let l = dot(v, v).sqrt();
        v.map(|c| c / l)
    }

    /// The centre of cell `k`.
    pub fn point(&self, k: usize) -> [f64; 3] {
        let (f, x, y) = self.split(k);
        self.point_at(f, x as f64, y as f64)
    }

    /// Face, column and row (in cells, whole at cell centres) of the unit
    /// vector `p`.
    pub fn locate(&self, p: [f64; 3]) -> (usize, f64, f64) {
        let face = (0..6)
            .max_by(|&a, &b| dot(p, FACES[a][0]).total_cmp(&dot(p, FACES[b][0])))
            .unwrap_or(0);
        let [normal, e1, e2] = FACES[face];
        let d = dot(p, normal);
        let n = f64::from(self.n);
        let cell = |c: f64| ((c / d).atan() / std::f64::consts::FRAC_PI_4 + 1.0) / 2.0 * n - 0.5;
        (face, cell(dot(p, e1)), cell(dot(p, e2)))
    }

    /// The cell holding the unit vector `p`.
    pub fn index_of(&self, p: [f64; 3]) -> usize {
        let (face, x, y) = self.locate(p);
        let last = f64::from(self.n - 1);
        let n = self.n as usize;
        let (x, y) = (x.round().clamp(0.0, last), y.round().clamp(0.0, last));
        (face * n + y as usize) * n + x as usize
    }

    /// The cell at column `x`, row `y` of face `face`, which may lie past
    /// the face's edges (then on the next face).
    pub fn cell(&self, face: usize, x: i64, y: i64) -> usize {
        let n = i64::from(self.n);
        if (0..n).contains(&x) && (0..n).contains(&y) {
            ((face as i64 * n + y) * n + x) as usize
        } else {
            self.index_of(self.point_at(face, x as f64, y as f64))
        }
    }

    /// The cell `dx` columns and `dy` rows from cell `k` (across a face
    /// edge, as seen from `k`'s face).
    pub fn offset(&self, k: usize, dx: i64, dy: i64) -> usize {
        let (f, x, y) = self.split(k);
        self.cell(f, x + dx, y + dy)
    }

    /// The eight cells around cell `k`. At the eight corners of the cube,
    /// where three faces meet, one of them repeats another.
    pub fn neighbours(&self, k: usize) -> impl Iterator<Item = usize> + '_ {
        const AROUND: [(i64, i64); 8] = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        AROUND
            .into_iter()
            .map(move |(dx, dy)| self.offset(k, dx, dy))
    }
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
