//! The command line: the flags, and their one conversion to `Inputs`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use adapters::FsStore;
use clap::{Parser, Subcommand};
use science::units::{Dex, Gyr, SolarMass};
use system::{ArchetypeId, INPUTS_SCHEMA, Inputs, Request, SystemRequest};
use timeline::{Code, Seed};

use crate::error::CliError;
use crate::system::SystemCommand;
use crate::world::WorldCommand;

/// The planetary-system generator.
#[derive(Parser, Debug)]
#[command(
    version,
    about = "Draws planetary systems, explains their values and maps their planets"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
    #[command(subcommand)]
    pub command: Command,
}

/// The commands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Draw a planetary system and print it as JSON.
    System(SystemCommand),
    /// Build the world of one planet of a system, export it as a world
    /// file and a PNG, and print a JSON summary.
    World(WorldCommand),
}

/// The flags that become the draw's [`Inputs`].
#[derive(clap::Args, Debug)]
pub struct SystemArgs {
    /// The root seed of the run [default: drawn from the OS and printed].
    #[arg(long)]
    pub seed: Option<u64>,
    /// The star's age in Gyr [default: drawn].
    #[arg(long, allow_negative_numbers = true)]
    pub present: Option<f64>,
    /// The host star's mass in M☉, 0.08-8 [default: drawn].
    #[arg(long, allow_negative_numbers = true)]
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
    pub fn inputs(&self, seed: Seed) -> Inputs {
        let mut archetypes = BTreeMap::new();
        for (id, count) in &self.archetypes {
            let total: &mut u32 = archetypes.entry(id.clone()).or_insert(0);
            *total = total.saturating_add(*count);
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

/// The options every command shares: where the store lives and how large
/// its cache may grow.
#[derive(clap::Args, Debug)]
pub struct GlobalArgs {
    /// The store's root directory.
    #[arg(long, global = true, default_value = "map-generator-store")]
    pub root: PathBuf,
    /// The cache budget: bytes with an optional IEC suffix (KiB, MiB, GiB,
    /// TiB), default 2 GiB.
    #[arg(
        long,
        global = true,
        value_name = "SIZE",
        value_parser = parse_size,
        default_value_t = catalogue::DEFAULT_BUDGET,
        hide_default_value = true
    )]
    pub cache_budget: u64,
}

impl GlobalArgs {
    /// The filesystem store these options describe.
    pub fn store(&self) -> FsStore {
        FsStore::new(self.root.clone(), self.cache_budget)
    }
}

/// Parses a size: a whole number of bytes with an optional IEC suffix
/// (`KiB`, `MiB`, `GiB`, `TiB`). SI suffixes are refused, not guessed.
pub fn parse_size(text: &str) -> Result<u64, String> {
    let digits_end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (digits, suffix) = text.split_at(digits_end);
    if digits.is_empty() {
        return Err(format!(
            "expected a number of bytes with an optional suffix KiB, MiB, GiB or TiB, got {text:?}"
        ));
    }
    let shift = match suffix {
        "" => 0,
        "KiB" => 10,
        "MiB" => 20,
        "GiB" => 30,
        "TiB" => 40,
        "KB" | "MB" | "GB" | "TB" | "K" | "M" | "G" | "T" => {
            return Err(format!(
                "unknown size suffix {suffix:?}: use KiB, MiB, GiB or TiB, not {suffix}"
            ));
        }
        _ => {
            return Err(format!(
                "unknown size suffix {suffix:?}: use KiB, MiB, GiB or TiB"
            ));
        }
    };
    let count = digits
        .parse::<u64>()
        .map_err(|error| format!("the size {text:?} is not a number: {error}"))?;
    count
        .checked_mul(1 << shift)
        .ok_or_else(|| format!("the size {text:?} does not fit in 64 bits"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn system_args(extra: &[&str]) -> Result<SystemArgs, clap::Error> {
        let mut argv = vec!["map-generator", "system"];
        argv.extend_from_slice(extra);
        let cli = Cli::try_parse_from(argv)?;
        let Command::System(command) = cli.command else {
            panic!("the system command");
        };
        Ok(command.system)
    }

    fn world_command(extra: &[&str]) -> Result<crate::world::WorldCommand, clap::Error> {
        let mut argv = vec!["map-generator", "world"];
        argv.extend_from_slice(extra);
        let Command::World(command) = Cli::try_parse_from(argv)?.command else {
            panic!("the world command");
        };
        Ok(command)
    }

    #[test]
    fn world_flags_default() {
        let command = world_command(&["--body", "b", "--out", "o"]).unwrap();
        assert_eq!(command.body, "b");
        assert_eq!(command.out, PathBuf::from("o"));
        assert_eq!(command.cell_km, 15.0);
        assert_eq!(command.field.0.as_str(), "relief.height");
        assert_eq!(command.width, 2048);
    }

    #[test]
    fn world_needs_a_body_and_an_out() {
        for argv in [&["--body", "b"][..], &["--out", "o"][..]] {
            let error = world_command(argv).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument, "{argv:?}");
        }
    }

    #[test]
    fn world_refuses_a_cell_size_at_or_below_zero_or_not_finite() {
        for bad in ["0", "-1", "NaN", "inf", "x"] {
            let error =
                world_command(&["--body", "b", "--out", "o", "--cell-km", bad]).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::ValueValidation, "{bad}");
        }
    }

    #[test]
    fn world_refuses_an_odd_or_zero_width() {
        for bad in ["0", "63", "-2", "x"] {
            let error = world_command(&["--body", "b", "--out", "o", "--width", bad]).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::ValueValidation, "{bad}");
        }
    }

