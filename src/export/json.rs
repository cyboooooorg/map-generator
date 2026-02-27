use crate::world::World;
use std::fs::File;
use std::io::Write;

pub fn export_json(world: &World, path: &str) {
    let json = serde_json::to_string_pretty(world).unwrap();
    let mut file = File::create(path).unwrap();
    file.write_all(json.as_bytes()).unwrap();
}
