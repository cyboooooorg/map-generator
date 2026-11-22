//! The errors of the command line: each prints its code first, so a
//! failure can be matched without parsing the sentence.

use std::fmt;

use adapters::{ExportError, StoreError};
use system::SystemError;
use timeline::{ContractError, Designation};
use world::relief::ReliefError;

use crate::explain::ExplainError;

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
    /// `--explain` could not explain the path.
    Explain(ExplainError),
    /// The OS gave no random source for the root seed.
    Seed(getrandom::Error),
    /// The output could not be written to stdout.
    Io(std::io::Error),
    /// The store could not be read or written.
    Store(StoreError),
    /// The world could not be exported.
    Export(ExportError),
    /// The body's relief could not be built.
    Relief(ReliefError),
    /// The system has no such body.
    NoSuchBody(Designation),
    /// The body is a star: only planets have a surface to map.
    NotAPlanet(Designation),
    /// The body's timeline has no keyframe at its present.
    NoPresentKeyframe(Designation),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System(error) => write!(f, "{error}"),
            Self::Contract(error) => write!(f, "cli.contract: {error}"),
            Self::Json(error) => write!(f, "cli.json: {error}"),
            Self::Explain(error) => write!(f, "{error}"),
            Self::Seed(error) => write!(f, "cli.seed.unavailable: {error}"),
            Self::Io(error) => write!(f, "cli.io: {error}"),
            Self::Store(error) => write!(f, "cli.store: {error}"),
            Self::Export(error) => write!(f, "{error}"),
            Self::Relief(error) => write!(f, "{error}"),
            Self::NoSuchBody(body) => write!(f, "cli.world.no-such-body: {body}"),
            Self::NotAPlanet(body) => write!(f, "cli.world.not-a-planet: {body}"),
            Self::NoPresentKeyframe(body) => {
                write!(f, "cli.world.no-present-keyframe: {body}")
            }
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

impl From<ExplainError> for CliError {
    fn from(error: ExplainError) -> Self {
        Self::Explain(error)
    }
}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<StoreError> for CliError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<ExportError> for CliError {
    fn from(error: ExportError) -> Self {
        Self::Export(error)
    }
}

impl From<ReliefError> for CliError {
    fn from(error: ReliefError) -> Self {
        Self::Relief(error)
    }
}
