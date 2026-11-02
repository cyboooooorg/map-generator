//! Biome definition, selection and metadata.
//!
//! This module owns the [`Biome`] type and all biome-related logic:
//!  - [`Biome`]             — the enum itself (colour, name, sort order).
//!  - [`BiomeRules`]        — every selection threshold, and the selector itself:
//!    altitude/temperature/moisture → base biome → volcanic override.
//!
//! Planet-specific climate shifts and biome remapping live in the planet
//! presets (see [`crate::preset`]).
use serde::{Deserialize, Serialize};

// ── Biome type ────────────────────────────────────────────────────────────────

/// Declaration order is the canonical display order (legend sorting relies on `Ord`).
///
/// The discriminant is the biome id written to `raw_data/biomes.png`: never
/// reorder or remove variants, only append new ones.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u8)]
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
    // ── Appended after id 26: keep new variants at the end ────────────────────
    /// Pack ice floating on polar seas.
    SeaIce,
    /// Inland water filling a basin.
    Lake,
    /// A river channel carrying the runoff of a large basin.
    River,
}

impl Biome {
    /// Every biome, in id order.
    pub const ALL: [Biome; 30] = [
        Biome::DeepOcean,
        Biome::Ocean,
        Biome::Beach,
        Biome::Wetland,
        Biome::IceCap,
        Biome::Tundra,
        Biome::Taiga,
        Biome::Shrubland,
        Biome::Plain,
        Biome::Forest,
        Biome::Savanna,
        Biome::Desert,
        Biome::Jungle,
        Biome::Mountain,
        Biome::Snow,
        Biome::Volcano,
        Biome::LavaField,
        Biome::AshLand,
        Biome::MagmaSea,
        Biome::ScorchedWaste,
        Biome::FrozenOcean,
        Biome::GlacialPlain,
        Biome::CausticLake,
        Biome::ToxicSwamp,
        Biome::AcidFlatland,
        Biome::RockyWaste,
        Biome::DustPlain,
        Biome::SeaIce,
        Biome::Lake,
        Biome::River,
    ];

    /// Whether the biome is a water surface, liquid or frozen (flat, not hillshaded).
    pub fn is_water(self) -> bool {
        matches!(
            self,
            Biome::DeepOcean
                | Biome::Ocean
                | Biome::SeaIce
                | Biome::MagmaSea
                | Biome::FrozenOcean
                | Biome::CausticLake
                | Biome::Lake
                | Biome::River
        )
    }

    /// Stable numeric id, used as the pixel value in `raw_data/biomes.png`.
    pub fn id(self) -> u8 {
        self as u8
    }
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
        Biome::SeaIce => [160, 196, 224],
        Biome::Lake => [55, 120, 210],
        Biome::River => [40, 95, 215],
    }
}

/// Human-readable name for a biome, used in the legend.
pub fn biome_name(b: Biome) -> &'static str {
    match b {
        Biome::DeepOcean => "Deep Ocean",
        Biome::Ocean => "Ocean",
        Biome::Beach => "Beach",
        Biome::Wetland => "Wetland",
        Biome::IceCap => "Ice Cap",
        Biome::Tundra => "Tundra",
        Biome::Taiga => "Taiga",
        Biome::Shrubland => "Shrubland",
        Biome::Plain => "Plain",
        Biome::Forest => "Forest",
        Biome::Savanna => "Savanna",
        Biome::Desert => "Desert",
        Biome::Jungle => "Jungle",
        Biome::Mountain => "Mountain",
        Biome::Snow => "Snow",
        Biome::Volcano => "Volcano",
        Biome::LavaField => "Lava Field",
        Biome::AshLand => "Ash Land",
        Biome::MagmaSea => "Magma Sea",
        Biome::ScorchedWaste => "Scorched Waste",
        Biome::FrozenOcean => "Frozen Ocean",
        Biome::GlacialPlain => "Glacial Plain",
        Biome::CausticLake => "Caustic Lake",
        Biome::ToxicSwamp => "Toxic Swamp",
        Biome::AcidFlatland => "Acid Flatland",
        Biome::RockyWaste => "Rocky Waste",
        Biome::DustPlain => "Dust Plain",
        Biome::SeaIce => "Sea Ice",
        Biome::Lake => "Lake",
        Biome::River => "River",
    }
}

