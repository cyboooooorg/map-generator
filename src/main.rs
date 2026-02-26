mod export_json;
mod export_png;
mod generation;
mod world;

use export_json::export_json;
use export_png::export_png;
use generation::generate_world;

fn main() {
    // sea_level: 0.0 = default, +0.3 = 70% ocean (Earth-like), -0.3 = mostly land
    let world = generate_world(1920, 1080, rand::random(), 0.2);

    let dir = format!("worlds/{}", world.seed);
    std::fs::create_dir_all(&dir).expect("failed to create output directory");

    export_png(&world, &format!("{}/world.png", dir));
    export_json(&world, &format!("{}/world.json", dir));

    println!("World generated → {}/", dir);
}
