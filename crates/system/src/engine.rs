//! The engine entry point: from the inputs to one timeline per body.
//!
//! [`generate`] draws the system ([`draw_with`] with the planet model's
//! classifier), runs the planet model once on each placement, then
//! builds one keyframe per body at the present. One `Trace` serves the
//! whole system; each keyframe freezes the part of it its state holds.
//! [`generate_with`] is the same with the classifier injected, for the
//! callers that also need the draw (the CLI report, the regression
//! references).

use timeline::{BodyState, Keyframe, Timeline, Trace};

use crate::planet::{class_of, model};
use crate::{Drawn, ENGINE_VERSION, Inputs, SystemError, Trial, TrialOutcome, draw_with};

/// What one generation produced.
#[derive(Debug)]
pub struct Generated {
    /// The draw: star, family, placements. Its traced values hold ids of
    /// `trace`'s nodes.
    pub drawn: Drawn,
    /// The system's trace: the draw's nodes, then the planet model's.
    pub trace: Trace,
    /// One timeline per body: the star's, then one per placement in the
    /// draw's order (closest first). Each has one keyframe, at the
    /// present, whose state holds ids of that keyframe's own graph.
    pub timelines: Vec<Timeline>,
}

/// The timelines of the system `inputs` asks for: the star's first, then
/// one per planet in the draw's order, each with one keyframe at the
/// present. A planet's timeline carries its body seed.
///
/// # Errors
///
/// Those of the draw (an input out of range, a request it cannot place,
/// `BodyRequestNotDrawn` for a body request), of the planet model, and a
/// contract error if a state fails its keyframe's checks.
pub fn generate(inputs: &Inputs) -> Result<Vec<Timeline>, SystemError> {
    generate_with(inputs, class_of).map(|generated| generated.timelines)
}

/// [`generate`] with the draw's classifier injected, keeping the draw and
/// the trace. The classifier is called by the draw only; the planet model
/// runs exactly once per placement.
///
/// # Errors
///
/// As [`generate`], plus those of `classify`.
pub fn generate_with(
    inputs: &Inputs,
    classify: impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
) -> Result<Generated, SystemError> {
    let mut trace = Trace::new();
    let drawn = draw_with(inputs, &mut trace, classify)?;
    let states = drawn
        .placements
        .iter()
        .map(|placement| model(placement, &mut trace))
        .collect::<Result<Vec<_>, _>>()?;
    // Every keyframe after the last model call: each freezes the whole
    // trace's subgraph its state holds.
    let present = *drawn.age.value();
    let mut timelines = Vec::with_capacity(1 + states.len());
    timelines.push(Timeline::new(
        drawn.designation,
        drawn.seed,
        ENGINE_VERSION,
        present,
        vec![Keyframe::new(
            present,
            BodyState::Star(drawn.star.clone()),
            &trace,
        )?],
    )?);
    for (placement, state) in drawn.placements.iter().zip(states) {
        timelines.push(Timeline::new(
            placement.designation,
            placement.seed,
            ENGINE_VERSION,
            present,
            vec![Keyframe::new(present, BodyState::Planet(state), &trace)?],
        )?);
    }
    Ok(Generated {
        drawn,
        trace,
        timelines,
    })
}

#[cfg(test)]
mod tests {
    use science::units::Gyr;
    use timeline::Seed;

    use super::*;
    use crate::{BodyRequest, INPUTS_SCHEMA, Request};

    #[test]
    fn a_body_request_is_not_drawn() {
        let inputs = Inputs {
            schema: INPUTS_SCHEMA,
            seed: Seed::new(1),
            present: Some(Gyr(4.57)),
            request: Request::Body(BodyRequest {
                host_mass: None,
                metallicity: None,
                archetype: None,
            }),
        };
        assert_eq!(generate(&inputs), Err(SystemError::BodyRequestNotDrawn));
    }
}
