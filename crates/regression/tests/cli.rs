//! The CLI's commands that draw a system with the planet model, through
//! the library (`map_generator::{report, explain, dispatch}`): the report
//! of a draw and `--explain`. Moved from the CLI's unit and binary tests,
//! which stay in tier 1 only where they never reach the classifier.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use clap::Parser;
use map_generator::args::{Cli, Command};
use map_generator::explain::{ExplainError, explain};
use map_generator::report::{SystemReport, report};
use map_generator::{dispatch, error::CliError};
use science::units::Gyr;
use serde_json::Value;
use system::{ArchetypeId, INPUTS_SCHEMA, Inputs, Request, SystemRequest};
use timeline::{Code, Seed, TracedFields};

fn cli(extra: &[&str], seed: u64) -> Cli {
    let mut argv = vec!["map-generator", "system", "--seed"];
    let seed = seed.to_string();
    argv.push(&seed);
    argv.extend_from_slice(extra);
    Cli::try_parse_from(argv).unwrap()
}

fn inputs(extra: &[&str], seed: u64) -> Inputs {
    let Command::System(command) = cli(extra, seed).command;
    command.system.inputs(Seed::new(seed))
}

fn report_of(extra: &[&str]) -> SystemReport {
    report(&inputs(extra, 1)).unwrap()
}

/// The report of seed 1, drawn once.
fn seed_one() -> &'static Value {
    static REPORT: OnceLock<Value> = OnceLock::new();
    REPORT.get_or_init(|| serde_json::to_value(report_of(&[])).unwrap())
}

fn plain_inputs(seed: u64) -> Inputs {
    Inputs {
        schema: INPUTS_SCHEMA,
        seed: Seed::new(seed),
        present: None,
        request: Request::System(SystemRequest {
            host_mass: None,
            metallicity: None,
            archetypes: BTreeMap::new(),
        }),
    }
}

fn explain_report(seed: u64) -> Value {
    serde_json::to_value(report(&plain_inputs(seed)).unwrap()).unwrap()
}

// The report.

#[test]
fn the_star_timeline_has_one_keyframe_at_the_present() {
    let report = report_of(&[]);
    assert!(!report.planets.is_empty(), "seed 1 draws a planet");
    assert_eq!(report.star.keyframes.len(), 1);
    assert_eq!(report.star.keyframes[0].date(), report.star.present);

    let fixed = report_of(&["--present", "4.5", "--host-mass", "1"]);
    assert_eq!(fixed.star.present, Gyr(4.5));
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
#[ignore = "heavy: draws and reports a system of its own (215 s in debug); tier 2 keeps the coverage on the shared references"]
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

// The command, through `dispatch`.

#[test]
fn prints_the_draw_as_json() {
    let text = dispatch(&cli(&[], 1)).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["schema"], 1);
    assert!(json.get("classifier").is_none(), "the classifier is gone");
    for key in ["inputs", "engine", "family", "star", "planets", "causes"] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    let planets = json["planets"].as_array().unwrap();
    assert!(!planets.is_empty());
    for planet in planets {
        assert!(planet["state"]["class"].is_object(), "{planet}");
    }
}

#[test]
#[ignore = "heavy: draws a system twice (260 s in debug); tier 2 checks the draw is pure on the shared references"]
fn the_same_seed_prints_the_same_bytes() {
    assert_eq!(
        dispatch(&cli(&[], 42)).unwrap(),
        dispatch(&cli(&[], 42)).unwrap()
    );
}

#[test]
fn explain_prints_a_tree() {
    let text = dispatch(&cli(&["--explain", "/planets/0/orbit/period"], 1)).unwrap();
    assert!(text.starts_with("system.draw.period = "), "{text}");
}

#[test]
fn explain_a_code_held_by_no_field_names_its_dependents() {
    let error = dispatch(&cli(&["--explain", "system.draw.break-up-fraction"], 1)).unwrap_err();
    assert!(matches!(error, CliError::Explain(_)), "{error}");
    assert!(
        error
            .to_string()
            .contains("held by no field; values that depend on it: /"),
        "{error}"
    );
}

// `--explain`.

#[test]
fn a_pointer_to_a_period_explains_it() {
    let text = explain(seed_one(), "/planets/0/orbit/period").unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("system.draw.period = "));
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
    let text = explain(seed_one(), "system.star.mass").unwrap();
    assert!(text.starts_with("system.star.mass = "));
    assert_eq!(text.lines().count(), 1);
}

#[test]
fn a_repeated_code_is_ambiguous() {
    let (seed, report) = (1..200)
        .map(|seed| (seed, explain_report(seed)))
        .find(|(_, report)| report["planets"].as_array().is_some_and(|p| p.len() >= 2))
        .expect("a seed draws two planets");
    match explain(&report, "system.draw.flux") {
        Err(ExplainError::Ambiguous { pointers, .. }) => {
            let planets = report["planets"].as_array().unwrap().len();
            // Each planet holds the flux twice: the draw's own field and
            // the copy in its state (the model's sources).
            assert_eq!(pointers.len(), 2 * planets, "seed {seed}: {pointers:?}");
            assert!(pointers.iter().all(|p| p.starts_with("/planets/")));
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

#[test]
fn a_code_held_by_no_field_lists_its_dependents() {
    let text = explain(seed_one(), "system.draw.break-up-fraction")
        .unwrap_err()
        .to_string();
    assert!(
        text.contains("held by no field; values that depend on it: /"),
        "{text}"
    );
    assert!(!text.ends_with(": "), "{text}");
    assert!(!text.contains(" 0 times"), "{text}");
}

#[test]
fn a_pointer_to_a_designation_is_not_traced() {
    let error = explain(seed_one(), "/planets/0/designation").unwrap_err();
    assert!(matches!(error, ExplainError::NotTraced { .. }));
}

#[test]
fn unknown_paths_are_not_found() {
    for path in ["/planets/99/mass", "system.draw.nothing"] {
        let error = explain(seed_one(), path).unwrap_err();
        assert!(matches!(error, ExplainError::NotFound { .. }), "{path}");
    }
}
