use crate::geo::Grid;
use crate::noise::EARTH_CIRCUMFERENCE_KM;
use serde::Serialize;
use std::fmt;

// Re-export so existing `use crate::world::*;` in other modules keeps working.
pub use crate::biome::{Biome, biome_color, biome_name};

/// Master planet archetype.  Controls global temperature/moisture offsets and
/// unlocks planet-specific biomes during biome selection.
#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
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

/// Everything needed to generate a world. The same parameters always produce
/// the same world.
#[derive(Clone, Debug)]
pub struct WorldParams {
    pub width: u32,
    pub height: u32,
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
    /// Equatorial circumference of the planet in kilometres. Earth ≈ 40 075 km.
    pub circumference_km: f32,
    /// Tilt of the rotation axis in degrees, in `[0, 90]`. Earth ≈ 23.44°.
    /// Sets the latitude temperature profile and the tropics / polar circles.
    pub axial_tilt_deg: f32,
}

impl WorldParams {
    /// Approximate surface gravity relative to Earth (1.0 = Earth gravity).
    ///
    /// Assumes constant density, so gravity scales linearly with radius (and
    /// thus circumference). Clamped to a plausible range for rocky planets.
    pub fn gravity_modifier(&self) -> f32 {
        (self.circumference_km / EARTH_CIRCUMFERENCE_KM).clamp(0.1, 5.0)
    }

    /// The pixel grid with its physical dimensions.
    pub fn grid(&self) -> Grid {
        Grid {
            width: self.width,
            height: self.height,
            circumference_km: self.circumference_km,
        }
    }

    /// Number of pixels in each layer.
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }
}

/// A generated world: its parameters plus one value per pixel for each layer.
///
/// Every layer is stored row-major (`index = y * width + x`), with row 0 at
/// the north pole and column 0 at longitude 0.
pub struct World {
    pub params: WorldParams,
    pub width: u32,
    pub height: u32,
    /// Terrain height in `[-1, 1]`, before the sea-level shift. Biome
    /// selection uses `elevation - sea_level`.
    pub elevation: Vec<f32>,
    /// Moisture in `[-1, 1]` (`> 0` is wet), planet offset included.
    pub moisture: Vec<f32>,
    /// Temperature in `[0, 1]` (`0` = polar, `1` = equatorial), planet offset included.
    pub temperature: Vec<f32>,
    pub biomes: Vec<Biome>,
}

impl World {
    /// Index of pixel `(x, y)` in the layer vectors.
    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }
}
