//! The surface of a planet: what condenses on it, what covers it and how
//! bright its ground is.
//!
//! Ported from `legacy-final` (`planet/albedo.rs`, `cover` and the ground
//! of `shortwave_budget`) with the values of study
//! [24](https://github.com/cyboooooorg/map-generator/wiki/24-surface-cover-audit);
//! see `docs/science/planet-atmospheres.md`, "Climate and surface
//! liquid". The laws take a temperature, a water inventory and a radius,
//! and no solver.

mod condense;
mod cover;
mod liquids;

pub use condense::*;
pub use cover::*;
pub use liquids::*;
