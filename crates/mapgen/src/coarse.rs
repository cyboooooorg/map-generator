//! Smooth fields sampled on a coarser grid than the map and interpolated.
//!
//! The climate noise fields (see [`crate::generation::ClimateField`]) have
//! no feature smaller than their finest octave, often many pixels wide on a
//! large map: sampling them at every pixel spends most of the noise stage
//! on values a cubic interpolation of a coarser grid gives within a small
//! fraction of their range. A [`CoarseField`] samples a field at nodes a
//! few pixels apart (the step chosen from the finest octave's wavelength,
//! see [`step_for`]) and interpolates every pixel with Catmull-Rom splines,
//! which pass through the nodes and keep the field's slope continuous, so
//! the thresholds it crosses (biome borders) stay smooth curves.
//!
//! The grid is equirectangular like the map: nodes evenly spaced in
//! longitude (wrapping round), and in latitude from the first row's centre
//! to the last one's.
use crate::geo::Grid;
use rayon::prelude::*;

/// Nodes per wavelength of the finest octave, at least. Perlin noise
/// varies within a lattice cell (one wavelength) along a quintic curve:
/// with 6 nodes per cell a Catmull-Rom spline follows it within about
/// 0.3 % of the field's range (see the tests).
pub const NODES_PER_WAVELENGTH: f64 = 6.0;

/// The node spacing, in pixels (1: every pixel), for a field whose finest
/// octave has `frequency` cycles per unit of the sphere's radius, on a map
/// `width` pixels around the equator.
pub fn step_for(frequency: f64, width: u32) -> u32 {
    let wavelength_px = f64::from(width) / (std::f64::consts::TAU * frequency);
    ((wavelength_px / NODES_PER_WAVELENGTH).floor() as u32).max(1)
}

/// A field sampled on a coarse equirectangular grid of a map.
pub struct CoarseField {
    /// Nodes round the equator, and from pole to pole.
    nx: usize,
    ny: usize,
    /// Map pixels per node, east-west and north-south (fractional: the
    /// nodes divide the map evenly).
    sx: f64,
    sy: f64,
    values: Vec<f32>,
}

impl CoarseField {
    /// Samples `field` (of a point on the unit sphere) on nodes about
    /// `step` pixels apart, over the map of `grid`, with its own mapping of
    /// pixels to the sphere.
    pub fn new(grid: &Grid, step: u32, field: impl Fn([f64; 3]) -> f32 + Sync) -> Self {
        let (w, h) = (f64::from(grid.width), f64::from(grid.height));
        let nx = (grid.width as usize).div_ceil(step as usize).max(4);
        let ny = ((grid.height as usize - 1).div_ceil(step as usize) + 1).max(2);
        let sx = w / nx as f64;
        let sy = (h - 1.0) / (ny - 1) as f64;
        let values = (0..nx * ny)
            .into_par_iter()
            .map(|k| {
                let (i, j) = (k % nx, k / nx);
                field(grid.unit_vector_at(i as f64 * sx, j as f64 * sy))
            })
            .collect();
        Self {
            nx,
            ny,
            sx,
            sy,
            values,
        }
    }

    /// The field at pixel `(x, y)`. Catmull-Rom splines overshoot the
    /// nodes slightly (by well under the field's own error, see the tests):
    /// the climate's consumers scale the fields linearly and clamp what
    /// they derive from them.
    pub fn at(&self, x: u32, y: u32) -> f32 {
        let (u, v) = (f64::from(x) / self.sx, f64::from(y) / self.sy);
        let (i, j) = (u.floor(), v.floor());
        let (fx, fy) = ((u - i) as f32, (v - j) as f32);
        let (i, j) = (i as isize, j as isize);
        let wx = catmull_rom(fx);
        let wy = catmull_rom(fy);
        let mut sum = 0.0f32;
        for (b, wyb) in wy.iter().enumerate() {
            // Latitude clamps at the poles; longitude wraps round.
            let row = (j + b as isize - 1).clamp(0, self.ny as isize - 1) as usize * self.nx;
            let mut line = 0.0f32;
            for (a, wxa) in wx.iter().enumerate() {
                let col = (i + a as isize - 1).rem_euclid(self.nx as isize) as usize;
                line += wxa * self.values[row + col];
            }
            sum += wyb * line;
        }
        sum
    }
}

/// Catmull-Rom weights of the four nodes around a point `t` of the way
/// from the second to the third.
fn catmull_rom(t: f32) -> [f32; 4] {
    let (t2, t3) = (t * t, t * t * t);
    [
        0.5 * (-t3 + 2.0 * t2 - t),
        0.5 * (3.0 * t3 - 5.0 * t2 + 2.0),
        0.5 * (-3.0 * t3 + 4.0 * t2 + t),
        0.5 * (t3 - t2),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::fbm;
    use noise::Perlin;

    #[test]
    fn weights_sum_to_one_and_hit_the_nodes() {
        for k in 0..=10 {
            let w = catmull_rom(k as f32 / 10.0);
            assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        }
        assert_eq!(catmull_rom(0.0), [0.0, 1.0, 0.0, 0.0]);
    }

    /// At the step chosen, an fbm field interpolated from its nodes stays
    /// within a small share of its range of the field sampled at every pixel.
    #[test]
    fn interpolated_fbm_follows_the_exact_field() {
        let perlin = Perlin::new(7);
        for (width, frequency, octaves) in [(4096u32, 1.5, 4u32), (8192, 6.0, 5), (8192, 4.0, 4)] {
            let height = width / 2;
            let top = frequency * f64::from(1u32 << (octaves - 1));
            let step = step_for(top, width);
            assert!(step >= 2, "{width} px, {top}: step {step}");
            let field = |p: [f64; 3]| {
                fbm(
                    &perlin,
                    p[0] * frequency,
                    p[1] * frequency,
                    p[2] * frequency,
                    octaves,
                )
            };
            let grid = Grid {
                width,
                height,
                circumference_km: 40_075.0,
            };
            let coarse = CoarseField::new(&grid, step, field);
            let mut worst = 0.0f32;
            for y in (0..height).step_by(7) {
                for x in (0..width).step_by(5) {
                    let exact = field(grid.unit_vector_at(f64::from(x), f64::from(y)));
                    worst = worst.max((coarse.at(x, y) - exact).abs());
                }
            }
            // fbm spans about [-0.7, 0.7]: under 1 % of it.
            assert!(
                worst < 0.01,
                "{width} px, {top}: step {step}, worst {worst}"
            );
        }
    }
}
