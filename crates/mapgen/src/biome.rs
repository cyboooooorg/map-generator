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
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
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
    // ── Appended after id 29 (caustic worlds) ─────────────────────────────────
    /// Open acid sea; replaces ocean on caustic worlds.
    AcidSea,
    /// Deep acid sea basins.
    DeepAcidSea,
    /// A stream of corrosive runoff; replaces rivers on caustic worlds.
    AcidRiver,
    /// Shores crusted with acid-precipitate salts; replaces beaches.
    AcidShore,
    // ── Appended after id 33 (frozen worlds) ──────────────────────────────────
    /// A lake sealed under ice; replaces lakes on frozen worlds.
    FrozenLake,
    // ── Appended after id 34 (giant planets: cloud bands, not ground) ───────
    /// A pale zone of a giant planet: rising air, high clouds.
    GiantZone,
    /// A dark belt of a giant planet: sinking air, deeper clouds.
    GiantBelt,
    /// A long-lived vortex (a great spot, a white oval, a dark spot).
    GiantStorm,
    /// The polar region of a giant: hazes, cyclones, sometimes a hexagon.
    GiantPolar,
}

impl Biome {
    /// Every biome, in id order.
    pub const ALL: [Biome; 39] = [
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
        Biome::AcidSea,
        Biome::DeepAcidSea,
        Biome::AcidRiver,
        Biome::AcidShore,
        Biome::FrozenLake,
        Biome::GiantZone,
        Biome::GiantBelt,
        Biome::GiantStorm,
        Biome::GiantPolar,
    ];

    /// Whether the biome is a water surface, liquid or frozen (flat, not hillshaded).
    /// Molten ground: no water lies on it, and rivers start at its edge.
    pub fn is_lava(self) -> bool {
        matches!(self, Biome::Volcano | Biome::LavaField)
    }

    pub fn is_water(self) -> bool {
        matches!(
            self,
            Biome::DeepOcean
                | Biome::Ocean
                | Biome::SeaIce
                | Biome::MagmaSea
                | Biome::FrozenOcean
                | Biome::CausticLake
                | Biome::AcidSea
                | Biome::DeepAcidSea
                | Biome::AcidRiver
                | Biome::FrozenLake
                | Biome::Lake
                | Biome::River
        )
    }

    /// Stable numeric id, used as the pixel value in `raw_data/biomes.png`.
    pub fn id(self) -> u8 {
        self as u8
    }
}

/// Surface colour of a biome as seen from orbit, sRGB: the palette of the
/// hillshaded renders (`world.png` and the viewer's detail tiles).
///
/// Unlike [`biome_color`], which keeps biomes apart on classification maps,
/// these colours follow satellite imagery: vegetation is dark and olive,
/// deserts are pale tan, open water is dark. Exotic biomes keep a hue of
/// their own, at the same muted saturation.
pub fn surface_color(b: Biome) -> [u8; 3] {
    match b {
        // Water: dark, so the shore tint and the sun glint read.
        Biome::DeepOcean => [8, 22, 48],
        Biome::Ocean => [18, 48, 86],
        Biome::Lake => [28, 60, 90],
        Biome::River => [45, 85, 120],
        // Shore
        Biome::Beach => [196, 178, 140],
        Biome::Wetland => [62, 78, 48],
        // Cold
        Biome::IceCap => [232, 238, 244],
        Biome::Tundra => [118, 112, 88],
        Biome::Taiga => [38, 56, 36],
        // Temperate
        Biome::Shrubland => [128, 118, 78],
        Biome::Plain => [104, 116, 62],
        Biome::Forest => [44, 66, 32],
        // Tropical
        Biome::Savanna => [150, 132, 80],
        Biome::Desert => [212, 184, 138],
        Biome::Jungle => [30, 58, 28],
        // High elevation
        Biome::Mountain => [112, 102, 90],
        Biome::Snow => [242, 244, 248],
        // Volcanic: incandescent vents, dark cooling flows, grey ash.
        Biome::Volcano => [226, 72, 20],
        Biome::LavaField => [150, 56, 24],
        Biome::AshLand => [72, 68, 66],
        Biome::MagmaSea => [196, 52, 12],
        Biome::ScorchedWaste => [66, 50, 40],
        // Ice
        Biome::FrozenOcean => [176, 198, 214],
        Biome::GlacialPlain => [214, 224, 232],
        Biome::SeaIce => [200, 214, 226],
        Biome::FrozenLake => [190, 208, 222],
        // Giants: a Jovian palette; a giant world's own replaces it.
        Biome::GiantZone => [236, 224, 198],
        Biome::GiantBelt => [174, 124, 84],
        Biome::GiantStorm => [192, 98, 64],
        Biome::GiantPolar => [140, 132, 128],
        // Caustic: sickly greens and yellows.
        Biome::CausticLake => [92, 140, 60],
        Biome::AcidSea => [52, 104, 58],
        Biome::DeepAcidSea => [26, 60, 42],
        Biome::AcidRiver => [110, 160, 70],
        Biome::AcidShore => [196, 190, 130],
        Biome::ToxicSwamp => [74, 86, 34],
        Biome::AcidFlatland => [170, 164, 96],
        // Barren
        Biome::RockyWaste => [104, 96, 86],
        Biome::DustPlain => [176, 150, 112],
    }
}

