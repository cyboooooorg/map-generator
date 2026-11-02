//! Planet presets: everything that makes a planet type, read from TOML.
//!
//! A preset sets the climate shift of the planet, overrides any biome
//! selection threshold, remaps biomes to planet-specific ones, and can give
//! default values or random ranges for the generation parameters. The five
//! built-in planet types are presets embedded from `presets/*.toml`; any other
//! TOML file following the same format can be passed to `--planet`.
//!
//! ```toml
//! name = "mars-like"                 # lowercase letters, digits and dashes
//! description = "Cold desert world"
//!
//! [defaults]                         # a fixed value or a [min, max] range
//! sea_level = [0.1, 0.3]             # or land_fraction = 0.29, not both
//! axial_tilt = 25.2
//!
//! [climate]                          # added to every pixel before biome selection
//! temperature = -0.35
//! moisture = -0.6
//! volcanic = 0.0
//!
//! [rules]                            # any BiomeRules field
//! polar = 0.2
//!
//! [remap]                            # target = [biomes replaced by it]
//! DustPlain = ["Plain", "Desert"]
//! ```
use crate::biome::{Biome, BiomeRules};
use crate::request;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// The built-in presets, in the order `--planet` lists them.
const BUILT_IN: [(&str, &str); 5] = [
    ("terran", include_str!("../../../presets/terran.toml")),
    ("volcanic", include_str!("../../../presets/volcanic.toml")),
    ("frozen", include_str!("../../../presets/frozen.toml")),
    ("caustic", include_str!("../../../presets/caustic.toml")),
    ("barren", include_str!("../../../presets/barren.toml")),
];

/// A generation parameter given by a preset: a fixed value or a range to draw from.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ParamDefault {
    Fixed(f32),
    Range([f32; 2]),
}

impl ParamDefault {
    /// The fixed value, or a random draw in the range.
    pub fn pick(self, rng: &mut impl RngExt) -> f32 {
        match self {
            ParamDefault::Fixed(v) => v,
            ParamDefault::Range([lo, hi]) if lo < hi => rng.random_range(lo..hi),
            ParamDefault::Range([lo, _]) => lo,
        }
    }

    fn check(self, field: &str, min: f32, max: f32) -> Result<(), String> {
        let (lo, hi) = match self {
            ParamDefault::Fixed(v) => (v, v),
            ParamDefault::Range([lo, hi]) => (lo, hi),
        };
        if lo > hi {
            return Err(format!("defaults.{field}: range [{lo}, {hi}] is reversed"));
        }
        if lo < min || hi > max {
            return Err(format!("defaults.{field}: must stay within [{min}, {max}]"));
        }
        Ok(())
    }
}

/// Default values of the generation parameters. Command-line options win;
/// parameters missing here fall back to the generator's own defaults.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Defaults {
    pub sea_level: Option<ParamDefault>,
    /// Share of the surface that is land; the sea level is computed to
    /// match it. Exclusive with `sea_level`.
    pub land_fraction: Option<ParamDefault>,
    pub volcanic: Option<ParamDefault>,
    pub circumference: Option<ParamDefault>,
    pub axial_tilt: Option<ParamDefault>,
    pub erosion: Option<ParamDefault>,
}

/// Global shifts added to every pixel before biome selection.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ClimateShift {
    /// Added to temperature (`[0, 1]` scale).
    pub temperature: f32,
    /// Added to moisture (`[-1, 1]` scale).
    pub moisture: f32,
    /// Added to the volcanic zone (`[0, 1]` scale).
    pub volcanic: f32,
}

/// A planet type. See the module documentation for the file format.
///
/// Deserialising validates the preset and builds its biome lookup, whether
/// it comes from a preset file or from a world's `world.json`.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Preset {
    /// Short identifier, used in output directory names.
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub climate: ClimateShift,
    #[serde(default)]
    pub rules: BiomeRules,
    /// Target biome → biomes replaced by it.
    #[serde(default)]
    pub remap: BTreeMap<Biome, Vec<Biome>>,
    /// `remap` inverted: the final biome for each biome id.
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    lookup: Vec<Biome>,
}

/// A preset as written, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetSpec {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    defaults: Defaults,
    #[serde(default)]
    climate: ClimateShift,
    #[serde(default)]
    rules: BiomeRules,
    #[serde(default)]
    remap: BTreeMap<Biome, Vec<Biome>>,
}

