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

use crate::args::{Cli, Command};
use crate::error::CliError;

/// Runs the command of `cli` and returns the text for stdout. When the
/// seed was drawn, the notice to replay it goes on stderr.
///
/// # Errors
///
/// The command's own error: the draw, the explain path or the output.
pub fn dispatch(cli: &Cli) -> Result<String, CliError> {
    match &cli.command {
        Command::System(command) => {
            let root = command.system.root_seed()?;
            if root.drawn {
                let n = root.seed.get();
                eprintln!("map-generator: seed {n} drawn; rerun with --seed {n}");
            }
            system::run(command, root.seed)
        }
    }
}
