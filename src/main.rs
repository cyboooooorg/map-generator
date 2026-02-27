mod export_json;
mod export_png;
mod generation;
mod world;

use export_json::export_json;
use export_png::export_png;
use generation::generate_world;
use rand::RngExt;
use world::PlanetType;

fn main() {
    let mut rng = rand::rng();

    // Parse optional named arguments:
    //   --planet    terran | volcanic | frozen | caustic | barren
    //   --sea-level <f32>          (default: random -0.3 .. 0.5)
    //   --volcanic  <f32>          (default: random 0.0 .. 1.0)
    //
    // Any omitted parameter is chosen randomly.
    let mut planet_arg: Option<String> = None;
    let mut sea_level_arg: Option<f32> = None;
    let mut volcanic_arg: Option<f32> = None;
    let mut circumference_arg: Option<f32> = None;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--planet" => {
                idx += 1;
                planet_arg = args.get(idx).cloned();
            }
            "--sea-level" => {
                idx += 1;
                sea_level_arg = args.get(idx).and_then(|v| v.parse().ok());
            }
            "--volcanic" => {
                idx += 1;
                volcanic_arg = args.get(idx).and_then(|v| v.parse().ok());
            }
            "--circumference" => {
                idx += 1;
                circumference_arg = args.get(idx).and_then(|v| v.parse().ok());
            }
            other => eprintln!("warning: unknown argument '{other}' — ignored"),
        }
        idx += 1;
    }

    let planet_type = match planet_arg.as_deref() {
        Some("terran") => PlanetType::Terran,
        Some("volcanic") => PlanetType::Volcanic,
        Some("frozen") => PlanetType::Frozen,
        Some("caustic") => PlanetType::Caustic,
        Some("barren") => PlanetType::Barren,
        Some(other) => {
            eprintln!("warning: unknown planet type '{other}', picking randomly");
            random_planet(&mut rng)
        }
        None => random_planet(&mut rng),
    };

    let sea_level = sea_level_arg.unwrap_or_else(|| rng.random_range(-0.30_f32..0.50));
    let volcanic_intensity = volcanic_arg.unwrap_or_else(|| rng.random_range(0.00_f32..1.00));
    // Default: random planet in the range of small rocky worlds to super-Earths.
    // Earth ≈ 40 075 km.  Range 20 000–80 000 km covers sub-Earth to ~2× Earth.
    let circumference_km =
        circumference_arg.unwrap_or_else(|| rng.random_range(20_000.0_f32..80_000.0));

    // Gravity mirrors the formula in generate_world — printed before generation
    // so the user sees it even without inspecting the JSON output.
    let gravity_preview = circumference_km / 40_075.0_f32;
    println!(
        "Parameters → planet={planet_type:?}  sea_level={sea_level:.2}  volcanic_intensity={volcanic_intensity:.2}  circumference={circumference_km:.0} km  gravity≈{gravity_preview:.2}g"
    );

    let world = generate_world(
        1920,
        1080,
        rand::random(),
        sea_level,
        volcanic_intensity,
        planet_type,
        circumference_km,
    );

    let dir = format!("worlds/{}-{}", planet_type, world.seed);
    std::fs::create_dir_all(&dir).expect("failed to create output directory");

    export_png(&world, &format!("{}/world.png", dir));
    export_json(&world, &format!("{}/world.json", dir));

    println!("World generated → {}/", dir);
}

fn random_planet(rng: &mut impl rand::RngExt) -> PlanetType {
    match rng.random_range(0u8..5) {
        0 => PlanetType::Terran,
        1 => PlanetType::Volcanic,
        2 => PlanetType::Frozen,
        3 => PlanetType::Caustic,
        _ => PlanetType::Barren,
    }
}
