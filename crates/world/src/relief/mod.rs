//! The relief of a body: noise over the sphere, shaped by the spectrum of
//! the science crate, the plates whose boundaries raise and sink it, and
//! the continents, mountains and volcanoes built on them.

pub mod noise;
pub mod passes;
pub mod plates;

use std::error::Error;
use std::fmt;

pub use noise::{Octaves, Spectrum, fbm, ridged};
pub use passes::{Forcing, continents, highlands, mountains, volcanoes};
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
