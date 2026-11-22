//! `map-generator`: the composition root. It wires the adapters into the
//! engines' ports and dispatches the commands (`system` so far).

mod args;
mod error;
mod explain;
mod report;
mod system;

use std::io::{self, ErrorKind, Write as _};
use std::process::ExitCode;

use clap::Parser;

use crate::args::{Cli, Command};
use crate::error::CliError;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(&cli) {
        Ok(text) => match writeln!(io::stdout().lock(), "{text}") {
            Ok(()) => ExitCode::SUCCESS,
            // The reader closed the pipe (`| head`): nothing left to say.
            Err(error) if error.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("map-generator: {}", CliError::Io(error));
                ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("map-generator: {error}");
            ExitCode::from(1)
        }
    }
}

fn dispatch(cli: &Cli) -> Result<String, CliError> {
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
