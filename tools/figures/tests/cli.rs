//! The `figures` binary: `--check` reports without writing, a plain run
//! writes into a directory it creates, a bad flag is a usage error.

use std::path::Path;
use std::process::{Command, Output};

fn figures(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_figures"))
        .args(args)
        .output()
        .expect("the figures binary runs")
}

fn path(dir: &Path) -> &str {
    dir.to_str().expect("a UTF-8 temporary path")
}

#[test]
fn check_on_an_empty_directory_names_every_figure_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let out = figures(&["--out", path(dir.path()), "--check"]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    for figure in ["main-sequence", "giant-radius"] {
        assert!(stderr.contains(&format!("{figure}.svg")), "{stderr}");
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a_run_creates_the_directory_and_then_check_passes() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a").join("b");
    let out = figures(&["--out", path(&nested)]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(nested.join("main-sequence.svg").is_file());
    let check = figures(&["--out", path(&nested), "--check"]);
    assert_eq!(check.status.code(), Some(0), "{check:?}");
}

#[test]
fn an_unknown_flag_is_a_usage_error() {
    let out = figures(&["--nope"]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("unknown argument --nope"), "{stderr}");
    assert!(stderr.contains("usage:"), "{stderr}");
}
