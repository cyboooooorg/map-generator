//! Draws the science book's figures: `figures [--out DIR] [--check]`.
//!
//! Without `--check`, writes every figure under DIR (default
//! `docs/science/figures`, created if missing). With it, writes nothing,
//! prints each missing or stale figure and exits 1 if there is any.

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: figures [--out DIR] [--check]";

fn main() -> ExitCode {
    let mut out = PathBuf::from("docs/science/figures");
    let mut check = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--out" => match args.next() {
                Some(dir) => out = PathBuf::from(dir),
                None => return usage("--out needs a directory"),
            },
            "--help" | "-h" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => return usage(&format!("unknown argument {other}")),
        }
    }
    let result = if check {
        figures::stale(&out).map(|stale| {
            for name in &stale {
                eprintln!(
                    "figures: {}/{name}.svg is missing or stale: run `mise run docs:figures`",
                    out.display()
                );
            }
            stale.is_empty()
        })
    } else {
        figures::write(&out).map(|names| {
            for name in names {
                println!("wrote {}/{name}.svg", out.display());
            }
            true
        })
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("figures: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("figures: {problem}\n{USAGE}");
    ExitCode::from(2)
}
