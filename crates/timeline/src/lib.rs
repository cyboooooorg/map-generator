//! The published contract between the system engine and the map
//! generator: per body, keyframes (the full state at a date) and dated
//! events, with their cause trees.
//!
//! Data types with validation; the only behaviour is seed derivation
//! and freezing a keyframe's causes. A keyframe carries physical
//! forcings and states, never the kind of the central object.

mod cause;
mod designation;
mod error;
mod interior;
mod keyframe;
mod macros;
mod planet;
mod seed;
mod sources;
mod star;

pub use cause::{
    CauseGraph, Code, Node, NodeId, NodeValue, NodeValueOf, Origin, Provenance, Trace, Traced,
    TracedFields,
};
pub use designation::{BodyPath, Designation, SystemNumber, planet_letters};
pub use error::ContractError;
pub use interior::{
    InteriorProfile, Layer, Material, MatterState, RadialSample, RadialSamples, Transport,
};
pub use keyframe::{BodyState, EngineVersion, Keyframe, Timeline};
pub use planet::{
    Atmosphere, Bulk, ClimateState, Composition, Core, CoreState, Gas, GlobalClimate, Heat, Life,
    LiquidKind, Orbit, PlanetClass, PlanetState, Renewal, Spin, Surface, SurfaceLiquid,
    TargetOutcome, TectonicRegime, Terrain, TerrainUnit, Water,
};
pub use seed::Seed;
pub use sources::{StellarSource, StellarSources};
pub use star::{StarPhase, StarState};
