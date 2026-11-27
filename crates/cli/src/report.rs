//! The report of `map-generator system`: the draw as one JSON document.

use science::units::EarthMass;
use serde::Serialize;
use system::planet::{class_of, model};
use system::{ArchetypeId, ENGINE_VERSION, Inputs, draw_with};
use timeline::{
    BodyState, CauseGraph, Designation, EngineVersion, Keyframe, NodeId, PlanetState, Seed,
    Timeline, Trace, Traced, TracedFields,
};

use crate::error::CliError;

/// The version of the report's shape.
pub const REPORT_SCHEMA: u32 = 2;

/// A drawn system: the inputs to replay it, the star, the planets and
/// the causes of the planets' fields.
#[derive(Debug, Serialize)]
pub struct SystemReport {
    pub schema: u32,
    /// The inputs as given, to rerun the draw.
    pub inputs: Inputs,
    pub engine: EngineVersion,
    /// The family of the system, as a code.
    pub family: &'static str,
    /// One keyframe, the present.
    pub star: Timeline,
    /// In the draw's order, closest first.
    pub planets: Vec<PlanetDraw>,
    /// The causes of `planets`, frozen.
    pub causes: CauseGraph,
}

/// What the draw decides of a planet, and the state the planet model
/// computes from it. The drawn orbit, spin and stellar sources are in
/// `state` only, which holds the same nodes: one field per node, so that
/// a code such as `system.draw.period` names one pointer per planet.
#[derive(Debug, Serialize)]
pub struct PlanetDraw {
    pub designation: Designation,
    pub seed: Seed,
    pub archetype: Option<ArchetypeId>,
    /// The drawn mass. It stays beside `state`'s bulk mass because the two
    /// differ once an envelope boils off; without a boil-off they are the
    /// same node, so `system.draw.mass` names two pointers per planet.
    pub mass: Traced<EarthMass>,
    /// The planet model's state on the drawn (possibly eccentric) trial:
    /// its orbit, spin, stellar sources, class, climate, liquid and
    /// biosphere.
    pub state: PlanetState,
}

impl TracedFields for PlanetDraw {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        self.mass.for_each_node(f);
        self.state.for_each_node(f);
    }
}

/// Draws the system of `inputs` and gathers the report.
pub fn report(inputs: &Inputs) -> Result<SystemReport, CliError> {
    let mut trace = Trace::new();
    let drawn = draw_with(inputs, &mut trace, class_of)?;
    let present = *drawn.age.value();
    let star = Timeline::new(
        drawn.designation,
        drawn.seed,
        ENGINE_VERSION,
        present,
        vec![Keyframe::new(present, BodyState::Star(drawn.star), &trace)?],
    )?;
    let mut planets = Vec::with_capacity(drawn.placements.len());
    for placement in drawn.placements {
        let state = model(&placement, &mut trace)?;
        planets.push(PlanetDraw {
            designation: placement.designation,
            seed: placement.seed,
            archetype: placement.trial.archetype,
            mass: placement.trial.mass,
            state,
        });
    }
    let causes = trace.freeze(&mut planets)?;
    Ok(SystemReport {
        schema: REPORT_SCHEMA,
        inputs: inputs.clone(),
        engine: ENGINE_VERSION,
        family: drawn.family.code(),
        star,
        planets,
        causes,
    })
}
