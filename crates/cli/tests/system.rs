//! The `system` command, through the binary: what needs the process (exit
//! codes, stderr, the closed pipe). The tests that draw a system are tier
//! 3; the ones asserting on the draw itself are in
//! `crates/regression/tests/cli.rs`, through the library.

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
#[ignore = "heavy: draws two systems through the binary"]
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
#[ignore = "heavy: draws a system with the planet model through the binary"]
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
#[ignore = "heavy: draws a system with the planet model through the binary"]
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
