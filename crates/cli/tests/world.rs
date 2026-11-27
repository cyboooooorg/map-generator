//! The `world` command, through the binary: exit codes, stderr, the
//! files written. The tests that refuse a flag before the system is drawn
//! are tier 1; the one that generates a world is tier 3.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// The seed of the generating test. Its body is chosen at run time, so a
/// change of the system engine moves the planet, not the test.
const SEED: &str = "15";

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_map-generator"))
}

fn run(dir: &Path, args: &[&str]) -> Output {
    binary()
        .arg("--root")
        .arg(dir.join("store"))
        .arg("world")
        .arg("--out")
        .arg(dir.join("out"))
        // A coarse grid and a small PNG keep the run short.
        .args(["--width", "64", "--cell-km", "100"])
        .args(args)
        .output()
        .expect("the binary runs")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn json(output: &Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

/// The designation of the first planet of `seed`'s system with a solid
/// surface at its present, read from the `system` command.
fn a_planet_with_a_surface(seed: &str) -> String {
    let report = json(&binary().args(["system", "--seed", seed]).output().unwrap());
    let timelines = report["timelines"].as_array().expect("the timelines");
    timelines
        .iter()
        .find(|timeline| {
            timeline["keyframes"].as_array().unwrap().iter().any(|keyframe| {
                keyframe["date"] == timeline["present"]
                    && !keyframe["state"]["planet"]["surface"].is_null()
            })
        })
        .unwrap_or_else(|| panic!("seed {seed} draws no planet with a surface: pick another"))
        ["body"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
#[ignore = "heavy: generates a system and a world"]
fn exports_a_planet_then_hits_the_caches() {
    let designation = a_planet_with_a_surface(SEED);
    let body = designation.rsplit(' ').next().unwrap();
    let dir = TempDir::new().unwrap();
    let first = json(&run(dir.path(), &["--seed", SEED, "--body", body]));
    assert_eq!(first["body"], designation.as_str());
    assert_eq!(first["timelines_cached"], false);
    assert_eq!(first["world_cached"], false);
    let png = first["png"].as_str().expect("the PNG path");
    let bytes = fs::read(png).unwrap();

    let second = json(&run(dir.path(), &["--seed", SEED, "--body", body]));
    assert_eq!(second["timelines_cached"], true);
    assert_eq!(second["world_cached"], true);
    assert_eq!(second["png"], first["png"]);
    assert!(fs::read(png).unwrap() == bytes);
}

#[test]
fn a_malformed_body_is_refused_at_once() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["--seed", SEED, "--body", "b c"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).starts_with("map-generator: cli.world.invalid-body"),
        "{}",
        stderr(&output)
    );
    assert!(!dir.path().join("store").exists());
}

#[test]
fn a_drawn_seed_is_announced_even_when_the_run_fails() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["--body", "b c"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    let (notice, error) = stderr.split_once('\n').expect("two lines");
    let n = notice
        .strip_prefix("map-generator: seed ")
        .and_then(|rest| rest.split_once(" drawn; rerun with --seed "))
        .map(|(n, again)| (n.to_owned(), again.to_owned()))
        .expect(&stderr);
    assert_eq!(n.0, n.1);
    assert!(
        error.starts_with("map-generator: cli.world.invalid-body"),
        "{stderr}"
    );
}
