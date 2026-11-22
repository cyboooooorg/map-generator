//! The report of `map-generator system`: the draw as one JSON document.

use science::units::EarthMass;
use serde::Serialize;
use system::{ArchetypeId, ENGINE_VERSION, Inputs, draw_with, stand_in};
use timeline::{
    BodyState, CauseGraph, Designation, EngineVersion, Keyframe, NodeId, Orbit, Seed, Spin,
    StellarSources, Timeline, Trace, Traced, TracedFields,
};

use crate::error::CliError;

/// The version of the report's shape.
pub const REPORT_SCHEMA: u32 = 1;

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
    /// Which classifier placed the planets: `stand-in` until the planet
    /// classifier is public.
    pub classifier: &'static str,
    /// One keyframe, the present.
    pub star: Timeline,
    /// In the draw's order, closest first.
    pub planets: Vec<PlanetDraw>,
    /// The causes of `planets`, frozen.
    pub causes: CauseGraph,
}

/// What the draw decides of a planet, before its state is computed.
#[derive(Debug, Serialize)]
pub struct PlanetDraw {
    pub designation: Designation,
    pub seed: Seed,
    pub archetype: Option<ArchetypeId>,
    pub mass: Traced<EarthMass>,
    pub orbit: Orbit,
    pub spin: Spin,
    pub sources: StellarSources,
}

impl TracedFields for PlanetDraw {
    fn for_each_node(&mut self, f: &mut dyn FnMut(&mut NodeId)) {
        self.mass.for_each_node(f);
        self.orbit.for_each_node(f);
        self.spin.for_each_node(f);
        self.sources.for_each_node(f);
    }
}

/// Draws the system of `inputs` and gathers the report.
pub fn report(inputs: &Inputs) -> Result<SystemReport, CliError> {
    let mut trace = Trace::new();
    let drawn = draw_with(inputs, &mut trace, stand_in::classify)?;
    let present = *drawn.age.value();
    let star = Timeline::new(
        drawn.designation,
        drawn.seed,
        ENGINE_VERSION,
        present,
        vec![Keyframe::new(present, BodyState::Star(drawn.star), &trace)?],
    )?;
    let mut planets: Vec<PlanetDraw> = drawn
        .placements
        .into_iter()
        .map(|placement| PlanetDraw {
            designation: placement.designation,
            seed: placement.seed,
            archetype: placement.trial.archetype,
            mass: placement.trial.mass,
            orbit: placement.trial.orbit,
            spin: placement.spin,
            sources: placement.trial.sources,
        })
        .collect();
    let causes = trace.freeze(&mut planets)?;
    Ok(SystemReport {
        schema: REPORT_SCHEMA,
        inputs: inputs.clone(),
        engine: ENGINE_VERSION,
        family: drawn.family.code(),
        classifier: "stand-in",
        star,
        planets,
        causes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{Cli, Command};
    use clap::Parser;
    use science::units::Gyr;
    use timeline::Code;

    fn inputs(extra: &[&str], seed: u64) -> Inputs {
        let mut argv = vec!["map-generator", "system"];
        argv.extend_from_slice(extra);
        let Command::System(command) = Cli::try_parse_from(argv).unwrap().command;
        command.system.inputs(Seed::new(seed))
    }

    #[test]
    fn the_star_timeline_has_one_keyframe_at_the_present() {
        let report = report(&inputs(&[], 1)).unwrap();
        assert!(!report.planets.is_empty(), "seed 1 draws a planet");
        assert_eq!(report.star.keyframes.len(), 1);
        assert_eq!(report.star.keyframes[0].date(), report.star.present);

        let fixed = report_of(&["--present", "4.5", "--host-mass", "1"]);
        assert_eq!(fixed.star.present, Gyr(4.5));
    }

    fn report_of(extra: &[&str]) -> SystemReport {
        report(&inputs(extra, 1)).unwrap()
    }

    #[test]
    fn every_planet_node_is_in_the_planet_graph() {
        let mut report = report_of(&[]);
        let len = report.causes.nodes.len();
        report.causes.validate().unwrap();
        for planet in &mut report.planets {
            planet.for_each_node(&mut |id| assert!((id.get() as usize) < len));
        }
    }

    #[test]
    fn asked_archetypes_are_placed() {
        let report = report(&inputs(&["--archetype", "earth-like=1"], 3)).unwrap();
        let asked = ArchetypeId(Code::of("earth-like"));
        assert!(
            report
                .planets
                .iter()
                .any(|p| p.archetype == Some(asked.clone()))
        );
    }
}
