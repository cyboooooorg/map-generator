//! `map-generator`: the composition root. It wires the adapters into the
//! engines' ports and dispatches the commands (`system` so far).
//!
//! A library so that the tier-2 tests (`crates/regression`) can run the
//! commands without the binary; `main.rs` keeps the process concerns
//! (arguments, stdout, stderr, exit codes).

pub mod args;
pub mod error;
pub mod explain;
pub mod report;
pub mod system;
mod world;

use timeline::Seed;

use crate::args::{Cli, Command};
use crate::error::CliError;

/// What a command gives back to `main`: the root seed when it was drawn
/// (`main` prints the notice to replay it, also when the command then
/// fails), and the text for stdout or the command's error.
#[derive(Debug)]
pub struct Run {
    /// The root seed drawn because `--seed` was omitted; `None` when it
    /// was given, or when drawing it failed.
    pub drawn_seed: Option<Seed>,
    /// The text for stdout, or the command's error: the draw, the explain
    /// path or the output.
    pub output: Result<String, CliError>,
}

/// Runs the command of `cli`. Prints nothing: `main` keeps stdout and
/// stderr.
#[must_use]
pub fn dispatch(cli: &Cli) -> Run {
    match &cli.command {
        Command::System(command) => match command.system.root_seed() {
            Ok(root) => Run {
                drawn_seed: root.drawn.then_some(root.seed),
                output: system::run(command, root.seed),
            },
            Err(error) => Run {
                drawn_seed: None,
                output: Err(error),
            },
        },
    }
}
