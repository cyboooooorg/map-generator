//! Resamples a generated world onto a hexagonal grid, for games and other
//! tile-based uses. See `mapgen::hex` for the output format.
use clap::Parser;
use mapgen::hex::{DEFAULT_RIVER_FLOW_KM2, resample};
use std::path::PathBuf;
use std::process::ExitCode;

/// Resample a generated world onto a pointy-top hexagonal grid.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// World directory produced by map-generator (contains raw_data/)
    world: PathBuf,

    /// Number of hex columns around the planet (the grid wraps east-west)
    #[arg(long, default_value_t = 96, value_parser = clap::value_parser!(u32).range(4..=4096))]
    columns: u32,

    /// Land hexes whose largest river drains at least this area (km²) get a river flag
    #[arg(long, default_value_t = DEFAULT_RIVER_FLOW_KM2)]
    river_flow: f32,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match resample(&args.world, args.columns, args.river_flow) {
        Ok(map) => {
            println!(
                "Hex grid {}×{} (hex ≈ {:.1} px wide) → {}/",
                map.columns,
                map.rows,
                map.hex_width_px,
                args.world.join("hex").display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
