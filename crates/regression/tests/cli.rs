//! The CLI's commands that draw a system with the planet model, through
//! the library (`map_generator::{report, explain, dispatch}`): the report
//! of a draw and `--explain`. Moved from the CLI's unit and binary tests,
//! which stay in tier 1 only where they never reach the classifier.
//!
//! No test here draws a system of its own: each draws reference 1
//! (`regression::reference_requests()[0]`, seed 1 at solar metallicity),
//! through `report` or, with the flags that give the same inputs, through
//! `dispatch`. These draws stay cheap: reference 1 is giants only (a hot
//! Jupiter and a gas giant), so the planet model solves no climate.

use std::sync::OnceLock;

use clap::Parser;
use map_generator::args::{Cli, Command};
use map_generator::explain::{ExplainError, explain};
use map_generator::report::{REPORT_SCHEMA, SystemReport, report};
use map_generator::{dispatch, error::CliError};
use regression::reference_requests;
use science::units::Gyr;
use serde_json::Value;
use system::{ArchetypeId, Inputs};
use timeline::{Code, Seed, TracedFields};

/// The flags that give reference 1's inputs.
const REFERENCE_ONE: &[&str] = &["--seed", "1", "--metallicity", "0"];

fn cli(extra: &[&str]) -> Cli {
    let mut argv = vec!["map-generator", "system"];
    argv.extend_from_slice(REFERENCE_ONE);
    argv.extend_from_slice(extra);
    Cli::try_parse_from(argv).unwrap()
}

/// The stdout of `map-generator system` on reference 1 with `extra`.
fn run(extra: &[&str]) -> Result<String, CliError> {
    let run = dispatch(&cli(extra));
    assert_eq!(run.drawn_seed, None, "the seed was given");
    run.output
}

fn reference_one() -> Inputs {
    reference_requests().swap_remove(0)
}

fn report_one() -> SystemReport {
    report(&reference_one()).unwrap()
}

/// The report of reference 1 as JSON, drawn once per process.
fn reference_one_json() -> &'static Value {
    static REPORT: OnceLock<Value> = OnceLock::new();
    REPORT.get_or_init(|| serde_json::to_value(report_one()).unwrap())
}

// The report.

#[test]
fn the_star_timeline_has_one_keyframe_at_the_present() {
    let report = report_one();
    assert!(!report.planets.is_empty(), "reference 1 draws a planet");
    assert_eq!(report.star.keyframes.len(), 1);
    assert_eq!(report.star.keyframes[0].date(), report.star.present);
}

#[test]
#[ignore = "heavy: draws a system of its own at a fixed present (no reference fixes one); tier 2 checks the keyframe on reference 1"]
fn a_fixed_present_is_the_star_s_present() {
    let Command::System(command) = cli(&["--present", "4.5", "--host-mass", "1"]).command;
    let fixed = report(&command.system.inputs(Seed::new(1))).unwrap();
    assert_eq!(fixed.star.present, Gyr(4.5));
}

#[test]
fn every_planet_node_is_in_the_planet_graph() {
    let mut report = report_one();
    let len = report.causes.nodes.len();
    report.causes.validate().unwrap();
    for planet in &mut report.planets {
        planet.for_each_node(&mut |id| assert!((id.get() as usize) < len));
    }
}

#[test]
#[ignore = "heavy: draws and reports a system of its own (215 s in debug); tier 2 keeps the coverage on the shared references"]
fn asked_archetypes_are_placed() {
    let Command::System(command) = Cli::try_parse_from([
        "map-generator",
        "system",
        "--seed",
        "3",
        "--archetype",
        "earth-like=1",
    ])
    .unwrap()
    .command;
    let report = report(&command.system.inputs(Seed::new(3))).unwrap();
    let asked = ArchetypeId(Code::of("earth-like"));
    assert!(
        report
            .planets
            .iter()
            .any(|p| p.archetype == Some(asked.clone()))
    );
}

