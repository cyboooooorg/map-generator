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
//! [locales.fr]                       # optional: name and description per language
//! name = "Martienne"
//! description = "Désert froid"
//!
//! [defaults]                         # a fixed value or a [min, max] range
//! sea_level = [0.1, 0.3]             # or land_fraction = 0.29, not both
//! axial_tilt = 25.2
//!
//! surface_age = [3.0, 4.2]           # Ga
//!
//! [climate]                          # added to every pixel before biome selection
//! temperature = -15.0                # °C, after the star
//! internal_heat = 0.0                # K: the planet's own heat
//! moisture = -0.6
//! volcanic = 0.0
//!
//! [geology]                          # see the geology module
//! regime = "stagnant"
//! wet_until = 3.5                    # Ga
//!
//! [star]                             # see the star module
//! type = "G2V"                       # left out: drawn like the stars nearby
//! orbit_au = 1.524                   # or position = [0.1, 0.9] in the habitable zone
//!
//! [rules]                            # any BiomeRules field
//! polar = -5.0                       # °C
//!
//! [remap]                            # target = [biomes replaced by it]
//! DustPlain = ["Plain", "Desert"]
//! ```
use crate::atmosphere::Atmosphere;
use crate::biome::{Biome, BiomeRules};
use crate::geology::Geology;
use crate::request;
use crate::star::StarSpec;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// The built-in presets, in the order `--planet` lists them: the rocky
/// worlds first (the first [`DRAWN`] are drawn when no planet is asked
/// for), then the giants.
const BUILT_IN: [(&str, &str); 8] = [
    ("terran", include_str!("../../../presets/terran.toml")),
    ("volcanic", include_str!("../../../presets/volcanic.toml")),
    ("frozen", include_str!("../../../presets/frozen.toml")),
    ("caustic", include_str!("../../../presets/caustic.toml")),
    ("barren", include_str!("../../../presets/barren.toml")),
    ("gas-giant", include_str!("../../../presets/gas-giant.toml")),
    ("ice-giant", include_str!("../../../presets/ice-giant.toml")),
    (
        "hot-jupiter",
        include_str!("../../../presets/hot-jupiter.toml"),
    ),
];

/// How many of the built-in presets a request without a planet draws
/// from: the rocky ones (a giant is asked for by name).
pub const DRAWN: usize = 5;

/// The presets of moons: not planet types (neither listed nor drawn for
/// a planet), given by the generator to the moons of a world.
const MOON_PRESETS: [(&str, &str); 2] = [
    (
        "rocky-moon",
        include_str!("../../../presets/moons/rocky-moon.toml"),
    ),
    (
        "icy-moon",
        include_str!("../../../presets/moons/icy-moon.toml"),
    ),
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

    pub(crate) fn check(self, field: &str, min: f32, max: f32) -> Result<(), String> {
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
    /// Surface pressure in bar (0 = airless).
    pub pressure: Option<ParamDefault>,
    /// Age of the oldest surviving surface, in Ga.
    pub surface_age: Option<ParamDefault>,
}

/// Global shifts added to every pixel before biome selection.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ClimateShift {
    /// Added to temperature, °C.
    pub temperature: f32,
    /// Added to moisture (`[-1, 1]` scale).
    pub moisture: f32,
    /// Added to the volcanic zone (`[0, 1]` scale).
    pub volcanic: f32,
    /// Heat from within the planet, as the temperature (K) it would keep
    /// the surface at without any starlight: combined with the star's
    /// heating as radiated power (`T⁴` adds up), so a world far from its
    /// star stays warm. 0: none (Earth's is negligible).
    pub internal_heat: f32,
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
    /// Sky and clouds; Earth's when the preset has no `[atmosphere]` table.
    #[serde(default)]
    pub atmosphere: Atmosphere,
    /// Tectonic regime and wet era; Earth's when the preset has no
    /// `[geology]` table.
    #[serde(default)]
    pub geology: Geology,
    /// The host star and the planet's orbit: drawn when left out (see
    /// `crate::star`).
    #[serde(default)]
    pub star: StarSpec,
    /// A giant planet: cloud bands instead of a surface (see
    /// `crate::giant_world`); `None` for a rocky or icy world.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub giant: Option<GiantSpec>,
    /// Target biome → biomes replaced by it.
    #[serde(default)]
    pub remap: BTreeMap<Biome, Vec<Biome>>,
    /// Translated name and description, by language code (`fr`). Only the
    /// preset files carry them: `world.json` does not, so a world does not
    /// change when a translation does.
    #[serde(skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    pub locales: BTreeMap<String, PresetLocale>,
    /// `remap` inverted: the final biome for each biome id.
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    lookup: Vec<Biome>,
}

