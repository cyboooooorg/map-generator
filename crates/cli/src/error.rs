//! The errors of the command line: each prints its code first, so a
//! failure can be matched without parsing the sentence.

use std::fmt;

use system::SystemError;
use timeline::ContractError;

/// A failed command. `Display` starts with the error's code.
#[derive(Debug)]
#[non_exhaustive]
pub enum CliError {
    /// The system draw failed; its text carries its own `system.*` code.
    System(SystemError),
    /// A violation of the timeline contract.
    Contract(ContractError),
    /// The output could not be written as JSON.
    Json(serde_json::Error),
    /// The OS gave no random source for the root seed.
    Seed(getrandom::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System(error) => write!(f, "{error}"),
            Self::Contract(error) => write!(f, "cli.contract: {error}"),
            Self::Json(error) => write!(f, "cli.json: {error}"),
            Self::Seed(error) => write!(f, "cli.seed.unavailable: {error}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<SystemError> for CliError {
    fn from(error: SystemError) -> Self {
        Self::System(error)
    }
}

impl From<ContractError> for CliError {
    fn from(error: ContractError) -> Self {
        Self::Contract(error)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<getrandom::Error> for CliError {
    fn from(error: getrandom::Error) -> Self {
        Self::Seed(error)
    }
}