impl<'de> Deserialize<'de> for Preset {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let spec = PresetSpec::deserialize(deserializer)?;
        Preset::try_from(spec).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<PresetSpec> for Preset {
    type Error = String;

    fn try_from(spec: PresetSpec) -> Result<Self, String> {
        let valid_name = !spec.name.is_empty()
            && spec
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid_name {
            return Err(format!(
                "name {:?} must use lowercase letters, digits and dashes only",
                spec.name
            ));
        }

        let d = &spec.defaults;
        if d.sea_level.is_some() && d.land_fraction.is_some() {
            return Err("defaults: set sea_level or land_fraction, not both".to_string());
        }
        for (field, value, limits) in [
            ("sea_level", d.sea_level, request::SEA_LEVEL),
            ("land_fraction", d.land_fraction, request::LAND_FRACTION),
            ("volcanic", d.volcanic, request::VOLCANIC),
            ("circumference", d.circumference, request::CIRCUMFERENCE_KM),
            ("axial_tilt", d.axial_tilt, request::AXIAL_TILT_DEG),
            ("erosion", d.erosion, request::EROSION),
        ] {
            if let Some(v) = value {
                v.check(field, limits.min, limits.max)?;
            }
        }

        let mut lookup = Biome::ALL.to_vec();
        let mut seen = BTreeMap::new();
        for (&target, sources) in &spec.remap {
            for &source in sources {
                if let Some(previous) = seen.insert(source, target) {
                    return Err(format!(
                        "remap: {source:?} is mapped to both {previous:?} and {target:?}"
                    ));
                }
                lookup[source.id() as usize] = target;
            }
        }
        Ok(Preset {
            name: spec.name,
            description: spec.description,
            defaults: spec.defaults,
            climate: spec.climate,
            rules: spec.rules,
            remap: spec.remap,
            lookup,
        })
    }
}

/// Why a preset could not be loaded.
#[derive(Debug)]
pub struct PresetError(String);

impl fmt::Display for PresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PresetError {}

impl Preset {
    /// Parses and validates a preset. `origin` names the source in errors.
    pub fn from_toml(text: &str, origin: &str) -> Result<Self, PresetError> {
        toml::from_str(text).map_err(|e| PresetError(format!("{origin}: {e}")))
    }

    /// Loads a built-in preset by name, or a preset file when `spec` ends
    /// with `.toml`.
    pub fn load(spec: &str) -> Result<Self, PresetError> {
        if spec.ends_with(".toml") {
            let text = std::fs::read_to_string(Path::new(spec))
                .map_err(|e| PresetError(format!("{spec}: {e}")))?;
            return Preset::from_toml(&text, spec);
        }
        BUILT_IN
            .iter()
            .find(|(name, _)| *name == spec)
            .map(|(name, text)| Preset::from_toml(text, &format!("built-in preset {name}")))
            .unwrap_or_else(|| {
                Err(PresetError(format!(
                    "unknown planet {spec:?}: use one of {} or a .toml preset file",
                    built_in_names().join(", ")
                )))
            })
    }

    /// Every built-in preset.
    pub fn built_in() -> Vec<Preset> {
        BUILT_IN
            .iter()
            .map(|(name, _)| Preset::load(name).expect("built-in presets are valid"))
            .collect()
    }

    /// The final biome after this planet's remapping.
    pub fn remap(&self, biome: Biome) -> Biome {
        self.lookup[biome.id() as usize]
    }
}

/// Names of the built-in presets.
pub fn built_in_names() -> Vec<&'static str> {
    BUILT_IN.iter().map(|(name, _)| *name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_presets_load() {
        let presets = Preset::built_in();
        assert_eq!(presets.len(), 5);
        for (preset, name) in presets.iter().zip(built_in_names()) {
            assert_eq!(preset.name, name);
        }
    }

    #[test]
    fn terran_remaps_nothing() {
        let terran = Preset::load("terran").unwrap();
        for b in Biome::ALL {
            assert_eq!(terran.remap(b), b);
        }
    }

    #[test]
    fn remap_and_overrides_apply() {
        let p = Preset::from_toml(
            r#"
            name = "test-world"
            [rules]
            polar = 0.3
            [remap]
            DustPlain = ["Plain", "Desert"]
            "#,
            "test",
        )
        .unwrap();
        assert_eq!(p.remap(Biome::Plain), Biome::DustPlain);
        assert_eq!(p.remap(Biome::Forest), Biome::Forest);
        assert_eq!(p.rules.polar, 0.3);
        // Fields that are not overridden keep their default.
        assert_eq!(p.rules.waterline, BiomeRules::default().waterline);
    }

    #[test]
    fn a_preset_read_back_from_json_remaps() {
        let volcanic = Preset::load("volcanic").unwrap();
        let json = serde_json::to_string(&volcanic).unwrap();
        let back: Preset = serde_json::from_str(&json).unwrap();
        assert_eq!(back.remap(Biome::Ocean), Biome::MagmaSea);
        assert_eq!(back.remap(Biome::Mountain), Biome::Mountain);
    }

    #[test]
    fn invalid_presets_are_rejected() {
        let cases = [
            ("name = \"Bad Name\"", "lowercase"),
            (
                "name = \"x\"\n[remap]\nOcean = [\"Plain\"]\nDesert = [\"Plain\"]",
                "both",
            ),
            (
                "name = \"x\"\n[remap]\nOcean = [\"Lava\"]",
                "unknown variant",
            ),
            ("name = \"x\"\n[rules]\npolarr = 0.1", "unknown field"),
            ("name = \"x\"\n[defaults]\nsea_level = [0.5, 2.0]", "within"),
            (
                "name = \"x\"\n[defaults]\nsea_level = 0.2\nland_fraction = 0.3",
                "not both",
            ),
            ("name = \"x\"\n[defaults]\nland_fraction = 1.5", "within"),
        ];
        for (text, expected) in cases {
            let err = Preset::from_toml(text, "test").unwrap_err().to_string();
            assert!(err.contains(expected), "{text:?} gave {err:?}");
        }
    }

    #[test]
    fn defaults_pick_within_range() {
        let mut rng = rand::rng();
        assert_eq!(ParamDefault::Fixed(0.4).pick(&mut rng), 0.4);
        let v = ParamDefault::Range([0.1, 0.2]).pick(&mut rng);
        assert!((0.1..0.2).contains(&v));
    }
}
