//! The system draw. Owner: mapgen3.
//!
//! [`draw_with`] draws the star, the family and the planets (in a later
//! task, their designations and spins). `classify` is a parameter: the
//! planet model's `class_of` is mapgen1's, and the tests pass a closure.

use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use timeline::{Designation, Seed, StarState, SystemNumber, Trace, Traced};

use crate::archetype::{self, asked_host};
use crate::error::SystemError;
use crate::families::{Family, FamilyContext, Placed, pick_family, possible};
use crate::inputs::{Inputs, Request};
use crate::placement::{Placement, Trial, TrialOutcome};
use crate::sources::light;
use crate::spin;
use crate::star;
use science::units::Gyr;

mod place;
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
/// The star comes first (its mass and date from the request, else from
/// the asked archetypes' host), then the family on
/// `seed.derive("family")`, then the planets, sorted by semi-major axis:
/// the closest is rank 0 (`Designation::planet(system, rank)`), its body
/// seed is `seed.derive(<designation text>)` and its spin is drawn from
/// that body seed.
///
/// # Errors
///
/// `BodyRequestNotDrawn` for a [`Request::Body`] (a single body is not
/// drawn by this entry point), the asked archetypes' errors
/// (`UnknownArchetype`, `IncompatibleHost`, `TooManyPlanets`, `NoRoom`),
/// the star's errors and a classifier error on an asked archetype.
pub fn draw_with(
    inputs: &Inputs,
    trace: &mut Trace,
    classify: impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
) -> Result<Drawn, SystemError> {
    let (mut drawn, placed) = draw_system(inputs, trace, classify)?;
    let system = drawn.designation.system;
    for (rank, planet) in (0_u32..).zip(placed) {
        let designation = Designation::planet(system, rank)?;
        let seed = inputs.seed.derive(&designation.to_string());
        let archetype = planet.asked.as_ref().and_then(archetype::placement);
        let spin = spin::spin(
            planet.outcome.class,
            &planet.trial.mass,
            planet.outcome.radius,
            archetype,
            seed,
            trace,
        )?;
        drawn.placements.push(Placement {
            designation,
            seed,
            spin,
            trial: planet.trial,
        });
    }
    Ok(drawn)
}

/// The star, the family and the placed planets sorted by semi-major axis,
/// before designations. `Drawn.placements` stays empty.
pub(crate) fn draw_system(
    inputs: &Inputs,
    trace: &mut Trace,
    classify: impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
) -> Result<(Drawn, Vec<Placed>), SystemError> {
    let request = match &inputs.request {
        Request::System(request) => request,
        Request::Body(_) => return Err(SystemError::BodyRequestNotDrawn),
    };
    let (host_mass, present) = asked_host(&request.archetypes, request.host_mass, inputs.present)?;
    let star = star::star(host_mass, request.metallicity, present, inputs.seed, trace)?;
    let mut rng = ChaCha8Rng::seed_from_u64(inputs.seed.derive("family").get());
    let family = pick_family(&mut rng, |family| possible(family, &star.state));
    let ctx = FamilyContext::draw(family, &star.state, &mut rng);
    let light = light(&star)?;
    let system = place::System {
        star: &star,
        light: &light,
        ctx: &ctx,
        seed: inputs.seed,
    };
    let placed = place::place(&system, &request.archetypes, trace, &classify)?;
    let drawn = Drawn {
        designation: Designation::star(system_number(inputs.seed)?),
        seed: inputs.seed,
        star: star.state,
        age: star.age,
        family,
        placements: Vec::new(),
    };
    Ok((drawn, placed))
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