/// The name and description of a preset in one language. Either may be
/// left out: the English one is shown instead.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
pub struct PresetLocale {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// A preset as written, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetSpec {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    locales: BTreeMap<String, PresetLocale>,
    #[serde(default)]
    defaults: Defaults,
    #[serde(default)]
    climate: ClimateShift,
    #[serde(default)]
    rules: BiomeRules,
    #[serde(default)]
    atmosphere: Atmosphere,
    #[serde(default)]
    geology: Geology,
    #[serde(default)]
    star: StarSpec,
    #[serde(default)]
    giant: Option<GiantSpec>,
    #[serde(default)]
    remap: BTreeMap<Biome, Vec<Biome>>,
}

/// The `[giant]` table of a giant planet's preset.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GiantSpec {
    /// Gas (Jupiter, Saturn) or ice (Uranus, Neptune).
    pub kind: crate::giants::GiantKind,
    /// Mass in Earth masses, a value or a `[min, max]` range (drawn
    /// log-uniformly would favour none: drawn uniformly in it).
    pub mass: ParamDefault,
    /// Orbital period in days, a value or a range: places a hot giant
    /// close to its star instead of the `[star]` position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_days: Option<ParamDefault>,
    /// Odds of bright rings (Saturn's) and of faint ones; the rest have
    /// none.
    #[serde(default)]
    pub rings: [f32; 2],
}

