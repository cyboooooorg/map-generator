//! The map generator: turns one keyframe or event state of one body,
//! read through its `TimelineSource` port, into a world on the
//! cube-sphere simulation grid.
//!
//! It spatializes the global state and never feeds anything back into the
//! system engine. It never reads the kind of the central object.

pub mod error;
pub mod fields;
pub mod grid;

pub use error::WorldError;
pub use fields::{Field, FieldId, Storage, Values, World};
pub use grid::{DEFAULT_CELL, Grid, MAX_CELLS};
