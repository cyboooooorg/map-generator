//! The look of a world, until #424: everything that renders a planet, read
//! from TOML.
//!
//! A look sets the climate shift of the planet, overrides any biome
//! selection threshold, remaps biomes to planet-specific ones and sets its
//! sky, geology and, for a giant, its cloud bands. Looks are internal:
//! `planet::interim` picks one for the model of a planet, the moon draw
//! gives one to each moon, and a request never names one. The built-in
//! looks are embedded from `looks/*.toml` and `looks/moons/*.toml`.
//!
//! ```toml
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
//! [rules]                            # any BiomeRules field
//! polar = -5.0                       # °C
//!
//! [remap]                            # target = [biomes replaced by it]
//! DustPlain = ["Plain", "Desert"]
//! ```
pub use crate::archetype::ParamDefault;
use crate::atmosphere::Atmosphere;
use crate::biome::{Biome, BiomeRules};
use crate::geology::Geology;
use crate::moons::Composition;
use crate::request;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// The built-in looks of planets, by name.
const BUILT_IN: [(&str, &str); 8] = [
    ("terran", include_str!("../../../looks/terran.toml")),
    ("volcanic", include_str!("../../../looks/volcanic.toml")),
    ("frozen", include_str!("../../../looks/frozen.toml")),
    ("caustic", include_str!("../../../looks/caustic.toml")),
    ("barren", include_str!("../../../looks/barren.toml")),
    ("gas-giant", include_str!("../../../looks/gas-giant.toml")),
    ("ice-giant", include_str!("../../../looks/ice-giant.toml")),
    (
        "hot-jupiter",
        include_str!("../../../looks/hot-jupiter.toml"),
    ),
];

/// The looks of moons, given by the generator to the moons of a world.
const MOON_LOOKS: [(&str, &str); 2] = [
    (
        "rocky-moon",
        include_str!("../../../looks/moons/rocky-moon.toml"),
    ),
    (
        "icy-moon",
        include_str!("../../../looks/moons/icy-moon.toml"),
    ),
];

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

/// How a world is drawn until #424: biome rules and remap, climate shift,
/// sky, geology and giant look. Chosen by `planet::interim` (or by the moon
/// draw), never named by a request.
///
/// Deserialising validates the look and builds its biome lookup, whether it
/// comes from a look file or from a world's `world.json`.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Look {
    #[serde(default)]
    pub climate: ClimateShift,
    #[serde(default)]
    pub rules: BiomeRules,
    /// Sky and clouds; Earth's when the look has no `[atmosphere]` table.
    #[serde(default)]
    pub atmosphere: Atmosphere,
    /// Tectonic regime and wet era; Earth's when the look has no
    /// `[geology]` table.
    #[serde(default)]
    pub geology: Geology,
    /// A giant planet: cloud bands instead of a surface (see
    /// `crate::giant_world`); `None` for a rocky or icy world.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub giant: Option<GiantSpec>,
    /// Target biome → biomes replaced by it.
    #[serde(default)]
    pub remap: BTreeMap<Biome, Vec<Biome>>,
    /// `remap` inverted: the final biome for each biome id.
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    lookup: Vec<Biome>,
}

/// A look as written, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LookSpec {
    #[serde(default)]
    climate: ClimateShift,
    #[serde(default)]
    rules: BiomeRules,
    #[serde(default)]
    atmosphere: Atmosphere,
    #[serde(default)]
    geology: Geology,
    #[serde(default)]
    giant: Option<GiantSpec>,
    #[serde(default)]
    remap: BTreeMap<Biome, Vec<Biome>>,
}

/// The `[giant]` table of a giant planet's look.
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

impl<'de> Deserialize<'de> for Look {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let spec = LookSpec::deserialize(deserializer)?;
        Look::try_from(spec).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<LookSpec> for Look {
    type Error = String;

    fn try_from(spec: LookSpec) -> Result<Self, String> {
        spec.atmosphere.validate()?;
        spec.geology.validate()?;
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
        Ok(Look {
            climate: spec.climate,
            rules: spec.rules,
            atmosphere: spec.atmosphere,
            geology: spec.geology,
            giant: spec.giant,
            remap: spec.remap,
            lookup,
        })
    }
}

/// Why a look could not be loaded.
#[derive(Debug)]
pub struct LookError(String);

impl fmt::Display for LookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LookError {}

impl Look {
    /// Parses and validates a look. `origin` names the source in errors.
    pub fn from_toml(text: &str, origin: &str) -> Result<Self, LookError> {
        toml::from_str(text).map_err(|e| LookError(format!("{origin}: {e}")))
    }

    /// Loads a built-in look by name.
    ///
    /// # Panics
    ///
    /// When no built-in look has that name: looks are chosen by the
    /// generator, never by a request.
    pub fn load(name: &str) -> Self {
        let (_, text) = BUILT_IN
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("unknown look {name:?}"));
        Look::from_toml(text, &format!("built-in look {name}")).expect("built-in looks are valid")
    }