impl GiantSpec {
    fn validate(&self) -> Result<(), String> {
        let (lo, hi) = (request::GIANT_MASS.min, request::GIANT_MASS.max);
        self.mass
            .check("mass", lo, hi)
            .map_err(|e| e.replacen("defaults.", "giant.", 1))?;
        if let Some(period) = self.period_days {
            period
                .check("period_days", 0.2, 1000.0)
                .map_err(|e| e.replacen("defaults.", "giant.", 1))?;
        }
        if self.rings.iter().any(|r| !(0.0..=1.0).contains(r))
            || self.rings[0] + self.rings[1] > 1.0
        {
            return Err("giant.rings: two odds in [0, 1] adding up to 1 at most".into());
        }
        Ok(())
    }
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
            ("pressure", d.pressure, request::PRESSURE_BAR),
            ("surface_age", d.surface_age, request::SURFACE_AGE_GA),
        ] {
            if let Some(v) = value {
                v.check(field, limits.min, limits.max)?;
            }
        }

        for language in spec.locales.keys() {
            let valid = (2..=3).contains(&language.len())
                && language.chars().all(|c| c.is_ascii_lowercase());
            if !valid {
                return Err(format!(
                    "locales.{language}: a language code is two or three lowercase letters (fr, de)"
                ));
            }
        }
        spec.atmosphere.validate()?;
        spec.geology.validate()?;
        spec.star.validate()?;
        if let Some(giant) = &spec.giant {
            giant.validate()?;
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
            atmosphere: spec.atmosphere,
            geology: spec.geology,
            star: spec.star,
            giant: spec.giant,
            remap: spec.remap,
            locales: spec.locales,
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
    /// Converts a preset stored by a world of format version 3 or older,
    /// whose temperatures were on the former `[0, 1]` scale, to °C: its
    /// thresholds through the same curve as the temperatures (so every
    /// biome is kept), its climate shift by the curve's mean slope.
    pub fn to_celsius(&mut self) {
        use crate::climate::{DEGREES_PER_UNIT, celsius};
        let r = &mut self.rules;
        for t in [
            &mut r.sea_ice_temperature,
            &mut r.highland_snow_temperature,
            &mut r.polar,
            &mut r.boreal,
            &mut r.temperate,
        ] {
            *t = celsius(*t);
        }
        self.climate.temperature *= DEGREES_PER_UNIT;
    }

    /// Parses and validates a preset. `origin` names the source in errors.
    pub fn from_toml(text: &str, origin: &str) -> Result<Self, PresetError> {
        let mut preset: Self =
            toml::from_str(text).map_err(|e| PresetError(format!("{origin}: {e}")))?;
        // A preset file written for the former moisture scale: the world
        // files keep theirs (see `BiomeRules::is_legacy`), a preset does not.
        if preset.rules.is_legacy() {
            eprintln!(
                "warning: {origin}: shrubland_moisture and desert_moisture are ignored; \
                 arid land follows Koppen's criterion B (steppe_ratio, desert_ratio), and \
                 the moisture thresholds are now on the log scale of the annual rain"
            );
            preset.rules.shrubland_moisture = None;
            preset.rules.desert_moisture = None;
        }
        // A lone legacy field means nothing alone.
        preset.rules.shrubland_moisture = None;
        preset.rules.desert_moisture = None;
        Ok(preset)
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

    /// The preset of a moon, `rocky-moon` or `icy-moon` (see
    /// `crate::moons`).
    pub fn moon(name: &str) -> Result<Self, PresetError> {
        MOON_PRESETS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(n, text)| Preset::from_toml(text, &format!("moon preset {n}")))
            .unwrap_or_else(|| Err(PresetError(format!("unknown moon preset {name:?}"))))
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
        assert_eq!(presets.len(), 8);
        assert!(presets[..DRAWN].iter().all(|p| p.giant.is_none()));
        assert!(presets[DRAWN..].iter().all(|p| p.giant.is_some()));
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
    fn old_presets_convert_to_the_celsius_thresholds() {
        // The former [0, 1] defaults land on the °C defaults of format
        // version 4 to 10, when every threshold was an annual mean (ice
        // and highland snow now compare with the summer, read without
        // seasons on those worlds).
        let mut old = Preset::load("terran").unwrap();
        let r = &mut old.rules;
        (r.sea_ice_temperature, r.highland_snow_temperature) = (0.07, 0.35);
        (r.polar, r.boreal, r.temperate) = (0.10, 0.30, 0.55);
        old.climate.temperature = -0.6;
        old.to_celsius();
        let new = crate::biome::BiomeRules::default();
        for (a, b) in [
            (old.rules.sea_ice_temperature, new.sea_ice_temperature),
            (old.rules.highland_snow_temperature, 2.6),
            (old.rules.polar, -11.6),
            (old.rules.boreal, new.boreal),
            (old.rules.temperate, new.temperate),
        ] {
            assert!((a - b).abs() < 0.1, "{a} vs {b}");
        }
        assert!((old.climate.temperature + 27.0).abs() < 1e-4);
    }

    #[test]
    fn locales_are_read_but_not_written_to_worlds() {
        let p = Preset::from_toml(
            r#"
            name = "dune"
            description = "Sand world"
            [locales.fr]
            name = "Dune"
            description = "Monde de sable"
            [locales.de]
            description = "Sandwelt"
            "#,
            "test",
        )
        .unwrap();
        assert_eq!(p.locales["fr"].name.as_deref(), Some("Dune"));
        assert_eq!(p.locales["de"].name, None);
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("locales"), "{json}");
        let back: Preset = serde_json::from_str(&json).unwrap();
        assert!(back.locales.is_empty());
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
            ("name = \"x\"\n[locales.FR]\nname = \"X\"", "language code"),
            ("name = \"x\"\n[locales.fr]\ntitle = \"X\"", "unknown field"),
            ("name = \"x\"\n[defaults]\nsurface_age = 5.0", "within"),
            ("name = \"x\"\n[geology]\nwet_until = -1.0", "within"),
            (
                "name = \"x\"\n[geology]\nregime = \"floating\"",
                "unknown variant",
            ),
        ];
        for (text, expected) in cases {
            let err = Preset::from_toml(text, "test").unwrap_err().to_string();
            assert!(err.contains(expected), "{text:?} gave {err:?}");
        }
    }

    #[test]
    fn presets_without_geology_get_earths() {
        let p = Preset::from_toml("name = \"x\"", "test").unwrap();
        assert_eq!(p.geology, Geology::default());
        let barren = Preset::load("barren").unwrap();
        assert_eq!(barren.geology.regime, crate::geology::Regime::Stagnant);
    }

    #[test]
    fn defaults_pick_within_range() {
        let mut rng = rand::rng();
        assert_eq!(ParamDefault::Fixed(0.4).pick(&mut rng), 0.4);
        let v = ParamDefault::Range([0.1, 0.2]).pick(&mut rng);
        assert!((0.1..0.2).contains(&v));
    }
}
