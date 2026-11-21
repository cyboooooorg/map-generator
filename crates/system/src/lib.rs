//! The system engine: computes a system from its birth to its horizon
//! (a hierarchical tree of orbits, stars with phases, global climate,
//! interiors, events) and publishes one timeline per body.
//!
//! Owns the request and archetype types (the inputs) and the
//! `EnvironmentSource` port. Never depends on the map generator.

mod archetype;
mod draw;
pub mod engine;
mod error;
mod families;
mod inputs;
mod placement;
pub mod planet;
mod random;
mod sources;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "wired by the system draw, Task 10")
)]
mod spin;
pub mod star;

pub use archetype::ArchetypeId;
pub use draw::{Drawn, draw_with};
pub use error::SystemError;
pub use families::Family;
pub use inputs::{BodyRequest, INPUTS_SCHEMA, Inputs, Request, SystemRequest};
pub use placement::{Placement, Trial, TrialOutcome};

use timeline::EngineVersion;

/// The version of the engines: bumped whenever a timeline or a world
/// could change for the same inputs. A cache key, never migrated.
pub const ENGINE_VERSION: EngineVersion = EngineVersion(1);