// ── Thresholds ────────────────────────────────────────────────────────────────

/// Every threshold used by biome selection, in one place.
///
/// Elevations are relative to the shifted sea level (`elevation - sea_level`)
/// in `[-1, 1]`; temperatures in `[0, 1]`; moisture in `[-1, 1]`; volcanic
/// zone in `[0, 1]`.
///
/// In a planet preset, the `[rules]` table overrides any of these fields;
/// the others keep their default value.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct BiomeRules {
    // Altitude bands
    /// Below this elevation, a pixel is open water.
    pub waterline: f32,
    /// Below this elevation, open water is deep ocean.
    pub deep_ocean: f32,
    /// Below this temperature, the sea surface freezes into pack ice.
    pub sea_ice_temperature: f32,
    /// Shore only forms on land lower than this elevation.
    pub shore_top: f32,
    /// Shore only forms on land closer to open water than this, in km.
    pub shore_width_km: f32,
    /// Above this elevation, land is highland (mountain or snow).
    pub highland: f32,
    /// Above this elevation, highland is always snow.
    pub snow_line: f32,
    /// Below this temperature, highland is snow whatever its elevation.
    pub highland_snow_temperature: f32,

    // Temperature zones
    /// Below this temperature, land and shore are ice cap.
    pub polar: f32,
    /// Below this temperature, land is boreal (tundra, taiga).
    pub boreal: f32,
    /// Below this temperature, land is temperate; above, tropical.
    pub temperate: f32,

    // Moisture splits
    /// Shore wetter than this is wetland, otherwise beach.
    pub wetland_moisture: f32,
    /// Boreal land wetter than this is taiga, otherwise tundra.
    pub taiga_moisture: f32,
    /// Temperate land drier than this is shrubland.
    pub shrubland_moisture: f32,
    /// Temperate land wetter than this is forest.
    pub forest_moisture: f32,
    /// Tropical land drier than this is desert.
    pub desert_moisture: f32,
    /// Tropical land wetter than this is jungle; between, savanna.
    pub jungle_moisture: f32,

    // Volcanic overrides
    /// Highland above this elevation can become an active volcano.
    pub volcano_elevation: f32,
    /// Volcanic zone needed for an active volcano.
    pub volcano_zone: f32,
    /// Volcanic zone needed to turn highland into lava fields.
    pub lava_zone: f32,
    /// Land above this elevation can be covered in ash.
    pub ash_elevation: f32,
    /// Volcanic zone needed for ash land.
    pub ash_zone: f32,
}

impl Default for BiomeRules {
    fn default() -> Self {
        Self {
            waterline: -0.15,
            deep_ocean: -0.45,
            sea_ice_temperature: 0.07,
            shore_top: 0.0,
            shore_width_km: 40.0,
            highland: 0.70,
            snow_line: 0.88,
            highland_snow_temperature: 0.35,
            polar: 0.10,
            boreal: 0.30,
            temperate: 0.55,
            wetland_moisture: 0.30,
            taiga_moisture: 0.05,
            shrubland_moisture: -0.10,
            forest_moisture: 0.20,
            desert_moisture: -0.05,
            jungle_moisture: 0.30,
            volcano_elevation: 0.80,
            volcano_zone: 0.55,
            lava_zone: 0.30,
            ash_elevation: 0.30,
            ash_zone: 0.15,
        }
    }
}

