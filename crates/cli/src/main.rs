//! `map-generator`: the binary. Parses the arguments, runs the library's
//! `dispatch`, prints on stdout, prints the drawn seed's notice and the
//! errors on stderr, and maps the errors to exit codes.

use std::io::{self, ErrorKind, Write as _};
use std::process::ExitCode;

use clap::Parser;
use map_generator::args::Cli;
use map_generator::dispatch;
use map_generator::error::CliError;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let run = dispatch(&cli);
    if let Some(seed) = run.drawn_seed {
        let n = seed.get();
        eprintln!("map-generator: seed {n} drawn; rerun with --seed {n}");
    }
    match run.output {
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
