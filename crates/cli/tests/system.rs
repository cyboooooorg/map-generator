//! The `system` command, through the binary.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_map-generator"))
        .arg("system")
        .args(args)
        .output()
        .expect("the binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

#[test]
fn prints_the_draw_as_json() {
    let output = run(&["--seed", "1"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(json["schema"], 1);
    assert_eq!(json["classifier"], "stand-in");
    for key in ["inputs", "engine", "family", "star", "planets", "causes"] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
}

#[test]
fn the_same_seed_prints_the_same_bytes() {
    let a = run(&["--seed", "42"]);
    let b = run(&["--seed", "42"]);
    assert_eq!(a.stdout, b.stdout);
}

#[test]
fn a_date_after_the_star_dies_fails() {
    let output = run(&["--seed", "1", "--host-mass", "1", "--present", "50"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).starts_with("map-generator: system.present-after-life"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_unknown_archetype_fails() {
    let output = run(&["--seed", "1", "--archetype", "pluto-like=1"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("system.unknown-archetype"));
}

#[test]
fn a_malformed_archetype_is_a_usage_error() {
    let output = run(&["--archetype", "earth-like"]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn an_omitted_seed_is_drawn_and_replayable() {
    let drawn = run(&[]);
    assert_eq!(drawn.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_str(&stdout(&drawn)).unwrap();
    let n = json["inputs"]["seed"]
        .as_u64()
        .expect("the seed is a number");
    assert_eq!(
        stderr(&drawn),
        format!("map-generator: seed {n} drawn; rerun with --seed {n}\n")
    );
    let replay = run(&["--seed", &n.to_string()]);
    assert_eq!(replay.stdout, drawn.stdout);
    assert_eq!(stderr(&replay), "");
}

#[test]
fn explain_prints_a_tree() {
    let output = run(&["--seed", "1", "--explain", "/planets/0/orbit/period"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout(&output).starts_with("system.draw.period = "),
        "{}",
        stdout(&output)
    );
}

#[test]
fn explain_errors_exit_1() {
    let output = run(&["--seed", "1", "--explain", "/planets/0/designation"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).starts_with("map-generator: cli.explain.not-traced"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_negative_host_mass_reaches_the_draw() {
    let output = run(&["--seed", "1", "--host-mass", "-1"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("system.host-mass-out-of-range"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_closed_pipe_is_not_a_panic() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_map-generator"))
        .args(["system", "--seed", "35", "--archetype", "jupiter-like=1"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the binary runs");
    drop(child.stdout.take());
    let output = child.wait_with_output().expect("the binary ends");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn explain_a_code_held_by_no_field_names_its_dependents() {
    let output = run(&["--seed", "1", "--explain", "system.draw.break-up-fraction"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("held by no field; values that depend on it: /"),
        "{}",
        stderr(&output)
    );
}
