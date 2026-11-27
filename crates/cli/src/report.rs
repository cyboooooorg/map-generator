//! The report of `map-generator system`: the generated system as one JSON
//! document.

use science::units::EarthMass;
use serde::Serialize;
use system::planet::class_of;
use system::{ArchetypeId, ENGINE_VERSION, Generated, Inputs, generate_with};
use timeline::{CauseGraph, Designation, EngineVersion, NodeId, Timeline, Traced, TracedFields};

use crate::error::CliError;

/// The version of the report's shape.
pub const REPORT_SCHEMA: u32 = 3;

/// A generated system: the inputs to replay it, one timeline per body and
/// what the draw decided of each planet.
#[derive(Debug, Serialize)]
pub struct SystemReport {
    pub schema: u32,
    /// The inputs as given, to rerun the draw.
    pub inputs: Inputs,
    pub engine: EngineVersion,
    /// The family of the system, as a code.
    pub family: &'static str,
    /// One per body, `system::generate`'s output: the star, then the
    /// planets closest first. Each has one keyframe, the present, with
    /// its own cause graph.
    pub timelines: Vec<Timeline>,
    /// What the draw decided of each planet, in `timelines[1..]`'s order.
    pub planets: Vec<PlanetDraw>,
    /// The causes of `planets` (the drawn masses), frozen.
    pub causes: CauseGraph,
}

/// What the draw decides of a planet beyond its state: its archetype and
/// its drawn mass. The body seed and the state are in the planet's
/// timeline.
#[derive(Debug, Serialize)]
pub struct PlanetDraw {
    pub designation: Designation,
    pub archetype: Option<ArchetypeId>,
    /// The drawn mass. It stays beside the state's bulk mass because the
    /// two differ once an envelope boils off; without a boil-off they are
    /// the same node, so `system.draw.mass` names two pointers per planet.
    pub mass: Traced<EarthMass>,
}

impl TracedFields for PlanetDraw {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        self.mass.for_each_node(f);
    }
}

/// Generates the system of `inputs` and gathers the report.
pub fn report(inputs: &Inputs) -> Result<SystemReport, CliError> {
    let Generated {
        drawn,
        trace,
        timelines,
    } = generate_with(inputs, class_of)?;
    let mut planets: Vec<PlanetDraw> = drawn
        .placements
        .into_iter()
        .map(|placement| PlanetDraw {
            designation: placement.designation,
            archetype: placement.trial.archetype,
            mass: placement.trial.mass,
        })
        .collect();
    let causes = trace.freeze(&mut planets)?;
    Ok(SystemReport {
        schema: REPORT_SCHEMA,
        inputs: inputs.clone(),
        engine: ENGINE_VERSION,
        family: drawn.family.code(),
        timelines,
        planets,
        causes,
    })
}
