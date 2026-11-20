//! `guardrails budget <junit.xml> <seconds> <tier>`: fails when a test
//! tier ran past its budget (CLAUDE.md, Tests).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Workspace guardrails")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fail when the run of a JUnit report took longer than the budget.
    Budget {
        /// nextest's JUnit report (target/nextest/<profile>/junit-<tier>.xml).
        report: PathBuf,
        /// Budget in seconds.
        seconds: f64,
        /// Tier name for the message, e.g. "tier 1".
        tier: String,
    },
}

fn main() -> ExitCode {
    let Command::Budget {
        report,
        seconds,
        tier,
    } = Cli::parse().command;
    let outcome = std::fs::read_to_string(&report)
        .map_err(|error| format!("cannot read {}: {error}", report.display()))
        .and_then(|xml| guardrails::budget::wall_seconds(&xml))
        .and_then(|wall| {
            guardrails::budget::check(&tier, wall, seconds)?;
            println!("{tier}: {wall:.1} s, budget {seconds:.0} s");
            Ok(())
        });
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
