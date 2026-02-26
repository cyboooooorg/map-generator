mod export_json;
mod export_png;
mod generation;
mod world;

use export_json::export_json;
use export_png::export_png;
use generation::generate_world;
use world::PlanetType;

fn main() {
    // planet_type:        master archetype that drives global climate and unlocks exclusive biomes.
    //   PlanetType::Terran   — Earth-like (full biome spectrum)
    //   PlanetType::Volcanic — Fire world (MagmaSea, ScorchedWaste, extreme heat)
    //   PlanetType::Frozen   — Ice world  (FrozenOcean, GlacialPlain, perpetual cold)
    //   PlanetType::Caustic  — Acid world (CausticLake, ToxicSwamp, AcidFlatland)
    //   PlanetType::Barren   — Dead rock  (RockyWaste, DustPlain, no water)
    //
    // sea_level:          0.0 = default, +0.3 = 70 % ocean (Earth-like), -0.3 = mostly land
    // volcanic_intensity: 0.0 = no volcanoes, 0.5 = some volcanic chains, 1.0 = many volcanoes
    let world = generate_world(1920, 1080, rand::random(), 0.2, 0.8, PlanetType::Terran);

    let dir = format!("worlds/{}", world.seed);
    std::fs::create_dir_all(&dir).expect("failed to create output directory");

    export_png(&world, &format!("{}/world.png", dir));
    export_json(&world, &format!("{}/world.json", dir));

    println!("World generated → {}/", dir);
}
