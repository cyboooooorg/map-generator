//! The `system` command: draw one planetary system.

use clap::Args;
use timeline::Seed;

use crate::args::SystemArgs;
use crate::error::CliError;
use crate::explain;
use crate::report;

/// `map-generator system`: the draw's flags and the explain option.
#[derive(Args, Debug)]
pub struct SystemCommand {
    #[command(flatten)]
    pub system: SystemArgs,
    /// Explain one value: a JSON pointer into the output (e.g.
    /// /timelines/1/keyframes/0/state/planet/orbit/period) or a cause code
    /// (e.g. system.star.mass).
    #[arg(long)]
    pub explain: Option<String>,
}

/// Runs the command and returns the text for stdout. `main` resolves the
/// seed first, so this stays a function of its arguments.
pub fn run(command: &SystemCommand, seed: Seed) -> Result<String, CliError> {
    let report = report::report(&command.system.inputs(seed))?;
    match &command.explain {
        Some(path) => Ok(explain::explain(&serde_json::to_value(&report)?, path)?),
        None => Ok(serde_json::to_string_pretty(&report)?),
    }
}
