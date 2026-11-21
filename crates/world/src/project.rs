//! Equirectangular (2:1) projection of a cell field, for display and
//! export. The raster is a view: nothing reads it back into the world.

use std::f64::consts::PI;

use crate::error::WorldError;
use crate::fields::{FieldId, World};

/// Definition: the samples per mean cell side of the projection lattice:
/// two, so that a sample never skips a cell the pixel covers (Nyquist).
const SAMPLES_PER_CELL: f64 = 2.0;

/// A 2:1 raster, row-major from the north-west corner.
#[derive(Clone, Debug, PartialEq)]
pub struct Raster {
    /// Pixels per row.
    pub width: u32,
    /// Rows (`width / 2`).
    pub height: u32,
    /// `width * height` values.
    pub values: Vec<f32>,
}

/// Projects one field onto a raster of `width` by `width / 2` pixels.
///
/// Pixel `(i, j)` covers longitude `[-π + 2π·i/w, -π + 2π·(i+1)/w]` and
/// latitude `[π/2 − π·(j+1)/h, π/2 − π·j/h]`. It is sampled on a lattice
/// uniform in longitude and in `sin φ`, so every sample covers the same
/// area and the mean needs no weights. The lattice is denser than the
/// grid (`SAMPLES_PER_CELL` per mean cell side, with the longitude count
/// following the pixel's widest parallel), so a polar pixel, a long thin
/// strip, still meets every cell it covers. Samples are not capped: their
/// count is about four per grid cell over the whole raster, plus one per
/// pixel at least.
///
/// Continuous fields average the samples; categorical fields take the
/// most frequent class (ties: the smallest class).
///
/// # Errors
///
/// [`WorldError::Missing`] when the world has no field `id`.
///
/// # Panics
///
/// When `width` is zero or odd.
pub fn project(world: &World, id: &FieldId, width: u32) -> Result<Raster, WorldError> {
    assert!(
        width > 0 && width.is_multiple_of(2),
        "projection width must be even and non-zero"
    );
    let field = world
        .field(id)
        .ok_or_else(|| WorldError::Missing(id.0.clone()))?;
    let grid = world.grid();
    let height = width / 2;
    let cell = (4.0 * PI / grid.cell_count() as f64).sqrt();
    let dlon = 2.0 * PI / f64::from(width);
    let dlat = PI / f64::from(height);
    let mut values = Vec::with_capacity(width as usize * height as usize);
    let mut classes: Vec<(f32, u32)> = Vec::new();
    for j in 0..height {
        let north = PI / 2.0 - dlat * f64::from(j);
        let south = north - dlat;
        let (s_lo, s_hi) = (south.sin(), north.sin());
        let phi_max = if north * south <= 0.0 {
            0.0
        } else {
            north.abs().min(south.abs())
        };
        let kx = ((SAMPLES_PER_CELL * dlon * phi_max.cos() / cell).ceil() as usize).max(1);
        let ky = ((SAMPLES_PER_CELL * dlat / cell).ceil() as usize).max(1);
        for i in 0..width {
            let lon0 = -PI + dlon * f64::from(i);
            classes.clear();
            let mut sum = 0.0_f64;
            for b in 0..ky {
                let sin_phi = s_lo + (s_hi - s_lo) * (b as f64 + 0.5) / ky as f64;
                let cos_phi = (1.0 - sin_phi * sin_phi).max(0.0).sqrt();
                for a in 0..kx {
                    let lon = lon0 + dlon * (a as f64 + 0.5) / kx as f64;
                    let k = grid.index_of([cos_phi * lon.cos(), cos_phi * lon.sin(), sin_phi]);
                    let v = field.values.get_f32(k);
                    if field.categorical {
                        match classes.iter_mut().find(|(c, _)| *c == v) {
                            Some((_, n)) => *n += 1,
                            None => classes.push((v, 1)),
                        }
                    } else {
                        sum += f64::from(v);
                    }
                }
            }
            values.push(if field.categorical {
                let mut best = classes[0];
                for &(c, n) in &classes[1..] {
                    if n > best.1 || (n == best.1 && c < best.0) {
                        best = (c, n);
                    }
                }
                best.0
            } else {
                (sum / (kx * ky) as f64) as f32
            });
        }
    }
    Ok(Raster {
        width,
        height,
        values,
    })
}

#[cfg(test)]
#[path = "project_tests.rs"]
mod tests;
