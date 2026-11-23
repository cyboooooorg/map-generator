//! Laws of physics and typed units, shared by both engines.
//!
//! Functions take and return typed quantities. Nothing here knows a body,
//! a system, a seed or an identifier, and nothing draws at random: those
//! belong to the engines. Every constant cites its source (see the crate
//! README). The laws arrive with the pull requests that use them.

pub mod constants;
pub mod dynamo;
pub mod envelope;
pub mod eos;
pub mod escape;
pub mod giants;
pub mod habitable;
pub mod heat;
pub mod hydrogen;
pub mod orbit;
pub mod radiation;
pub mod relief;
pub mod stellar;
pub mod structure;
pub mod tectonics;
pub mod units;
pub mod volatiles;
pub mod water;
pub mod xuv;
