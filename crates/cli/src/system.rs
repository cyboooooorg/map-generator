//! The `system` command: draw one planetary system.

use clap::Args;
use timeline::Seed;

use crate::args::SystemArgs;
use crate::error::CliError;
use crate::report;

/// `map-generator system`: the draw's flags and the explain option.
#[derive(Args, Debug)]
pub struct SystemCommand {
    #[command(flatten)]
    pub system: SystemArgs,
    /// Explain one value of the draw, by its path in the output.
    #[arg(long)]
    pub explain: Option<String>,
}

/// Runs the command and returns the text for stdout. `main` resolves the
/// seed first, so this stays a function of its arguments.
pub fn run(command: &SystemCommand, seed: Seed) -> Result<String, CliError> {
    let report = report::report(&command.system.inputs(seed))?;
    Ok(serde_json::to_string_pretty(&report)?)
}