/// Inputs of biome selection for one pixel.
#[derive(Clone, Copy, Debug)]
pub struct BiomeInput {
    /// Elevation relative to the shifted sea level, in `[-1, 1]`.
    pub elevation: f32,
    /// Moisture in `[-1, 1]` (`> 0` is wet).
    pub moisture: f32,
    /// Temperature in `[0, 1]` (`0` = polar, `1` = equatorial).
    pub temperature: f32,
    /// Volcanic zone in `[0, 1]` (`0` = inert, `1` = fully volcanic).
    pub volcanic_zone: f32,
    /// Distance to the nearest open water in km (`0` on water).
    pub coast_distance_km: f32,
    /// Inside a lake.
    pub lake: bool,
    /// On a river.
    pub river: bool,
}

// ── Top-level biome selector ──────────────────────────────────────────────────

impl BiomeRules {
    /// Selects the biome of a pixel: altitude band, then volcanic override.
    /// Planet presets remap the result afterwards.
    pub fn choose(&self, input: BiomeInput) -> Biome {
        let BiomeInput {
            elevation: e,
            moisture: m,
            temperature: t,
            ..
        } = input;
        let base = if e < self.waterline {
            self.ocean(e, t)
        } else if input.lake {
            if t < self.sea_ice_temperature {
                Biome::IceCap
            } else {
                Biome::Lake
            }
        } else if input.river && t >= self.polar {
            Biome::River
        } else if e < self.shore_top && input.coast_distance_km <= self.shore_width_km {
            self.shore(t, m)
        } else if e > self.highland {
            self.highland(e, t)
        } else {
            self.land(t, m)
        };
        self.apply_volcanic(base, e, input.volcanic_zone)
    }

    // ── Altitude bands ────────────────────────────────────────────────────────

    fn ocean(&self, e: f32, t: f32) -> Biome {
        if t < self.sea_ice_temperature {
            Biome::SeaIce
        } else if e < self.deep_ocean {
            Biome::DeepOcean
        } else {
            Biome::Ocean
        }
    }

    fn shore(&self, t: f32, m: f32) -> Biome {
        if t < self.polar {
            Biome::IceCap // frozen shore / pack ice
        } else if m > self.wetland_moisture {
            Biome::Wetland // mangroves / marshes
        } else {
            Biome::Beach
        }
    }

    fn highland(&self, e: f32, t: f32) -> Biome {
        if t < self.highland_snow_temperature || e > self.snow_line {
            Biome::Snow
        } else {
            Biome::Mountain
        }
    }

    // ── Land: temperature zones ───────────────────────────────────────────────

    fn land(&self, t: f32, m: f32) -> Biome {
        if t < self.polar {
            Biome::IceCap
        } else if t < self.boreal {
            if m > self.taiga_moisture {
                Biome::Taiga
            } else {
                Biome::Tundra
            }
        } else if t < self.temperate {
            if m < self.shrubland_moisture {
                Biome::Shrubland
            } else if m > self.forest_moisture {
                Biome::Forest
            } else {
                Biome::Plain
            }
        } else if m < self.desert_moisture {
            Biome::Desert
        } else if m < self.jungle_moisture {
            Biome::Savanna
        } else {
            Biome::Jungle
        }
    }

    // ── Volcanic modifier ─────────────────────────────────────────────────────

