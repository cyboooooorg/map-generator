//! The planet model. Owner: mapgen1.
//!
//! ```text
//! pub fn class_of(trial: &Trial, trace: &Trace) -> Result<TrialOutcome, SystemError>
//! pub fn class_read(trial: &Trial, trace: &Trace) -> Result<ClassRead, SystemError>
//! pub fn model(placement: &Placement, trace: &mut Trace) -> Result<PlanetState, SystemError>
//! ```
//!
//! All three run one private chain (`chain`), from the trial seed: the
//! composition, the snowball draw and the life draw derive from
//! `trial.seed` (`placement.trial.seed` in `model`), never from the trace,
//! so the same trial gives the same answer. [`class_read`] clones `trace`
//! into a scratch trace (so the `Traced` ids the trial holds stay valid),
//! runs the chain with the spin drawn from the trial seed
//! (`spin::spin`, the rocky law, as the draw draws the placement's), and
//! returns the class, the radius, the density and the obliquity the
//! climate read; [`class_of`] keeps the outcome. Legacy's `trial_class`
//! (`legacy-final:crates/mapgen/src/system.rs:896`) ran without a spin.
//! [`model`] runs the chain on the placement's trial with the placement's
//! spin (the climate reads its obliquity node itself) and builds the
//! [`PlanetState`] from what the chain recorded. The draw classifies the
//! circular trial and `model` runs on the eccentric one (its higher mean
//! flux), so their classes may differ; at `e = 0` they agree.
//!
//! The errors of the chain are [`InteriorError`]s, reaching the caller as
//! `SystemError::Planet`; the spin's are the system's own.
//!
//! # The chain
//!
//! [`draws`] draws a planet's composition, [`interior`] builds its
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
//! any climate. [`climate`] holds the climate solver (the body it settles,
//! its albedo, its wet balance, the collapse of its CO₂, the carbonate
//! thermostat, the maximum greenhouse, the fixed points of its
//! condensation and of its albedo and emissivity, the temperate and
//! frozen branches, the snowball chance and the state its draw finds,
//! and [`climate::record`], which records the settled climate in the
//! cause trace).
//!
//! In order, `chain` runs: the draw; the ground ([`bare`]'s `formed`: a
//! planet that kept an envelope stops there, with no spin and no
//! surface); the rotation (the obliquity, and `locked` an imposed false
//! until #604); [`settled`]: the ground and the air, the climate, the
//! reducing-air fallback, the boil loop (a wet planet with no wet balance
//! boils: run again from the steam forced on its ocean, its leftover
//! condensed again when that run does not boil) and the steam world (its
//! envelope and its inflated radius); [`life`]: the life stage, the air
//! it oxygenated and the climate solved again on that air; [`classify`]:
//! the class. Every value is recorded in the system's cause `Trace`, with
//! the codes of [`codes`].

mod air;
mod bare;
mod boil_off;
mod chain;
mod classify;
mod climate;
mod codes;
mod core;
mod draws;
mod envelope;
mod enveloped;
mod escape;
mod heat;
mod interior;
mod life;
mod regime;
mod settled;
mod volcanism;
mod water;

#[cfg(test)]
mod air_tests;
#[cfg(test)]
mod bare_tests;
#[cfg(test)]
mod boil_off_tests;
#[cfg(test)]
mod chain_tests;
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

pub use crate::spin::is_giant;
pub use climate::ALBEDO_TOLERANCE_K;
pub use interior::InteriorError;

use crate::{Placement, SystemError, Trial, TrialOutcome};
use chain::{Spun, chain, obliquity_of, state_of};
use science::units::Degree;
use timeline::{PlanetState, Trace};

/// What [`class_of`] read of a trial (D2): the outcome, and the obliquity
/// the climate read, `None` for a planet without a surface (one that kept
/// an envelope). Plain values: the chain ran on a scratch trace whose node
/// ids mean nothing in the caller's.
///
/// For a planet with a surface the obliquity is the one the draw gives
/// the placement of the same trial, bit for bit: both are drawn from the
/// trial seed with the rocky law, which reads neither the mass nor the
/// radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClassRead {
    pub outcome: TrialOutcome,
    pub obliquity: Option<Degree>,
}

/// The class, radius and density of `trial`, and the obliquity its
/// climate read: the chain run on a scratch copy of `trace`, with the spin
/// drawn from the trial seed. `trace` is left as it was.
///
/// # Errors
///
/// `SystemError::Planet` when the chain fails (an interior, an envelope,
/// a trace error), and the spin's errors.
pub fn class_read(trial: &Trial, trace: &Trace) -> Result<ClassRead, SystemError> {
    let mut scratch = trace.clone();
    let chained = chain(trial, Spun::Trial, &mut scratch)?;
    let bulk = chained.living.settled.bare.body.bulk();
    Ok(ClassRead {
        outcome: TrialOutcome {
            class: *chained.class.value(),
            radius: *bulk.radius.value(),
            mean_density: *bulk.mean_density.value(),
        },
        obliquity: obliquity_of(&chained),
    })
}

/// The classifier of the draw: [`class_read`]'s outcome.
///
/// # Errors
///
/// As [`class_read`].
pub fn class_of(trial: &Trial, trace: &Trace) -> Result<TrialOutcome, SystemError> {
    class_read(trial, trace).map(|read| read.outcome)
}

/// The state of the planet `placement`: the chain run on its trial (the
/// drawn, possibly eccentric, orbit and its sources) from the trial seed,
/// with the placement's spin, recorded in `trace`. The state validates.
///
/// # Errors
///
/// As [`class_read`], and `SystemError::Contract` when the state does not
/// validate.
pub fn model(placement: &Placement, trace: &mut Trace) -> Result<PlanetState, SystemError> {
    let chained = chain(&placement.trial, Spun::Placed(&placement.spin), trace)?;
    let state = state_of(placement, chained, trace)?;
    state.validate()?;
    Ok(state)
}