// The command, through `dispatch`.

#[test]
fn prints_the_draw_as_json() {
    let json: Value = serde_json::from_str(&run(&[]).unwrap()).unwrap();
    assert_eq!(json["schema"], REPORT_SCHEMA);
    assert_eq!(REPORT_SCHEMA, 2);
    assert_eq!(
        json["inputs"],
        serde_json::to_value(reference_one()).unwrap(),
        "the flags give reference 1's inputs"
    );
    assert!(json.get("classifier").is_none(), "the classifier is gone");
    for key in ["inputs", "engine", "family", "star", "planets", "causes"] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    let planets = json["planets"].as_array().unwrap();
    assert!(!planets.is_empty());
    for planet in planets {
        assert!(planet["state"]["class"].is_object(), "{planet}");
        // The orbit, spin and sources are in `state` only.
        for key in ["orbit", "spin", "sources"] {
            assert!(planet.get(key).is_none(), "{key} twice: {planet}");
            assert!(!planet["state"][key].is_null(), "{key}: {planet}");
        }
    }
}

#[test]
#[ignore = "heavy: draws a system twice (260 s in debug); tier 2 checks the draw is pure on the shared references"]
fn the_same_seed_prints_the_same_bytes() {
    let seed = |seed| Cli::try_parse_from(["map-generator", "system", "--seed", seed]).unwrap();
    assert_eq!(
        dispatch(&seed("42")).output.unwrap(),
        dispatch(&seed("42")).output.unwrap()
    );
}

// `--explain`.

#[test]
fn a_pointer_to_a_period_explains_it() {
    let text = run(&["--explain", "/planets/0/state/orbit/period"]).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("system.draw.period = "), "{text}");
    assert!(text.contains("system.star.mass"));
    let depth = |line: &str| line.len() - line.trim_start().len();
    for (i, line) in lines.iter().enumerate() {
        let has_sources = lines
            .get(i + 1)
            .is_some_and(|next| depth(next) > depth(line));
        assert!(
            has_sources || line.ends_with(']') || line.ends_with("(see above)"),
            "{line}"
        );
    }
}

#[test]
fn a_unique_code_explains_it() {
    let text = explain(reference_one_json(), "system.star.mass").unwrap();
    assert!(text.starts_with("system.star.mass = "));
    assert_eq!(text.lines().count(), 1);
}

#[test]
fn a_repeated_code_is_ambiguous() {
    let report = reference_one_json();
    let planets = report["planets"].as_array().unwrap().len();
    assert!(planets >= 2, "reference 1 draws {planets} planets");
    match explain(report, "system.draw.period") {
        Err(ExplainError::Ambiguous { pointers, .. }) => {
            // One field per node: each planet holds its period once, in
            // its state's orbit.
            assert_eq!(pointers.len(), planets, "{pointers:?}");
            assert!(pointers.iter().all(|p| p.starts_with("/planets/")));
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

#[test]
fn a_code_held_by_no_field_lists_its_dependents() {
    let error = run(&["--explain", "system.draw.break-up-fraction"]).unwrap_err();
    assert!(matches!(error, CliError::Explain(_)), "{error}");
    let text = error.to_string();
    assert!(
        text.contains("held by no field; values that depend on it: /"),
        "{text}"
    );
    assert!(!text.ends_with(": "), "{text}");
    assert!(!text.contains(" 0 times"), "{text}");
}

#[test]
fn a_pointer_to_a_designation_is_not_traced() {
    let error = explain(reference_one_json(), "/planets/0/designation").unwrap_err();
    assert!(matches!(error, ExplainError::NotTraced { .. }));
}

#[test]
fn unknown_paths_are_not_found() {
    for path in ["/planets/99/mass", "system.draw.nothing"] {
        let error = explain(reference_one_json(), path).unwrap_err();
        assert!(matches!(error, ExplainError::NotFound { .. }), "{path}");
    }
}