    #[test]
    fn world_help_shows_the_defaults() {
        let help = Cli::try_parse_from(["map-generator", "world", "--help"])
            .unwrap_err()
            .to_string();
        for default in [
            "[default: 15]",
            "[default: relief.height]",
            "[default: 2048]",
        ] {
            assert!(help.contains(default), "{default}\n{help}");
        }
    }

    #[test]
    fn world_refuses_a_malformed_field() {
        let error =
            world_command(&["--body", "b", "--out", "o", "--field", "Relief Height"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ValueValidation);
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
    fn repeated_archetype_counts_saturate() {
        let args = system_args(&[
            "--archetype",
            "earth-like=4294967295",
            "--archetype",
            "earth-like=1",
        ])
        .unwrap();
        let Request::System(request) = args.inputs(Seed::new(1)).request else {
            panic!("a system request");
        };
        assert_eq!(
            request.archetypes,
            BTreeMap::from([(id("earth-like"), u32::MAX)])
        );
    }

    #[test]
    fn negative_numbers_reach_the_draw() {
        let args = system_args(&["--present", "-1", "--host-mass", "-1"]).unwrap();
        assert_eq!(args.present, Some(-1.0));
        assert_eq!(args.host_mass, Some(-1.0));
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

    #[test]
    fn sizes_take_iec_suffixes() {
        assert_eq!(parse_size("1024"), Ok(1024));
        assert_eq!(parse_size("1KiB"), Ok(1024));
        assert_eq!(parse_size("3MiB"), Ok(3 << 20));
        assert_eq!(parse_size("2GiB"), Ok(catalogue::DEFAULT_BUDGET));
        assert_eq!(parse_size("1TiB"), Ok(1 << 40));
        assert_eq!(parse_size("0"), Ok(0));
    }

    #[test]
    fn si_suffixes_name_the_iec_one() {
        assert!(parse_size("1MB").unwrap_err().contains("MiB"));
    }

    #[test]
    fn malformed_sizes_are_errors() {
        for bad in ["2 GiB", "1.5GiB", "GiB", "", "-1", "16777216TiB"] {
            assert!(parse_size(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn global_options_default() {
        let cli = Cli::try_parse_from(["map-generator", "system"]).unwrap();
        assert_eq!(cli.global.root, PathBuf::from("map-generator-store"));
        assert_eq!(cli.global.cache_budget, catalogue::DEFAULT_BUDGET);
    }

    #[test]
    fn global_options_go_before_or_after_the_command() {
        for argv in [
            &[
                "map-generator",
                "--cache-budget",
                "1GiB",
                "--root",
                "r",
                "system",
            ][..],
            &[
                "map-generator",
                "system",
                "--cache-budget",
                "1GiB",
                "--root",
                "r",
            ][..],
        ] {
            let cli = Cli::try_parse_from(argv).unwrap();
            assert_eq!(cli.global.cache_budget, 1 << 30);
            assert_eq!(cli.global.root, PathBuf::from("r"));
        }
    }
}
