//! The command line: the flags, and their one conversion to `Inputs`.

use std::collections::BTreeMap;

use clap::{Parser, Subcommand};
use science::units::{Dex, Gyr, SolarMass};
use system::{ArchetypeId, INPUTS_SCHEMA, Inputs, Request, SystemRequest};
use timeline::{Code, Seed};

use crate::error::CliError;
use crate::system::SystemCommand;

/// The planetary-system generator.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// The commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Draw a planetary system and print it as JSON.
    System(SystemCommand),
}

/// The flags that become the draw's [`Inputs`].
#[derive(clap::Args, Debug)]
pub struct SystemArgs {
    /// The root seed of the run [default: drawn from the OS and printed].
    #[arg(long)]
    pub seed: Option<u64>,
    /// The star's age in Gyr [default: drawn].
    #[arg(long)]
    pub present: Option<f64>,
    /// The host star's mass in M☉, 0.08-8 [default: drawn].
    #[arg(long)]
    pub host_mass: Option<f64>,
    /// The host star's metallicity in dex [default: drawn].
    #[arg(long, allow_negative_numbers = true)]
    pub metallicity: Option<f64>,
    /// ID=COUNT, repeatable; ids: earth-like, hot-jupiter, hot-neptune,
    /// jupiter-like, lava-world, mars-like, mercury-like, mini-neptune,
    /// neptune-like, ocean-world, saturn-like, super-earth, uranus-like,
    /// venus-like.
    #[arg(long = "archetype", value_name = "ID=COUNT", value_parser = parse_archetype)]
    pub archetypes: Vec<(ArchetypeId, u32)>,
}

/// A root seed, and whether it was drawn because `--seed` was omitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootSeed {
    pub seed: Seed,
    pub drawn: bool,
}

impl SystemArgs {
    /// The given seed, or a root seed drawn from the OS (`drawn: true`).
    pub fn root_seed(&self) -> Result<RootSeed, CliError> {
        if let Some(seed) = self.seed {
            return Ok(RootSeed {
                seed: Seed::new(seed),
                drawn: false,
            });
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "the root seed of a run without --seed: the top of the seed hierarchy"
        )]
        let value = getrandom::u64()?;
        Ok(RootSeed {
            seed: Seed::new(value),
            drawn: true,
        })
    }

    /// The draw's inputs. Range checks stay in the draw, which owns them.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the draw calls it once the command is wired in")
    )]
    pub fn inputs(&self, seed: Seed) -> Inputs {
        let mut archetypes = BTreeMap::new();
        for (id, count) in &self.archetypes {
            *archetypes.entry(id.clone()).or_insert(0) += count;
        }
        Inputs {
            schema: INPUTS_SCHEMA,
            seed,
            present: self.present.map(Gyr),
            request: Request::System(SystemRequest {
                host_mass: self.host_mass.map(SolarMass),
                metallicity: self.metallicity.map(Dex),
                archetypes,
            }),
        }
    }
}

/// Parses `ID=COUNT`: the shape only, the draw knows which ids exist.
pub fn parse_archetype(text: &str) -> Result<(ArchetypeId, u32), String> {
    let (id, count) = text
        .split_once('=')
        .ok_or_else(|| format!("expected ID=COUNT, got {text:?}"))?;
    let code = Code::new(id.to_owned()).map_err(|error| format!("{error}"))?;
    let count = count
        .parse::<u32>()
        .map_err(|error| format!("the count {count:?} is not a number: {error}"))?;
    Ok((ArchetypeId(code), count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn system_args(extra: &[&str]) -> Result<SystemArgs, clap::Error> {
        let mut argv = vec!["map-generator", "system"];
        argv.extend_from_slice(extra);
        let cli = Cli::try_parse_from(argv)?;
        let Command::System(command) = cli.command;
        Ok(command.system)
    }

    fn id(text: &'static str) -> ArchetypeId {
        ArchetypeId(Code::of(text))
    }

    #[test]
    fn inputs_carry_every_flag() {
        let args = system_args(&[
            "--seed",
            "7",
            "--present",
            "4.5",
            "--host-mass",
            "1.1",
            "--metallicity",
            "-0.2",
            "--archetype",
            "earth-like=2",
        ])
        .unwrap();
        assert_eq!(
            args.root_seed().unwrap(),
            RootSeed {
                seed: Seed::new(7),
                drawn: false
            }
        );
        assert_eq!(
            args.inputs(Seed::new(7)),
            Inputs {
                schema: INPUTS_SCHEMA,
                seed: Seed::new(7),
                present: Some(Gyr(4.5)),
                request: Request::System(SystemRequest {
                    host_mass: Some(SolarMass(1.1)),
                    metallicity: Some(Dex(-0.2)),
                    archetypes: BTreeMap::from([(id("earth-like"), 2)]),
                }),
            }
        );
    }

    #[test]
    fn repeated_archetypes_add_up() {
        let args =
            system_args(&["--archetype", "earth-like=1", "--archetype", "earth-like=2"]).unwrap();
        let Request::System(request) = args.inputs(Seed::new(1)).request else {
            panic!("a system request");
        };
        assert_eq!(request.archetypes, BTreeMap::from([(id("earth-like"), 3)]));
    }

    #[test]
    fn malformed_archetypes_are_usage_errors() {
        for bad in [
            "earth-like",
            "earth-like=x",
            "Earth-Like=1",
            "=2",
            "earth-like=-1",
        ] {
            let error = system_args(&["--archetype", bad]).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::ValueValidation, "{bad}");
        }
    }

    #[test]
    fn an_omitted_seed_is_drawn() {
        let args = system_args(&[]).unwrap();
        let (a, b) = (args.root_seed().unwrap(), args.root_seed().unwrap());
        assert!(a.drawn && b.drawn);
        assert_ne!(a.seed, b.seed);
    }
}
