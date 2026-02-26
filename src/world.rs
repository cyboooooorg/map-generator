use serde::Serialize;

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
    pub tiles: Vec<Tile>,
}
