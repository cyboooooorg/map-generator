//! Archetypes: the named kinds of body a request can ask for.
//!
//! An archetype is split in two halves so that two sessions can work
//! at once:
//!
//! ```text
//! Archetype { id: ArchetypeId, placement: ArchetypePlacement, targets: ArchetypeTargets }
//! ```
//!
//! - `placement` is read by the system draw and filled by mapgen3: the
//!   host, the insolation or period band, the mass draw, the
//!   eccentricity, the tilt and the age.
//! - `targets` is filled by mapgen1: the planet's inputs, the
//!   present-day targets and their tolerances.
//!
//! The backward search moves only the planet's own inputs and records an
//! unreachable target as an outcome with its cause, never as a silent
//! miss.

use serde::{Deserialize, Serialize};
use timeline::Code;

/// The identifier of an archetype (`earth-like`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArchetypeId(pub Code);