/// Classification colour of a biome: saturated and far apart, for the flat
/// SVG, the legend's key swatches, the hex grid and the biome table.
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
        // Teal-green liquids, set apart from the yellow-green acid land.
        Biome::CausticLake => [48, 168, 112],
        Biome::AcidSea => [32, 138, 100],
        Biome::DeepAcidSea => [16, 84, 66],
        Biome::AcidRiver => [64, 196, 136],
        Biome::AcidShore => [214, 220, 150],
        Biome::FrozenLake => [185, 215, 240],
        Biome::GiantZone => [240, 226, 190],
        Biome::GiantBelt => [176, 116, 70],
        Biome::GiantStorm => [204, 84, 52],
        Biome::GiantPolar => [128, 124, 136],
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
        Biome::AcidSea => "Acid Sea",
        Biome::DeepAcidSea => "Deep Acid Sea",
        Biome::AcidRiver => "Acid River",
        Biome::AcidShore => "Acid Shore",
        Biome::FrozenLake => "Frozen Lake",
        Biome::GiantZone => "Zone",
        Biome::GiantBelt => "Belt",
        Biome::GiantStorm => "Storm",
        Biome::GiantPolar => "Polar Region",
    }
}

// ── Thresholds ────────────────────────────────────────────────────────────────

/// Every threshold used by biome selection, in one place.
///
/// Elevations are relative to the shifted sea level (`elevation - sea_level`)
/// in `[-1, 1]`; temperatures in °C (annual means); moisture in `[-1, 1]`; volcanic
/// zone in `[0, 1]`.
///
/// In a planet preset, the `[rules]` table overrides any of these fields;
/// the others keep their default value.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
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
    /// Above this elevation (plus the pixel's highland offset), land is
    /// highland: mountain or snow.
    pub highland: f32,
    /// Above this elevation, highland is always snow.
    pub snow_line: f32,
    /// Below this summer temperature, highland is snow whatever its
    /// elevation: snow that lasts the year.
    pub highland_snow_temperature: f32,

    // Temperature zones
    /// Below this summer temperature, land and shore are ice cap, and
    /// rivers freeze: ice needs a summer below freezing (Köppen's frost
    /// climate EF, warmest month below 0 °C; Kottek et al. 2006,
    /// doi:10.1127/0941-2948/2006/0130).
    pub polar: f32,
    /// Below this annual mean temperature, land is boreal (tundra, taiga).
    pub boreal: f32,
    /// Below this summer temperature, boreal land is tundra whatever its
    /// moisture: too cold a summer for trees (Köppen's tundra climate ET,
    /// warmest month below 10 °C; Kottek et al. 2006).
    pub tundra_summer: f32,
    /// Below this annual mean temperature, land is temperate; above,
    /// tropical.
    pub temperate: f32,
    /// Below this winter temperature, land is never tropical: it is
    /// temperate at most. Köppen's tropical climates A need a coldest month
    /// of at least 18 °C (Kottek et al. 2006); warmer land with cooler
    /// winters is warm temperate (C) or dry (B). A high tilt gives its
    /// poles a warm annual mean but freezing winters.
    pub tropical_winter: f32,

    // Legacy fields, read from worlds and systems made before the arid
    // criterion (format version 10 and older), never written
    /// Former cut-off: temperate land with a moisture under it was
    /// shrubland. Only present in the rules stored with an older world,
    /// whose biomes are re-chosen with the former rules (see
    /// [`is_legacy`](Self::is_legacy)); in a preset it is dropped with a
    /// warning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shrubland_moisture: Option<f32>,
    /// Former cut-off: tropical land with a moisture under it was desert
    /// (see [`shrubland_moisture`](Self::shrubland_moisture)).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub desert_moisture: Option<f32>,

    // Dry land (Köppen's B)
    /// Land with an annual rain under this many times its dryness
    /// threshold is arid: steppe (shrubland) or desert. Köppen's 10
    /// (Kottek et al. 2006, doi:10.1127/0941-2948/2006/0130, Table 1:
    /// `P_ann < 10 P_th`).
    pub steppe_ratio: f32,
    /// Arid land with an annual rain under this many times the dryness
    /// threshold is desert, otherwise steppe. Köppen's 5 (`P_ann ≤ 5 P_th`).
    pub desert_ratio: f32,

    // Moisture splits
    /// Shore with a moisture index above this (1 000 mm of rain a year at
    /// 0) is wetland, otherwise beach.
    pub wetland_moisture: f32,
    /// Boreal land wetter than this is taiga, otherwise tundra.
    pub taiga_moisture: f32,
    /// Temperate land wetter than this is forest.
    pub forest_moisture: f32,
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
    /// Volcanic zone needed to turn dry land below the highland (forest,
    /// plain, desert…) into lava fields, and into a volcano halfway from it
    /// to 1: the flanks and vent of a cone rising out of the lowland. Above
    /// the background of a volcanic province, so only the cones reach it.
    pub vent_zone: f32,
}

