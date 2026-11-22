//! The relief of a body: noise over the sphere, shaped by the spectrum of
//! the science crate, and the plates whose boundaries raise and sink it.

pub mod noise;
pub mod plates;

pub use noise::{Octaves, Spectrum, fbm, ridged};
pub use plates::{PlateMap, PlateRelief};
