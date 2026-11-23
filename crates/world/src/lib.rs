//! The map generator: turns one keyframe or event state of one body,
//! read through its `TimelineSource` port, into a world on the
//! cube-sphere simulation grid.
//!
//! It spatializes the global state and never feeds anything back into the
//! system engine. It never reads the kind of the central object.

pub mod error;
pub mod fields;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod grid;
pub mod project;
pub mod relief;
pub mod stats;

use timeline::EngineVersion;

/// Definition: the version of what the map generator produces, bumped by
/// any change to it (a law, a field, a grid layout). Cached worlds of
/// another version are never read.
pub const ENGINE_VERSION: EngineVersion = EngineVersion(1);

pub use error::WorldError;
pub use fields::{Field, FieldId, Storage, Values, World};
pub use grid::{DEFAULT_CELL, Grid, MAX_CELLS, MAX_N};
pub use project::{Raster, project};
