//! The system draw. Owner: mapgen3.
//!
//! [`draw_with`] draws the star, then (in later tasks) the family and
//! the placements. `classify` is a parameter: the planet model's
//! `class_of` is mapgen1's, and the tests pass a closure.

use timeline::{Designation, Seed, StarState, SystemNumber, Trace, Traced};

use crate::error::SystemError;
use crate::families::Family;
use crate::inputs::{Inputs, Request};
use crate::placement::{Placement, Trial, TrialOutcome};
use crate::star;
use science::units::Gyr;

#[cfg(test)]
mod tests;

/// The first survey number, and how many numbers follow it.
///
/// Definition: the survey range of a system number is 10 000 to 99 999
/// (the legacy `MGS` range, `timeline::SystemNumber`).
const NUMBER_BASE: u64 = 10_000;
const NUMBER_SPAN: u64 = 90_000;

/// A drawn system: the star's designation, seed and state, the age, the
/// family and one placement per planet.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub designation: Designation,
    pub seed: Seed,
    pub star: StarState,
    pub age: Traced<Gyr>,
    pub family: Family,
    pub placements: Vec<Placement>,
}

/// Draws the system of `inputs`, recording every node in `trace`.
///
/// This step draws the star only and returns no placement; the family is
/// a placeholder until the family draw lands. `classify` reads the class
/// of a trial; it is not called yet.
///
/// # Errors
///
/// `BodyRequestNotDrawn` for a [`Request::Body`] (a single body is not
/// drawn by this entry point), and the star's errors.
pub fn draw_with(
    inputs: &Inputs,
    trace: &mut Trace,
    _classify: impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
) -> Result<Drawn, SystemError> {
    let request = match &inputs.request {
        Request::System(request) => request,
        Request::Body(_) => return Err(SystemError::BodyRequestNotDrawn),
    };
    let star = star::star(
        request.host_mass,
        request.metallicity,
        inputs.present,
        inputs.seed,
        trace,
    )?;
    Ok(Drawn {
        designation: Designation::star(system_number(inputs.seed)?),
        seed: inputs.seed,
        star: star.state,
        age: star.age,
        family: Family::Sparse,
        placements: Vec::new(),
    })
}

/// The survey number, drawn from the seed: it may collide with another
/// system's and is never a key.
///
/// Definition: `10 000 + hash("designation") % 90 000`, agreed with
/// mapgen2.
fn system_number(seed: Seed) -> Result<SystemNumber, SystemError> {
    let number = NUMBER_BASE + seed.derive("designation").get() % NUMBER_SPAN;
    // In range by construction: 10 000 to 99 999.
    Ok(SystemNumber::new(number as u32)?)
}