impl Default for BiomeRules {
    fn default() -> Self {
        Self {
            // Altitude lines, in elevation units (25 km) above the sea
            // level; the waterline is 0.15 below it. Set for the floating
            // crust's hypsometry (see `generation::floating`): deep ocean
            // 5 km under the waterline, shores under 1.5 km above it,
            // highland from 2 km, snow from 4.5 km, volcanoes from 3 km,
            // ash from 1.5 km.
            waterline: -0.15,
            deep_ocean: -0.35,
            sea_ice_temperature: -13.5,
            shore_top: -0.09,
            shore_width_km: 40.0,
            highland: -0.07,
            snow_line: 0.03,
            highland_snow_temperature: 0.0,
            polar: 0.0,
            boreal: 0.0,
            tundra_summer: 10.0,
            temperate: 12.0,
            tropical_winter: 18.0,
            wetland_moisture: 0.0,
            taiga_moisture: -0.33,
            shrubland_moisture: None,
            desert_moisture: None,
            steppe_ratio: 10.0,
            desert_ratio: 5.0,
            forest_moisture: -0.25,
            jungle_moisture: 0.10,
            volcano_elevation: -0.03,
            volcano_zone: 0.55,
            lava_zone: 0.30,
            ash_elevation: -0.09,
            ash_zone: 0.15,
            vent_zone: 0.45,
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
    /// Annual mean temperature in °C.
    pub temperature: f32,
    /// Half the difference between summer and winter temperatures, °C
    /// (see [`crate::climate::Seasons`]): summer is `temperature` plus it,
    /// winter `temperature` minus it.
    pub seasonality: f32,
    /// Volcanic zone in `[0, 1]` (`0` = inert, `1` = fully volcanic).
    pub volcanic_zone: f32,
    /// Distance to the nearest open water in km (`0` on water).
    pub coast_distance_km: f32,
    /// Inside a lake.
    pub lake: bool,
    /// On a river.
    pub river: bool,
    /// Local shift of the highland line, so mountain borders do not follow
    /// elevation contours.
    pub highland_offset: f32,
}

impl BiomeInput {
    /// Summer temperature, °C.
    pub fn summer(&self) -> f32 {
        self.temperature + self.seasonality
    }

    /// Winter temperature, °C.
    pub fn winter(&self) -> f32 {
        self.temperature - self.seasonality
    }
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
        let summer = input.summer();
        // Lava overrides water: no lake or river inside a lava field or a
        // volcano, whose rivers start at its edge instead.
        if (input.lake || input.river) && e >= self.waterline {
            let dry = self.choose(BiomeInput {
                lake: false,
                river: false,
                ..input
            });
            if dry.is_lava() {
                return dry;
            }
        }
        let base = if e < self.waterline {
            self.ocean(e, t)
        } else if input.lake {
            if t < self.sea_ice_temperature {
                Biome::IceCap
            } else {
                Biome::Lake
            }
        } else if input.river && !self.frozen(input) {
            Biome::River
        } else if e < self.shore_top && input.coast_distance_km <= self.shore_width_km {
            self.shore(summer, m)
        } else if e + input.highland_offset > self.highland {
            self.highland(e + input.highland_offset, summer)
        } else {
            self.land(t, m, summer, input.winter())
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

    /// Whether the ground stays frozen all year (summer below
    /// [`polar`](Self::polar)): ice, and no flowing river.
    pub fn frozen(&self, input: BiomeInput) -> bool {
        input.summer() < self.polar
    }

    fn shore(&self, summer: f32, m: f32) -> Biome {
        if summer < self.polar {
            Biome::IceCap // frozen shore / pack ice
        } else if m > self.wetland_moisture {
            Biome::Wetland // mangroves / marshes
        } else {
            Biome::Beach
        }
    }

    fn highland(&self, e: f32, summer: f32) -> Biome {
        if summer < self.highland_snow_temperature || e > self.snow_line {
            Biome::Snow
        } else {
            Biome::Mountain
        }
    }

    // ── Land: temperature zones ───────────────────────────────────────────────

    /// Land from its annual mean `t`, moisture `m`, and its `summer` and
    /// `winter` temperatures.
    fn land(&self, t: f32, m: f32, summer: f32, winter: f32) -> Biome {
        if let (Some(shrubland), Some(desert)) = (self.shrubland_moisture, self.desert_moisture) {
            return self.legacy_land(t, m, shrubland, desert);
        }
        if summer < self.polar {
            Biome::IceCap
        } else if summer < self.tundra_summer && t < self.boreal {
            // Köppen tests the polar climates E first, then the arid B.
            Biome::Tundra
        } else if let Some(arid) = self.arid(t, m, summer, winter) {
            arid
        } else if t < self.boreal {
            if m > self.taiga_moisture {
                Biome::Taiga
            } else {
                Biome::Tundra
            }
        } else if t < self.temperate || winter < self.tropical_winter {
            if m > self.forest_moisture {
                Biome::Forest
            } else {
                Biome::Plain
            }
        } else if m < self.jungle_moisture {
            Biome::Savanna
        } else {
            Biome::Jungle
        }
    }

    /// Whether these rules come from a world made before the arid
    /// criterion: its moisture is the former linear index (not the log of
    /// the annual rain), and its biomes were chosen by cut-offs on it.
    pub fn is_legacy(&self) -> bool {
        self.shrubland_moisture.is_some() && self.desert_moisture.is_some()
    }

    /// The same rules for the current model: the former moisture cut-offs
    /// are dropped and the six moisture thresholds, which meant another
    /// scale, take their current defaults.
    pub fn modernised(&self) -> Self {
        if !self.is_legacy() {
            return self.clone();
        }
        let d = Self::default();
        Self {
            shrubland_moisture: None,
            desert_moisture: None,
            wetland_moisture: d.wetland_moisture,
            taiga_moisture: d.taiga_moisture,
            forest_moisture: d.forest_moisture,
            jungle_moisture: d.jungle_moisture,
            ..self.clone()
        }
    }

    /// Land as chosen before the seasons and the arid criterion (format
    /// version 10): annual mean `t` and linear moisture `m` only.
    fn legacy_land(&self, t: f32, m: f32, shrubland: f32, desert: f32) -> Biome {
        if t < self.polar {
            Biome::IceCap
        } else if t < self.boreal {
            if m > self.taiga_moisture {
                Biome::Taiga
            } else {
                Biome::Tundra
            }
        } else if t < self.temperate {
            if m < shrubland {
                Biome::Shrubland
            } else if m > self.forest_moisture {
                Biome::Forest
            } else {
                Biome::Plain
            }
        } else if m < desert {
            Biome::Desert
        } else if m < self.jungle_moisture {
            Biome::Savanna
        } else {
            Biome::Jungle
        }
    }

    /// Köppen's arid climates B: the annual rain of the moisture index `m`
    /// against the dryness threshold of the temperature and the season of
    /// the rain (Kottek et al. 2006, doi:10.1127/0941-2948/2006/0130):
    /// desert (BW) under [`desert_ratio`](Self::desert_ratio) thresholds,
    /// steppe (shrubland, BS) under [`steppe_ratio`](Self::steppe_ratio).
    pub fn arid(&self, t: f32, m: f32, summer: f32, winter: f32) -> Option<Biome> {
        let ratio =
            crate::climate::rain_mm(m) / crate::climate::dryness_threshold_mm(t, summer, winter);
        if ratio <= self.desert_ratio {
            Some(Biome::Desert)
        } else if ratio < self.steppe_ratio {
            Some(Biome::Shrubland)
        } else {
            None
        }
    }

    // ── Volcanic modifier ─────────────────────────────────────────────────────

    /// Optionally overrides a biome when it sits inside an active volcanic zone.
    ///
    /// Override ladder (strongest condition wins):
    /// - **Volcano**   — summit/caldera: highland above `volcano_elevation`
    /// - **LavaField** — flanks: highland in a `lava_zone`
    /// - **AshLand**   — lower slopes: open land above `ash_elevation`
    ///
    /// Other dry land in a `vent_zone` becomes a volcano or a lava field too.
    fn apply_volcanic(&self, biome: Biome, e: f32, vz: f32) -> Biome {
        if vz <= 0.0 {
            return biome;
        }
        match biome {
            Biome::Tundra
            | Biome::Taiga
            | Biome::Shrubland
            | Biome::Plain
            | Biome::Forest
            | Biome::Savanna
            | Biome::Desert
            | Biome::Jungle
                if vz > self.vent_zone =>
            {
                if vz > 0.5 * (1.0 + self.vent_zone) {
                    Biome::Volcano
                } else {
                    Biome::LavaField
                }
            }
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

    #[test]
    fn lava_overrides_lakes_and_rivers() {
        let rules = crate::preset::Preset::load("terran").unwrap().rules;
        let input = |lake: bool, river: bool, volcanic_zone: f32| BiomeInput {
            elevation: rules.highland + 0.05,
            moisture: 0.2,
            temperature: 20.0,
            seasonality: 0.0,
            volcanic_zone,
            coast_distance_km: 1000.0,
            lake,
            river,
            highland_offset: 0.0,
        };
        let lava = rules.lava_zone + 0.05;
        assert!(rules.choose(input(false, false, lava)).is_lava());
        assert!(rules.choose(input(true, false, lava)).is_lava());
        assert!(rules.choose(input(false, true, lava)).is_lava());
        // Away from lava, water stays water.
        assert_eq!(rules.choose(input(true, false, 0.0)), Biome::Lake);
        assert_eq!(rules.choose(input(false, true, 0.0)), Biome::River);
    }
    #[test]
    fn arid_land_follows_koppens_b_criterion() {
        use crate::climate::{dryness_threshold_mm, moisture_index};
        let rules = BiomeRules::default();
        // 20 °C all year: P_th is 54 mm, so desert up to 270 mm, steppe up
        // to 540 mm.
        let at = |mm: f32| {
            rules.choose(BiomeInput {
                elevation: -0.12,
                moisture: moisture_index(mm),
                temperature: 20.0,
                seasonality: 0.0,
                volcanic_zone: 0.0,
                coast_distance_km: 1000.0,
                lake: false,
                river: false,
                highland_offset: 0.0,
            })
        };
        assert_eq!(dryness_threshold_mm(20.0, 20.0, 20.0), 54.0);
        assert_eq!(at(200.0), Biome::Desert);
        assert_eq!(at(400.0), Biome::Shrubland);
        assert_ne!(at(700.0), Biome::Shrubland);
        assert_ne!(at(700.0), Biome::Desert);
        // The same rain is steppe on a hot summer-rain land and enough in a
        // cool, even climate (P_th 14 + 2 T).
        let cool = rules.choose(BiomeInput {
            temperature: 8.0,
            moisture: moisture_index(400.0),
            elevation: -0.12,
            seasonality: 0.0,
            volcanic_zone: 0.0,
            coast_distance_km: 1000.0,
            lake: false,
            river: false,
            highland_offset: 0.0,
        });
        assert!(!matches!(cool, Biome::Desert | Biome::Shrubland));
    }

    #[test]
    fn cones_rise_out_of_the_lowland() {
        let rules = crate::preset::Preset::load("terran").unwrap().rules;
        let forest = |volcanic_zone: f32| {
            rules.choose(BiomeInput {
                elevation: -0.12,
                moisture: 0.5,
                temperature: 8.0,
                seasonality: 0.0,
                volcanic_zone,
                coast_distance_km: 1000.0,
                lake: false,
                river: false,
                highland_offset: 0.0,
            })
        };
        // The background of a volcanic province leaves the forest alone…
        assert_eq!(forest(0.12), Biome::Forest);
        // …its cones turn it into lava flows around a vent.
        assert_eq!(forest(rules.vent_zone + 0.05), Biome::LavaField);
        assert_eq!(forest(0.95), Biome::Volcano);
    }

    use crate::preset::Preset;

    /// Every biome a planet preset can produce, over a coarse grid of inputs.
    fn reachable(preset: &Preset) -> std::collections::BTreeSet<Biome> {
        let steps = |lo: f32, hi: f32| (0..=20).map(move |i| lo + (hi - lo) * i as f32 / 20.0);
        let mut out = std::collections::BTreeSet::new();
        for e in steps(-1.0, 1.0) {
            for m in steps(-1.0, 1.0) {
                for t in steps(-40.0, 40.0) {
                    for vz in steps(0.0, 1.0) {
                        for (lake, river) in [(false, false), (true, false), (false, true)] {
                            let input = BiomeInput {
                                elevation: e,
                                moisture: m,
                                temperature: t,
                                seasonality: 0.0,
                                volcanic_zone: vz,
                                coast_distance_km: 0.0,
                                lake,
                                river,
                                highland_offset: 0.0,
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
            elevation: -0.13,
            moisture: 0.0,
            temperature: 20.0,
            seasonality: 0.0,
            volcanic_zone: 0.0,
            coast_distance_km,
            lake: false,
            river: false,
            highland_offset: 0.0,
        };
        assert_eq!(rules.choose(lowland(10.0)), Biome::Beach);
        assert_ne!(rules.choose(lowland(500.0)), Biome::Beach);
    }

    #[test]
    fn polar_seas_freeze_on_terran() {
        let input = BiomeInput {
            elevation: -0.5,
            moisture: 0.0,
            temperature: -15.0,
            seasonality: 0.0,
            volcanic_zone: 0.0,
            coast_distance_km: 0.0,
            lake: false,
            river: false,
            highland_offset: 0.0,
        };
        assert_eq!(BiomeRules::default().choose(input), Biome::SeaIce);
    }

    #[test]
    fn the_seasons_decide_ice_and_the_tropics() {
        let rules = BiomeRules::default();
        let land = |temperature: f32, seasonality: f32| {
            rules.choose(BiomeInput {
                elevation: -0.12,
                moisture: 0.5,
                temperature,
                seasonality,
                volcanic_zone: 0.0,
                coast_distance_km: 1000.0,
                lake: false,
                river: false,
                highland_offset: 0.0,
            })
        };
        // Ice needs a summer below freezing: a polar coast at −12 °C with
        // a summer at +8 °C is tundra (too cold a summer for trees), a
        // steady −12 °C is ice; a summer at +15 °C grows a taiga.
        assert_eq!(land(-12.0, 20.0), Biome::Tundra);
        assert_eq!(land(-5.0, 20.0), Biome::Taiga);
        assert_eq!(land(-12.0, 0.0), Biome::IceCap);
        assert_eq!(land(-25.0, 20.0), Biome::IceCap);
        // A warm annual mean with freezing winters (the pole of a world
        // tilted by 90°) is not tropical.
        assert_eq!(land(25.0, 2.0), Biome::Jungle);
        assert_eq!(land(25.0, 50.0), Biome::Forest);
        // Rivers freeze only where the summer does.
        let river = |seasonality| BiomeInput {
            river: true,
            temperature: -5.0,
            seasonality,
            elevation: -0.12,
            moisture: 0.5,
            volcanic_zone: 0.0,
            coast_distance_km: 1000.0,
            lake: false,
            highland_offset: 0.0,
        };
        assert!(rules.frozen(river(2.0)));
        assert!(!rules.frozen(river(10.0)));
        assert_eq!(rules.choose(river(10.0)), Biome::River);
    }

    #[test]
    fn deep_water_is_ocean_on_terran() {
        let input = BiomeInput {
            elevation: -0.9,
            moisture: 0.0,
            temperature: 20.0,
            seasonality: 0.0,
            volcanic_zone: 0.0,
            coast_distance_km: 0.0,
            lake: false,
            river: false,
            highland_offset: 0.0,
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
            (
                "frozen",
                &[Biome::FrozenOcean, Biome::GlacialPlain, Biome::FrozenLake],
            ),
            (
                "caustic",
                &[
                    Biome::CausticLake,
                    Biome::ToxicSwamp,
                    Biome::AcidFlatland,
                    Biome::AcidSea,
                ],
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
