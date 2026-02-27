use serde::Serialize;
use std::fmt;

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

#[derive(Clone, Copy, Serialize)]
pub enum Biome {
    // ── Standard water ────────────────────────────────────────────────────────
    DeepOcean,
    Ocean,
    // ── Shore ─────────────────────────────────────────────────────────────────
    Beach,
    Wetland,
    // ── Cold ──────────────────────────────────────────────────────────────────
    IceCap,
    Tundra,
    Taiga,
    // ── Temperate ─────────────────────────────────────────────────────────────
    Shrubland,
    Plain,
    Forest,
    // ── Tropical ──────────────────────────────────────────────────────────────
    Savanna,
    Desert,
    Jungle,
    // ── High elevation ────────────────────────────────────────────────────────
    Mountain,
    Snow,
    // ── Volcanic (Terran + Volcanic world) ────────────────────────────────────
    /// Active caldera / summit vent — molten rock at the peak.
    Volcano,
    /// Cooling lava flows spreading down volcanic flanks.
    LavaField,
    /// Barren ash-covered terrain surrounding a volcanic chain.
    AshLand,
    // ── Volcanic world exclusives ─────────────────────────────────────────────
    /// Seas of liquid rock; replaces ocean basins on volcanic worlds.
    MagmaSea,
    /// Vitrified rock scoured by superheated winds; replaces temperate land.
    ScorchedWaste,
    // ── Frozen world exclusives ───────────────────────────────────────────────
    /// Permanently ice-covered ocean; replaces open water on frozen worlds.
    FrozenOcean,
    /// Flat permafrost plains swept by blizzards; replaces temperate lowlands.
    GlacialPlain,
    // ── Caustic world exclusives ──────────────────────────────────────────────
    /// Pools and seas of corrosive liquid; replaces ocean basins.
    CausticLake,
    /// Rain-drenched wetlands saturated with toxic runoff.
    ToxicSwamp,
    /// Bleached flatlands encrusted with acid-precipitate minerals.
    AcidFlatland,
    // ── Barren world exclusives ───────────────────────────────────────────────
    /// Shattered boulder fields and exposed bedrock; replaces ocean (no water).
    RockyWaste,
    /// Fine regolith plains scoured by dry winds.
    DustPlain,
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

/// Canonical biome → RGB colour mapping, shared by all export backends.
pub fn biome_color(b: Biome) -> [u8; 3] {
    match b {
        // Water
        Biome::DeepOcean => [10, 20, 140],
        Biome::Ocean => [30, 70, 200],
        // Shore
        Biome::Beach => [220, 210, 120],
        Biome::Wetland => [90, 140, 80],
        // Cold
        Biome::IceCap => [210, 235, 255],
        Biome::Tundra => [160, 185, 155],
        Biome::Taiga => [30, 90, 60],
        // Temperate
        Biome::Shrubland => [170, 180, 80],
        Biome::Plain => [100, 200, 80],
        Biome::Forest => [20, 110, 20],
        // Tropical
        Biome::Savanna => [210, 190, 60],
        Biome::Desert => [240, 200, 100],
        Biome::Jungle => [0, 90, 20],
        // High elevation
        Biome::Mountain => [130, 120, 110],
        Biome::Snow => [245, 245, 250],
        // Volcanic (Terran + Volcanic world)
        Biome::Volcano => [255, 50, 0],
        Biome::LavaField => [200, 80, 10],
        Biome::AshLand => [95, 80, 70],
        // Volcanic world exclusives
        Biome::MagmaSea => [180, 20, 0],
        Biome::ScorchedWaste => [70, 35, 15],
        // Frozen world exclusives
        Biome::FrozenOcean => [140, 195, 235],
        Biome::GlacialPlain => [200, 220, 240],
        // Caustic world exclusives
        Biome::CausticLake => [60, 170, 40],
        Biome::ToxicSwamp => [45, 100, 20],
        Biome::AcidFlatland => [165, 185, 60],
        // Barren world exclusives
        Biome::RockyWaste => [110, 103, 90],
        Biome::DustPlain => [195, 168, 110],
    }
}
