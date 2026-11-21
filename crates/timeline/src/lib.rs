//! The published contract between the system engine and the map
//! generator: per body, keyframes (the full state at a date) and dated
//! events, with their cause trees.
//!
//! Data types only, with validation; no behaviour. A keyframe carries
//! physical forcings and states, never the kind of the central object.

mod cause;
mod designation;
mod error;
mod seed;

pub use cause::{
    CauseGraph, Code, Node, NodeId, NodeValue, NodeValueOf, Origin, Provenance, Trace, Traced,
    TracedFields,
};
pub use designation::{BodyPath, Designation, SystemNumber, planet_letters};
pub use error::ContractError;
pub use seed::Seed;
