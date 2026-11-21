//! The planet model. Owner: mapgen1.
//!
//! Future signatures:
//!
//! ```text
//! pub fn class_of(trial: &Trial, trace: &Trace) -> Result<PlanetClass, SystemError>
//! pub fn model(placement: &Placement, trace: &mut Trace) -> Result<PlanetState, SystemError>
//! ```
//!
//! Both call one private chain:
//!
//! ```text
//! fn chain(
//!     trial: &Trial,
//!     seed: Seed,
//!     spin: Option<&Spin>,
//!     trace: &mut Trace,
//! ) -> Result<PlanetState, SystemError>
//! ```
//!
//! `class_of` clones `trace` into a scratch trace (so the `Traced` ids
//! the trial holds stay valid), runs the chain on the trial with
//! `trial.seed` and no spin (legacy `trial_class`,
//! `legacy-final:crates/mapgen/src/system.rs:896`, also ran without
//! one), reads the class from the state and throws the scratch away:
//! one classifier. `model` runs the same chain with the body seed
//! (`placement.seed`, never `trial.seed`) and the spin, and the
//! archetype's backward search. `SystemError` does not exist yet.
//!
//! # What exists today
//!
//! The first slice of the chain: [`draws`] draws a planet's composition
//! from sub-seeds of the seed it is given (`chain`'s `seed`: the trial
//! seed for `class_of`, the body seed for `model`), [`interior`] builds its
//! structure with the `science` integrator (a planet without an envelope,
//! or a mini-Neptune's core), [`enveloped`] the mini-Neptunes and giants
//! around it, [`heat`] computes its internal heat.
//! Every value is recorded in the system's cause `Trace`, with the codes
//! of [`codes`].

// The slice is consumed by the planet chain (`class_of`, `model`), which
// arrives with the next pull request of the planet interior plan.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod codes;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod draws;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod envelope;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod enveloped;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod heat;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod interior;

#[cfg(test)]
mod envelope_tests;
#[cfg(test)]
mod tests;
