//! A nearly uniform grid on the sphere: the six faces of a cube, each cut
//! into `size × size` cells and blown up onto the sphere.
//!
//! Cells are spaced by equal angles across each face (an equi-angular cube
//! map), so their size varies by about 30 % over the sphere, against the
//! latitude-longitude grid whose cells shrink to nothing at the poles.
//! Cells across a face edge are found through the sphere: a cell index past
//! the edge of a face is a point on the plane of that face extended, which
//! falls in a cell of the next face.

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

/// A cube-sphere grid of `6 × size²` cells. Cell `k` is at column `x`, row
/// `y` of face `f`, with `k = (f × size + y) × size + x`.
#[derive(Clone, Copy, Debug)]
pub struct CubeGrid {
    pub size: u32,
}

impl CubeGrid {
    pub fn cell_count(&self) -> usize {
        6 * (self.size as usize).pow(2)
    }

    /// Face, column and row of cell `k`.
    fn split(&self, k: usize) -> (usize, i64, i64) {
        let n = self.size as usize;
        (k / (n * n), (k % n) as i64, (k / n % n) as i64)
    }

    /// The unit vector at column `x`, row `y` of face `face`, in cells (the
    /// centre of a cell at whole values); past the face's edges, on its
    /// plane extended.
    pub fn point_at(&self, face: usize, x: f64, y: f64) -> [f64; 3] {
        let n = f64::from(self.size);
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
        let n = f64::from(self.size);
        let cell = |c: f64| ((c / d).atan() / std::f64::consts::FRAC_PI_4 + 1.0) / 2.0 * n - 0.5;
        (face, cell(dot(p, e1)), cell(dot(p, e2)))
    }

    /// The cell holding the unit vector `p`.
    pub fn index_of(&self, p: [f64; 3]) -> usize {
        let (face, x, y) = self.locate(p);
        let last = f64::from(self.size - 1);
        let n = self.size as usize;
        let (x, y) = (x.round().clamp(0.0, last), y.round().clamp(0.0, last));
        (face * n + y as usize) * n + x as usize
    }

    /// The cell at column `x`, row `y` of face `face`, which may lie past
    /// the face's edges (then on the next face).
    pub fn cell(&self, face: usize, x: i64, y: i64) -> usize {
        let n = i64::from(self.size);
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
mod tests {
    use super::*;

    #[test]
    fn cells_cover_the_sphere_evenly() {
        let g = CubeGrid { size: 32 };
        let angle = |a: [f64; 3], b: [f64; 3]| dot(a, b).clamp(-1.0, 1.0).acos();
        let (mut short, mut long) = (f64::MAX, 0.0f64);
        for k in 0..g.cell_count() {
            let p = g.point(k);
            // Each centre lies in its own cell.
            assert_eq!(g.index_of(p), k);
            for j in g.neighbours(k) {
                assert_ne!(j, k);
                let step = angle(p, g.point(j));
                short = short.min(step);
                long = long.max(step);
            }
        }
        // Neighbours (diagonal ones included, √2 farther) are about a cell
        // apart everywhere, face edges and corners included.
        let cell = std::f64::consts::FRAC_PI_2 / 32.0;
        assert!(
            short > 0.6 * cell && long < 1.9 * cell,
            "{short} {long} {cell}"
        );
    }

    #[test]
    fn neighbours_across_an_edge_are_mutual() {
        let g = CubeGrid { size: 16 };
        for k in 0..g.cell_count() {
            for j in g.neighbours(k) {
                assert!(g.neighbours(j).any(|i| i == k), "{k} {j}");
            }
        }
    }
}
