//! The relief of a body: noise over the sphere, shaped by the spectrum of
//! the science crate, the plates whose boundaries raise and sink it, and
//! the continents, mountains and volcanoes built on them.

pub mod noise;
pub mod passes;
pub mod plates;

use crate::fields::{Field, Storage, Values};
use crate::grid::Grid;
use crate::stats::{area_mean, area_quantile};
use science::relief::spectrum::HEIGHT_RANGE;
use science::units::Fraction;
use std::error::Error;
use std::fmt;
use timeline::{PlanetState, Seed};

pub use noise::{Octaves, Spectrum, fbm, ridged};
pub use passes::{Forcing, continents, highlands, mountains, raise_orogens, volcanoes};
pub use plates::{PlateMap, PlateRelief};

/// Why a body's relief cannot be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReliefError {
    /// The body has no solid surface: the keyframe carries none.
    NoSurface,
}

impl fmt::Display for ReliefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReliefError::NoSurface => write!(f, "relief.no-surface"),
        }
    }
}

impl Error for ReliefError {}

/// The code of the height field.
pub const HEIGHT: &str = "relief.height";

/// The height of the sea: the quantile that leaves `ocean_share` of the
/// area below it, the area mean on a dry world (no liquid, or a share of
/// 0), the maximum for a share of 1.
pub fn sea_level(grid: &Grid, heights: &[f32], ocean_share: Option<Fraction>) -> f32 {
    match ocean_share.map(Fraction::get) {
        Some(share) if share > 0.0 => area_quantile(grid, heights, share),
        _ => area_mean(grid, heights),
    }
}

/// The height of every cell of `grid` above the sea, in metres, of the
/// body whose state is `state` and whose seed is `seed`.
///
/// # Errors
///
/// [`ReliefError::NoSurface`] when the state has no solid surface.
pub fn generate(state: &PlanetState, seed: Seed, grid: &Grid) -> Result<Field, ReliefError> {
    let forcing = Forcing::read(state)?;
    let (mut height, continental) = continents(grid, seed, &forcing);
    let (weight, arcs) = if forcing.plates {
        let plates = PlateMap::draw(grid, seed.derive("relief.plates"), &continental);
        let relief = plates.relief(grid, forcing.radius, forcing.gravity);
        for (h, add) in height.iter_mut().zip(&relief.height) {
            *h += add;
        }
        raise_orogens(
            grid,
            &mut height,
            &relief.orogeny,
            &relief.elevation,
            &continental,
            forcing.ocean_share,
        );
        (relief.orogeny, relief.arc)
    } else {
        (highlands(grid, seed), vec![0.0; grid.cell_count()])
    };
    let ranges = mountains(grid, seed, &forcing, &weight);
    let cones = volcanoes(grid, seed, &forcing, &arcs);
    for ((h, m), c) in height.iter_mut().zip(&ranges).zip(&cones) {
        *h += m + c;
    }
    let sea = sea_level(grid, &height, forcing.ocean_share);
    for h in &mut height {
        *h -= sea;
    }
    let range = HEIGHT_RANGE.0 as f32;
    Ok(Field {
        values: Values::F32(height),
        categorical: false,
        storage: Storage::U16 {
            min: -range,
            max: range,
        },
    })
}

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;
