//! `map-generator`: the composition root. It wires the adapters into the
//! engines' ports. Commands arrive with slice 1, PR 2 (`system`) and later.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!(
        "map-generator: no command yet; the rewrite workspace gains its first \
         command (system) in slice 1, PR 2. See README.md."
    );
    ExitCode::from(2)
}
