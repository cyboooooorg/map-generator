use serde::Serialize;
use std::fmt;

// Re-export so existing `use crate::world::*;` in other modules keeps working.
pub use crate::biome::{Biome, biome_color, biome_name, biome_order};

/// Master planet archetype.  Controls global temperature/moisture offsets and
/// unlocks planet-specific biomes during biome selection.
#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
pub enum PlanetType {
    /// Earth-like — full biome spectrum, no global modifier.
    Terran,
    /// Fire world — extreme heat, near-zero moisture, volcanic terrain dominates.
    Volcanic,
    /// Ice world — perpetually frozen, glaciers and permafrost everywhere.
    Frozen,
    /// Acid world — corrosive atmosphere, toxic wetlands, caustic pools.
    Caustic,
    /// Dead rock — arid and lifeless, dust and stone as far as the eye can see.
    Barren,
}

impl fmt::Display for PlanetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PlanetType::Terran => "terran",
            PlanetType::Volcanic => "volcanic",
            PlanetType::Frozen => "frozen",
            PlanetType::Caustic => "caustic",
            PlanetType::Barren => "barren",
        };
        f.write_str(s)
    }
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
    /// Master planet archetype driving global temperature/moisture offsets and
    /// unlocking planet-specific biomes.
    pub planet_type: PlanetType,
    /// Elevation bias applied before biome selection.
    /// 0.0 = default. Positive → more ocean, negative → more land. Range [-1, 1].
    pub sea_level: f32,
    /// Fraction of mountain chains that become volcanic.
    /// 0.0 = no volcanoes, 1.0 = most mountain chains are volcanic.
    pub volcanic_intensity: f32,
    /// Equatorial circumference of the planet in kilometres.
    /// Controls the physical scale of the world: a larger value produces
    /// broader continents and ocean basins relative to the planet's surface.
    /// Earth ≈ 40 075 km.
    pub circumference_km: f32,
    /// Approximate surface gravity relative to Earth (1.0 = Earth gravity).
    /// Derived from circumference assuming constant planetary density:
    /// g ∝ r ∝ C, so gravity_modifier = circumference_km / 40_075.
    /// Higher values flatten the landscape (mountains can't stand as tall);
    /// lower values produce more rugged, dramatic terrain.
    pub gravity_modifier: f32,
    pub tiles: Vec<Tile>,
}
