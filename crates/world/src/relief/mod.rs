//! The relief of a body: noise over the sphere, shaped by the spectrum of
//! the science crate.

pub mod noise;

pub use noise::{Octaves, Spectrum, fbm, ridged};
