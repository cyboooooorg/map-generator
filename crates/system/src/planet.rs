//! The planet model. Owner: mapgen1.
//!
//! Future signatures:
//!
//! ```text
//! pub fn class_of(trial: &Trial) -> Result<PlanetClass, SystemError>
//! pub fn model(placement: &Placement, trace: &mut Trace) -> Result<PlanetState, SystemError>
//! ```
//!
//! `class_of` runs `model` with a scratch `Trace` and returns the
//! class: one classifier. `model` draws its own inputs from sub-seeds
//! of the body seed and runs the archetype's backward search.
//! `SystemError` does not exist yet.
