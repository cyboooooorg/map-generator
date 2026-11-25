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
//! one classifier. `model` runs the same chain with the same trial seed
//! (the spin is drawn from it too) and the spin, and the
//! archetype's backward search. `SystemError` does not exist yet.
//!
//! # What exists today
//!
//! The first slice of the chain: [`draws`] draws a planet's composition
//! from sub-seeds of the seed it is given (`chain`'s `seed`: the trial
//! seed for `class_of` and `model`), [`interior`] builds its
//! structure with the `science` integrator (a planet without an envelope,
//! or a mini-Neptune's core), [`enveloped`] the mini-Neptunes and giants
//! around it, [`boil_off`] what a mini-Neptune's envelope loses to the
//! boil-off when its disc goes, [`escape`] what their envelope loses to
//! the stars' XUV light, [`heat`] computes its internal heat, [`water`]
//! the water it keeps under its stars' light and the planet left after a
//! runaway's steam, [`regime`] its tectonic regime, [`core`] its iron core
//! and its dynamo, [`air`] the air it outgasses and the cosmic shoreline
//! keeps, what the stars' ultraviolet leaves of its methane, whether it is
//! reducing (its CO2 then held back), what the cold of its equilibrium
//! temperature traps and what the photochemistry makes of it. [`bare`]
//! chains them in legacy's order, from the draw to the bare air, before
//! any climate: draw, interior, boil-off, escape and heat; for a planet
//! without an envelope, its water, the planet a runaway's steam left (body
//! and heat built again), regime, core and air, and after a runaway whose shoreline took vapour,
//! the body built once more for the water kept. [`climate`] holds the
//! first part of the climate solver (the body it settles, its albedo,
//! its wet balance, the collapse of its CO₂, the carbonate thermostat,
//! the maximum greenhouse, and the fixed points of its condensation and
//! of its albedo and emissivity, the temperate and frozen branches, the
//! snowball chance and the state its draw finds, and [`climate::record`],
//! which records the settled climate in the cause trace), and [`settled`]
//! chains it after [`bare`]: the ground and the air, the climate, the
//! reducing-air fallback (the air made again with its CO2 when the climate
//! leaves no Titan-like air), and the boil loop (a wet planet with no wet
//! balance boils: run again from the steam forced on its ocean, its
//! leftover condensed again when that run does not boil), then the steam
//! world (its envelope and its inflated radius). `settled` is a separate
//! step, not wired into the generation chain yet (2d-4), so that `bare`
//! keeps its cost. [`life`] follows it: the life stage of the settled
//! planet, the air it oxygenated and the climate solved again on that air.
//! What remains: the class (#659). Every value is
//! recorded in the system's cause `Trace`, with the codes of [`codes`].

// The slice is consumed by the planet chain (`class_of`, `model`), which
// arrives with the next pull request of the planet interior plan.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod air;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod bare;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod boil_off;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod classify;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod climate;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod codes;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod core;
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
mod escape;
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
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod life;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod regime;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod settled;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod volcanism;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed by the planet chain, next pull request")
)]
mod water;

#[cfg(test)]
mod air_tests;
#[cfg(test)]
mod bare_tests;
#[cfg(test)]
mod boil_off_tests;
#[cfg(test)]
mod classify_tests;
#[cfg(test)]
mod core_tests;
#[cfg(test)]
mod envelope_tests;
#[cfg(test)]
mod escape_tests;
#[cfg(test)]
mod interior_tests;
#[cfg(test)]
mod life_tests;
#[cfg(test)]
mod settled_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod volcanism_tests;
#[cfg(test)]
mod water_tests;
