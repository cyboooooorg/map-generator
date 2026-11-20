//! The map generator: turns one keyframe or event state of one body,
//! read through its `TimelineSource` port, into a world on the
//! cube-sphere simulation grid.
//!
//! It spatializes the global state and never feeds anything back into the
//! system engine. It never reads the kind of the central object.