    /// The look of a moon of `composition` (see `crate::moons`).
    pub fn moon(composition: Composition) -> Self {
        let name = Look::moon_name(composition);
        let (_, text) = MOON_LOOKS
            .iter()
            .find(|(n, _)| *n == name)
            .expect("both moon looks are built in");
        Look::from_toml(text, &format!("moon look {name}")).expect("moon looks are valid")
    }

    /// The class key of a moon of `composition`: `rocky-moon` or
    /// `icy-moon`.
    pub fn moon_name(composition: Composition) -> &'static str {
        match composition {
            Composition::Rock => "rocky-moon",
            Composition::Ice => "icy-moon",
        }
    }

    /// Every built-in look of a planet, by name.
    pub fn built_in() -> Vec<(&'static str, Look)> {
        BUILT_IN
            .iter()
            .map(|(name, _)| (*name, Look::load(name)))
            .collect()
    }

    /// The final biome after this look's remapping.
    pub fn remap(&self, biome: Biome) -> Biome {
        self.lookup[biome.id() as usize]
    }

    /// This look with a second remap, `then` (target, sources), applied to
    /// the output of its own: a biome its remap turns into a source of
    /// `then` ends as that source's target. `remap` is rewritten to the
    /// composition, so that the look reads back from JSON the same.
    pub fn then_remap(mut self, then: &[(Biome, &[Biome])]) -> Self {
        let second = |b: Biome| {
            then.iter()
                .find(|(_, sources)| sources.contains(&b))
                .map_or(b, |&(target, _)| target)
        };
        self.lookup = self.lookup.iter().map(|&b| second(b)).collect();
        let mut remap: BTreeMap<Biome, Vec<Biome>> = BTreeMap::new();
        for b in Biome::ALL {
            let target = self.remap(b);
            if target != b {
                remap.entry(target).or_default().push(b);
            }
        }
        self.remap = remap;
        self
    }
}

/// Names of the built-in looks of planets.
pub fn built_in_names() -> Vec<&'static str> {
    BUILT_IN.iter().map(|(name, _)| *name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_looks_load() {
        let looks = Look::built_in();
        assert_eq!(looks.len(), 8);
        assert!(looks[..5].iter().all(|(_, l)| l.giant.is_none()));
        assert!(looks[5..].iter().all(|(_, l)| l.giant.is_some()));
        for composition in [Composition::Rock, Composition::Ice] {
            assert!(Look::moon(composition).giant.is_none());
        }
    }

    #[test]
    fn terran_remaps_nothing() {
        let terran = Look::load("terran");
        for b in Biome::ALL {
            assert_eq!(terran.remap(b), b);
        }
    }

    #[test]
    fn remap_and_overrides_apply() {
        let p = Look::from_toml(
            r#"
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
    fn a_look_read_back_from_json_remaps() {
        let volcanic = Look::load("volcanic");
        let json = serde_json::to_string(&volcanic).unwrap();
        let back: Look = serde_json::from_str(&json).unwrap();
        assert_eq!(back.remap(Biome::Ocean), Biome::MagmaSea);
        assert_eq!(back.remap(Biome::Mountain), Biome::Mountain);
    }

    #[test]
    fn invalid_looks_are_rejected() {
        let cases = [
            ("[remap]\nOcean = [\"Plain\"]\nDesert = [\"Plain\"]", "both"),
            ("[remap]\nOcean = [\"Lava\"]", "unknown variant"),
            ("[rules]\npolarr = 0.1", "unknown field"),
            ("name = \"x\"", "unknown field"),
            ("[defaults]\npressure = 1.0", "unknown field"),
            ("[geology]\nwet_until = -1.0", "within"),
            ("[geology]\nregime = \"floating\"", "unknown variant"),
        ];
        for (text, expected) in cases {
            let err = Look::from_toml(text, "test").unwrap_err().to_string();
            assert!(err.contains(expected), "{text:?} gave {err:?}");
        }
    }

    #[test]
    fn looks_without_geology_get_earths() {
        let p = Look::from_toml("", "test").unwrap();
        assert_eq!(p.geology, Geology::default());
        let barren = Look::load("barren");
        assert_eq!(barren.geology.regime, crate::geology::Regime::Stagnant);
    }

    #[test]
    fn defaults_pick_within_range() {
        let mut rng = rand::rng();
        assert_eq!(ParamDefault::Fixed(0.4).pick(&mut rng), 0.4);
        let v = ParamDefault::Range([0.1, 0.2]).pick(&mut rng);
        assert!((0.1..0.2).contains(&v));
    }

    #[test]
    #[should_panic(expected = "unknown look")]
    fn an_unknown_look_panics() {
        Look::load("terran-ish");
    }
}
