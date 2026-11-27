//! The `world` command, through the binary: exit codes, stderr, the
//! files written. Every test here generates a system, so all are tier 3.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// Seed 15 draws MGS 99401, whose planets b to e all have a surface;
/// c is the only planet with liquid water at its surface among seeds 1
/// to 40 (288 K), so its relief has a sea.
const SEED: &str = "15";

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_map-generator"))
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

fn summary(output: &Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    serde_json::from_slice(&output.stdout).expect("the summary is JSON")
}

#[test]
#[ignore = "heavy: generates a world"]
fn exports_a_planet_then_hits_the_caches() {
    let dir = TempDir::new().unwrap();
    let first = summary(&run(dir.path(), &["--seed", SEED, "--body", "c"]));
    assert_eq!(first["body"], "MGS 99401 c");
    assert_eq!(first["timelines_cached"], false);
    assert_eq!(first["world_cached"], false);
    let png = first["png"].as_str().expect("the PNG path");
    let bytes = fs::read(png).unwrap();

    let second = summary(&run(dir.path(), &["--seed", SEED, "--body", "c"]));
    assert_eq!(second["timelines_cached"], true);
    assert_eq!(second["world_cached"], true);
    assert_eq!(second["png"], first["png"]);
    assert!(fs::read(png).unwrap() == bytes);
}

#[test]
#[ignore = "heavy: generates a system"]
fn a_drawn_seed_is_announced() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["--body", "b"]);
    assert!(
        stderr(&output).contains("drawn; rerun with --seed"),
        "{}",
        stderr(&output)
    );
}

#[test]
#[ignore = "heavy: generates a system"]
fn the_star_is_refused() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["--seed", SEED, "--body", "A"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("cli.world.not-a-planet"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        fs::read_dir(dir.path().join("out")).map_or(0, Iterator::count),
        0
    );
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
}
