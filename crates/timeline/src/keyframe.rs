//! Keyframes (the full state of a body at a date) and the timeline.

use crate::{
    CauseGraph, ContractError, Designation, NodeId, PlanetState, Seed, StarState, Trace,
    TracedFields,
};
use science::units::Gyr;
use serde::{Deserialize, Serialize};

/// The state of a star or of a planet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
// A keyframe is built once and read many times: boxing the planet would
// only complicate the public constructor.
#[allow(clippy::large_enum_variant)]
pub enum BodyState {
    Star(StarState),
    Planet(PlanetState),
}

impl BodyState {
    fn validate(&self) -> Result<(), ContractError> {
        match self {
            BodyState::Star(star) => star.validate(),
            BodyState::Planet(planet) => planet.validate(),
        }
    }

    fn same_kind(&self, other: &BodyState) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl TracedFields for BodyState {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        match self {
            BodyState::Star(star) => star.for_each_node(f),
            BodyState::Planet(planet) => planet.for_each_node(f),
        }
    }
}

/// The version of the engines that made a timeline: a cache key, never
/// migrated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EngineVersion(pub u32);

fn check_date(date: Gyr, code: &str) -> Result<(), ContractError> {
    if date.0.is_finite() {
        Ok(())
    } else {
        Err(ContractError::NotFinite {
            code: code.to_owned(),
        })
    }
}

/// The full state of a body at a date, with the causes of its fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    date: Gyr,
    state: BodyState,
    causes: CauseGraph,
}

impl Keyframe {
    /// Validates the date and the state, then freezes the subgraph of
    /// `trace` the state holds and renumbers the state's ids to it.
    pub fn new(date: Gyr, mut state: BodyState, trace: &Trace) -> Result<Keyframe, ContractError> {
        check_date(date, "keyframe.date")?;
        state.validate()?;
        let causes = trace.freeze(&mut state)?;
        Ok(Keyframe {
            date,
            state,
            causes,
        })
    }

    /// For a keyframe that came through deserialisation: the graph is
    /// valid, the state is valid and every id the state holds exists.
    pub fn validate(&self) -> Result<(), ContractError> {
        check_date(self.date, "keyframe.date")?;
        self.causes.validate()?;
        self.state.validate()?;
        let mut state = self.state.clone();
        let mut missing = None;
        state.for_each_node(&mut |id| {
            if self.causes.node(*id).is_none() && missing.is_none() {
                missing = Some(id.get());
            }
        });
        match missing {
            Some(node) => Err(ContractError::UnknownNode { node }),
            None => Ok(()),
        }
    }

    pub fn date(&self) -> Gyr {
        self.date
    }

    pub fn state(&self) -> &BodyState {
        &self.state
    }

    pub fn causes(&self) -> &CauseGraph {
        &self.causes
    }
}

/// The keyframes of one body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub body: Designation,
    pub seed: Seed,
    pub engine: EngineVersion,
    pub present: Gyr,
    pub keyframes: Vec<Keyframe>,
}

impl Timeline {
    /// A timeline that passes [`Timeline::validate`].
    pub fn new(
        body: Designation,
        seed: Seed,
        engine: EngineVersion,
        present: Gyr,
        keyframes: Vec<Keyframe>,
    ) -> Result<Timeline, ContractError> {
        let timeline = Timeline {
            body,
            seed,
            engine,
            present,
            keyframes,
        };
        timeline.validate()?;
        Ok(timeline)
    }

    /// At least one keyframe, dates strictly increasing, one kind of
    /// body, every keyframe valid.
    pub fn validate(&self) -> Result<(), ContractError> {
        check_date(self.present, "timeline.present")?;
        let first = self.keyframes.first().ok_or(ContractError::NoKeyframe)?;
        for pair in self.keyframes.windows(2) {
            if pair[0].date.0 >= pair[1].date.0 {
                return Err(ContractError::KeyframesNotOrdered);
            }
        }
        if self
            .keyframes
            .iter()
            .any(|k| !k.state.same_kind(&first.state))
        {
            return Err(ContractError::MixedBodyKinds);
        }
        self.keyframes.iter().try_for_each(Keyframe::validate)
    }
}

#[cfg(test)]
#[path = "keyframe_tests.rs"]
mod tests;