    /// Optionally overrides a biome when it sits inside an active volcanic zone.
    ///
    /// Override ladder (strongest condition wins):
    /// - **Volcano**   — summit/caldera: highland above `volcano_elevation`
    /// - **LavaField** — flanks: highland in a `lava_zone`
    /// - **AshLand**   — lower slopes: open land above `ash_elevation`
    fn apply_volcanic(&self, biome: Biome, e: f32, vz: f32) -> Biome {
        if vz <= 0.0 {
            return biome;
        }
        match biome {
            Biome::Mountain | Biome::Snow
                if e > self.volcano_elevation && vz > self.volcano_zone =>
            {
                Biome::Volcano
            }
            Biome::Mountain | Biome::Snow if vz > self.lava_zone => Biome::LavaField,
            Biome::Mountain | Biome::Snow | Biome::Shrubland | Biome::Plain | Biome::Tundra
                if e > self.ash_elevation && vz > self.ash_zone =>
            {
                Biome::AshLand
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;

    /// Every biome a planet preset can produce, over a coarse grid of inputs.
    fn reachable(preset: &Preset) -> std::collections::BTreeSet<Biome> {
        let steps = |lo: f32, hi: f32| (0..=20).map(move |i| lo + (hi - lo) * i as f32 / 20.0);
        let mut out = std::collections::BTreeSet::new();
        for e in steps(-1.0, 1.0) {
            for m in steps(-1.0, 1.0) {
                for t in steps(0.0, 1.0) {
                    for vz in steps(0.0, 1.0) {
                        for (lake, river) in [(false, false), (true, false), (false, true)] {
                            let input = BiomeInput {
                                elevation: e,
                                moisture: m,
                                temperature: t,
                                volcanic_zone: vz,
                                coast_distance_km: 0.0,
                                lake,
                                river,
                            };
                            out.insert(preset.remap(preset.rules.choose(input)));
                        }
                    }
                }
            }
        }
        out
    }

    #[test]
    fn all_lists_every_biome_in_id_order() {
        for (i, b) in Biome::ALL.iter().enumerate() {
            assert_eq!(b.id() as usize, i, "{b:?}");
        }
    }

    #[test]
    fn shore_needs_nearby_water() {
        let rules = BiomeRules::default();
        let lowland = |coast_distance_km| BiomeInput {
            elevation: -0.05,
            moisture: 0.0,
            temperature: 0.6,
            volcanic_zone: 0.0,
            coast_distance_km,
            lake: false,
            river: false,
        };
        assert_eq!(rules.choose(lowland(10.0)), Biome::Beach);
        assert_ne!(rules.choose(lowland(500.0)), Biome::Beach);
    }

    #[test]
    fn polar_seas_freeze_on_terran() {
        let input = BiomeInput {
            elevation: -0.5,
            moisture: 0.0,
            temperature: 0.05,
            volcanic_zone: 0.0,
            coast_distance_km: 0.0,
            lake: false,
            river: false,
        };
        assert_eq!(BiomeRules::default().choose(input), Biome::SeaIce);
    }

    #[test]
    fn deep_water_is_ocean_on_terran() {
        let input = BiomeInput {
            elevation: -0.9,
            moisture: 0.0,
            temperature: 0.5,
            volcanic_zone: 0.0,
            coast_distance_km: 0.0,
            lake: false,
            river: false,
        };
        assert_eq!(BiomeRules::default().choose(input), Biome::DeepOcean);
    }

    #[test]
    fn barren_worlds_have_no_water_or_life() {
        let biomes = reachable(&Preset::load("barren").unwrap());
        for b in [
            Biome::Lake,
            Biome::River,
            Biome::Ocean,
            Biome::DeepOcean,
            Biome::Forest,
            Biome::Jungle,
            Biome::Wetland,
        ] {
            assert!(!biomes.contains(&b), "{b:?} found on a barren world");
        }
    }

    #[test]
    fn exclusive_biomes_stay_on_their_planet() {
        let exclusive: [(&str, &[Biome]); 4] = [
            ("volcanic", &[Biome::MagmaSea, Biome::ScorchedWaste]),
            ("frozen", &[Biome::FrozenOcean, Biome::GlacialPlain]),
            (
                "caustic",
                &[Biome::CausticLake, Biome::ToxicSwamp, Biome::AcidFlatland],
            ),
            ("barren", &[Biome::RockyWaste, Biome::DustPlain]),
        ];
        for preset in Preset::built_in() {
            let biomes = reachable(&preset);
            for (owner, list) in exclusive {
                for b in list {
                    assert_eq!(
                        biomes.contains(b),
                        owner == preset.name,
                        "{b:?} on {}",
                        preset.name
                    );
                }
            }
        }
    }
}
