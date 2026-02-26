use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub enum Biome {
    // Water
    DeepOcean,
    Ocean,
    // Shore
    Beach,
    Wetland,
    // Cold
    IceCap,
    Tundra,
    Taiga,
    // Temperate
    Shrubland,
    Plain,
    Forest,
    // Tropical
    Savanna,
    Desert,
    Jungle,
    // High elevation
    Mountain,
    Snow,
}

#[derive(Clone, Serialize)]
pub struct Tile {
    pub q: i32,
    pub r: i32,
    pub elevation: f32,
    pub moisture: f32,
    pub temperature: f32,
    pub biome: Biome,
}

#[derive(Serialize)]
pub struct World {
    pub width: i32,
    pub height: i32,
    pub seed: u32,
    /// Elevation bias applied before biome selection.
    /// 0.0 = default. Positive → more ocean, negative → more land. Range [-1, 1].
    pub sea_level: f32,
    pub tiles: Vec<Tile>,
}
