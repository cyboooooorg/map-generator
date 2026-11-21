//! The system engine: computes a system from its birth to its horizon
//! (a hierarchical tree of orbits, stars with phases, global climate,
//! interiors, events) and publishes one timeline per body.
//!
//! Owns the request and archetype types (the inputs) and the
//! `EnvironmentSource` port. Never depends on the map generator.

mod archetype;
pub mod draw;
pub mod engine;
mod error;
mod inputs;
mod placement;
pub mod planet;
pub mod star;

pub use archetype::ArchetypeId;
pub use error::SystemError;
pub use inputs::{BodyRequest, INPUTS_SCHEMA, Inputs, Request, SystemRequest};
pub use placement::{Placement, Trial};

use timeline::EngineVersion;

/// The version of the engines: bumped whenever a timeline or a world
/// could change for the same inputs. A cache key, never migrated.
pub const ENGINE_VERSION: EngineVersion = EngineVersion(1);
