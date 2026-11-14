//! Archetypes: named sets of inputs of the physical planet model.
//!
//! An archetype is not a type of planet. It names a mass, a composition and
//! an orbit, each a fixed value or a `[min, max]` range; what the planet
//! becomes (its class, its air, its oceans) is derived from them by
//! [`crate::planet`]. An input an archetype leaves out is drawn from the
//! formation context, like that of a planet nobody asked for. The built-in
//! archetypes are embedded from `archetypes/*.toml`; any TOML file with the
//! same format can be given to `--planet`.
//!
//! ```toml
//! name = "mars-like"                  # lowercase letters and dashes
//! description = "Mars-like: a small cold desert"
//!
//! [locales.fr]                        # optional: name and description per language
//! name = "Martienne"
//! description = "Un petit désert froid"
//!
//! [inputs]                            # a fixed value or a [min, max] range
//! mass_earth = [0.096, 0.118]         # required, Earth masses (a moon's: 1e-6 to 0.1)
//! core_mass_fraction = 0.24           # iron share of the refractory mass
//! water_oceans = [0.002, 0.008]       # Earth oceans (1.4e21 kg each)
//! envelope_fraction = 0.0             # hydrogen and helium share of the mass
//! co2 = 1.0                           # factor on the formation rule, 1 typical
//! n2 = 1.0                            # idem
//! radiogenic = 1.0                    # Earth's abundance = 1
//! life = true                         # true: arises where it can; false: never
//!
//! [orbit]
//! star = "G2V"                        # spectral type, used when asked alone
//! insolation = [0.39, 0.47]           # Earth's = 1, or period_days, not both
//! eccentricity = 0.093
//! axial_tilt = 25.19                  # degrees
//! ```
//!
//! A giant's moon has a `[moon]` table instead of `[orbit]`: its host's
//! archetype places its host, and a whole system asking for the moon
//! puts it on a giant of that archetype (see `crate::system`). It is not
//! asked for as a world on its own.
//!
//! ```toml
//! [moon]
//! host = "saturn-like"                # a built-in giant archetype
//! origin = "disk"                     # grown in its disk, or "captured"
//! semi_major_axis_radii = [18.0, 22.0]  # in its host's radii
//! eccentricity = [0.02, 0.03]
//! ice_fraction = [0.40, 0.50]         # water and ices, share of its mass
//! ```
//!
//! The Solar System archetypes take the masses, core shares and orbits of
//! [`crate::planet::reference::SOLAR_BODIES`] (sources in its module
//! docs) with ranges of about ±10 % on mass and insolation, the
//! eccentricities and tilts of NASA's planetary fact sheets
//! (nssdc.gsfc.nasa.gov/planetary/factsheet; tilts read as seasons).
//! `mars-like` departs from Mars's reference values on two inputs, so
//! that it renders as Mars does: its water, 0.002 to 0.008 oceans, about
//! the 0.004 of its polar deposits and ground ice (Carr & Head 2015, see
//! `planet::classify::ICE_WORLD_MIN_OCEANS`), stays under the 0.01 that
//! shows as ice; its CO₂ and N₂ factors, 0.3 (Calibration, no source:
//! Mars lost most of its early air, which the formation rule does not
//! model), bring its air to about the fact sheet's 6 mbar at the Sun's
//! age on the median of the seeds; single worlds spread from about 0.002
//! to 0.02 bar with the mass (escape takes more from a lighter one) and
//! the star's age (a young one has lost less).
//! The exoplanet kinds (`super-earth`, `mini-neptune`, `ocean-world`,
//! `lava-world`, `hot-jupiter`) name a range typical of the kind:
//! Calibration, no source. The four small ones keep near-circular orbits
//! (`eccentricity = [0, 0.05]`), as the tides keep close-in small planets
//! (Van Eylen et al. 2019, doi:10.3847/1538-3881/aaf22f, fit a mean of
//! 0.05 to the small planets of multi-transiting systems), rather than
//! the formation draw, which can carry an ocean world to a runaway at
//! periastron. `lava-world` and
//! `mini-neptune` asked alone sit round a K dwarf (`K0V`, `K2V`), as the
//! known lava worlds do (55 Cnc e, CoRoT-7 b and K2-141 b orbit K stars),
//! so that a drawn M dwarf does not pull their orbit inside the closest
//! one a request allows. `ocean-world` asked alone sits round a `G2V`:
//! Earth's light (its 0.9 to 1.1) lies inside the Sun's habitable zone,
//! whose inner edge is at 1.107 (Kopparapu et al. 2014, see
//! `crate::star::habitable_fluxes`), but past that of a cooler star (1.01
//! for a K2V, about 0.9 for a red M dwarf), where the ocean runs away.
//! In a system they take the system's star.
//!
//! The moon archetypes (`titan-like`, `europa-like`, `ganymede-like`,
//! `callisto-like`, `io-like`, `triton-like`, `enceladus-like`) take the
//! masses of the NASA satellite fact sheets and JPL's satellite physical
//! parameters with ranges of about ±10 %, the orbits and eccentricities
//! of JPL's mean orbital elements (semi-major axes over their giant's
//! equatorial radius), and the ice shares their densities and moments of
//! inertia give (sources in each file). They set causes only: a Titan is
//! a hydrocarbon world because its mass, ice and cold host make one, and
//! comes out as one only when the model says so.
use crate::planet::InputOverrides;
use crate::request::Limits;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// The built-in archetypes, in the order they are listed.
const BUILT_IN: [(&str, &str); 20] = [
    (
        "mercury-like",
        include_str!("../../../archetypes/mercury-like.toml"),
    ),
    (
        "venus-like",
        include_str!("../../../archetypes/venus-like.toml"),
    ),
    (
        "earth-like",
        include_str!("../../../archetypes/earth-like.toml"),
    ),
    (
        "mars-like",
        include_str!("../../../archetypes/mars-like.toml"),
    ),
    (
        "jupiter-like",
        include_str!("../../../archetypes/jupiter-like.toml"),
    ),
    (
        "saturn-like",
        include_str!("../../../archetypes/saturn-like.toml"),
    ),
    (
        "uranus-like",
        include_str!("../../../archetypes/uranus-like.toml"),
    ),
    (
        "neptune-like",
        include_str!("../../../archetypes/neptune-like.toml"),
    ),
    (
        "super-earth",
        include_str!("../../../archetypes/super-earth.toml"),
    ),
    (
        "mini-neptune",
        include_str!("../../../archetypes/mini-neptune.toml"),
    ),
    (
        "ocean-world",
        include_str!("../../../archetypes/ocean-world.toml"),
    ),
    (
        "lava-world",
        include_str!("../../../archetypes/lava-world.toml"),
    ),
    (
        "hot-jupiter",
        include_str!("../../../archetypes/hot-jupiter.toml"),
    ),
    (
        "titan-like",
        include_str!("../../../archetypes/titan-like.toml"),
    ),
    (
        "europa-like",
        include_str!("../../../archetypes/europa-like.toml"),
    ),
    (
        "ganymede-like",
        include_str!("../../../archetypes/ganymede-like.toml"),
    ),
    (
        "callisto-like",
        include_str!("../../../archetypes/callisto-like.toml"),
    ),
    ("io-like", include_str!("../../../archetypes/io-like.toml")),
    (
        "triton-like",
        include_str!("../../../archetypes/triton-like.toml"),
    ),
    (
        "enceladus-like",
        include_str!("../../../archetypes/enceladus-like.toml"),
    ),
];

/// Mass of a planet, Earth masses: from the Moon's tenth to 13 Jupiter
/// masses, the deuterium-burning limit above which a body is a brown dwarf
/// (Spiegel, Burrows & Milsom 2011, doi:10.1088/0004-637X/727/1/57).
pub const MASS_EARTH: Limits = Limits {
    min: 0.01,
    max: 4131.0,
};
/// Mass of a moon asked for by archetype, Earth masses: from a moonlet of
/// about 70 km (6 × 10¹⁸ kg) to a tenth of Earth's, past Ganymede's 0.025
/// (NASA). Calibration, no source.
pub const MOON_MASS_EARTH: Limits = Limits {
    min: 1.0e-6,
    max: 0.1,
};
/// Semi-major axis of a moon asked for, in its giant's radii: from the
/// giant's cloud tops to beyond the farthest stable prograde orbit of a
/// Saturn at Saturn's orbit (a third of its Hill radius, about 360
/// radii). A host that cannot keep it there refuses it. Calibration, no
/// source.
pub const MOON_SEMI_MAJOR_AXIS_RADII: Limits = Limits {
    min: 1.0,
    max: 400.0,
};
/// Share of a moon's mass in water and ices. Calibration, no source: from
/// dry Io to a body of ice with a little rock.
pub const MOON_ICE_FRACTION: Limits = Limits { min: 0.0, max: 0.9 };
/// Iron share of the refractory mass. Calibration, no source: from no core
/// to Mercury's 0.70 and beyond.
pub const CORE_MASS_FRACTION: Limits = Limits { min: 0.0, max: 0.9 };
/// Water and ices, Earth oceans. Calibration, no source: none to a
/// planet's worth of water.
pub const WATER_OCEANS: Limits = Limits {
    min: 0.0,
    max: 1.0e6,
};
/// Hydrogen and helium share of the mass. Calibration, no source: a
/// Jupiter holds 0.93 to 0.96; the formation draw reaches `1 − 5 / 4131`
/// (0.9988) for the heaviest giant with the lightest core, which a pinned
/// request must be able to ask for again.
pub const ENVELOPE_FRACTION: Limits = Limits {
    min: 0.0,
    max: 0.999,
};
/// Factor on the CO₂ or N₂ formation rule. Calibration, no source: two
/// orders of magnitude either way is more than the draw scatters.
pub const VOLATILE_FACTOR: Limits = Limits {
    min: 0.0,
    max: 100.0,
};
/// Radiogenic abundance, Earth's = 1. Calibration, no source: a decade
/// either way.
pub const RADIOGENIC: Limits = Limits {
    min: 0.1,
    max: 10.0,
};
/// Light received over the year, Earth's = 1. Calibration, no source:
/// from far beyond Neptune (0.001) to a lava world's inferno.
pub const INSOLATION: Limits = Limits {
    min: 1e-5,
    max: 1e5,
};
/// Orbital period, days. Calibration, no source: from an ultra-short
/// period planet to a few hundred years.
pub const PERIOD_DAYS: Limits = Limits { min: 0.1, max: 1e5 };
/// Eccentricity of the orbit. Calibration, no source: a bound orbit that
/// keeps its planet out of the star.
pub const ECCENTRICITY: Limits = Limits { min: 0.0, max: 0.9 };
/// Axial tilt, degrees. Calibration, no source: beyond 90° the spin runs
/// backwards, which this tilt does not express.
pub const AXIAL_TILT_DEG: Limits = Limits {
    min: 0.0,
    max: 90.0,
};

/// A parameter given by a preset or an archetype: a fixed value or a
/// range to draw from.
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

/// The name and description of a preset or an archetype in one language.
/// Either may be left out: the English one is shown instead.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
pub struct Locale {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// The inputs of the planet model an archetype sets (the `[inputs]`
/// table). Those left out are drawn from the formation context.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ArchetypeInputs {
    /// Mass, Earth masses.
    pub mass_earth: ParamDefault,
    /// Iron share of the refractory mass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_mass_fraction: Option<ParamDefault>,
    /// Water and ices, Earth oceans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_oceans: Option<ParamDefault>,
    /// Hydrogen and helium share of the mass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope_fraction: Option<ParamDefault>,
    /// Factor on the CO₂ formation rule (1 typical).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub co2: Option<ParamDefault>,
    /// Factor on the N₂ formation rule (1 typical).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n2: Option<ParamDefault>,
    /// Radiogenic abundance, Earth's = 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiogenic: Option<ParamDefault>,
    /// `true`: life arises wherever the world allows it; `false`: never.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life: Option<bool>,
}

/// Where an archetype sits (the `[orbit]` table): by its light or by its
/// period, not both.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ArchetypeOrbit {
    /// Spectral type of the star, used when the archetype is asked alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub star: Option<String>,
    /// Light received, Earth's = 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insolation: Option<ParamDefault>,
    /// Orbital period in days, instead of the insolation (a hot Jupiter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_days: Option<ParamDefault>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eccentricity: Option<ParamDefault>,
    /// Degrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub axial_tilt: Option<ParamDefault>,
}

impl ArchetypeOrbit {
    /// Semi-major axis of the orbit, AU, round a star of `luminosity` (L☉)
    /// and `star_mass` (M☉): `a = sqrt(L / S)` for the insolation `S`
    /// (Earth's = 1), or Kepler's third law `a = (M P²)^⅓` (`P` in years)
    /// for the period; each range drawn from `rng`. `None` when the
    /// archetype sets neither.
    pub fn orbit_au(&self, luminosity: f64, star_mass: f64, rng: &mut impl RngExt) -> Option<f64> {
        match (self.insolation, self.period_days) {
            (Some(s), _) => Some((luminosity / f64::from(s.pick(rng))).sqrt()),
            (None, Some(p)) => {
                let years = f64::from(p.pick(rng)) / 365.25;
                Some((star_mass * years * years).cbrt())
            }
            (None, None) => None,
        }
    }
}

impl ArchetypeOrbit {
    /// The nearest and farthest orbits the archetype's range gives, AU,
    /// round a star of `luminosity` (L☉) and `star_mass` (M☉): the same
    /// laws as [`orbit_au`](Self::orbit_au) at the two ends of the range.
    /// `None` when the archetype sets neither insolation nor period.
    pub fn orbit_range_au(&self, luminosity: f64, star_mass: f64) -> Option<(f64, f64)> {
        let ends = |p: ParamDefault| match p {
            ParamDefault::Fixed(v) => (f64::from(v), f64::from(v)),
            ParamDefault::Range([lo, hi]) => (f64::from(lo), f64::from(hi.max(lo))),
        };
        match (self.insolation, self.period_days) {
            (Some(s), _) => {
                // Less light, farther out.
                let (lo, hi) = ends(s);
                Some(((luminosity / hi).sqrt(), (luminosity / lo).sqrt()))
            }
            (None, Some(p)) => {
                let (lo, hi) = ends(p);
                let at = |days: f64| (star_mass * (days / 365.25).powi(2)).cbrt();
                Some((at(lo), at(hi)))
            }
            (None, None) => None,
        }
    }
}

/// How a moon asked for by archetype came to its giant.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ArchetypeMoonOrigin {
    /// Grown in its giant's disk, as the Galilean moons and Titan.
    Disk,
    /// A passing body its giant captured, as Triton: only an ice giant
    /// captures one today.
    Captured,
}

/// What makes an archetype a giant's moon (the `[moon]` table): its host
/// and its orbit round it. The moon's `[inputs]` mass is its own; it has
/// no `[orbit]`, its host's archetype placing the host.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ArchetypeMoon {
    /// The built-in giant archetype it orbits (`saturn-like`).
    pub host: String,
    /// Semi-major axis, in its host's radii.
    pub semi_major_axis_radii: ParamDefault,
    pub eccentricity: ParamDefault,
    /// Share of its mass in water and ices.
    pub ice_fraction: ParamDefault,
    pub origin: ArchetypeMoonOrigin,
}

/// A named set of model inputs. See the module documentation for the file
/// format.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Archetype {
    /// Short identifier: lowercase letters and dashes.
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub inputs: ArchetypeInputs,
    #[serde(default)]
    pub orbit: ArchetypeOrbit,
    /// Set on a giant's moon: its host and its orbit round it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub moon: Option<ArchetypeMoon>,
    /// Translated name and description, by language code (`fr`).
    #[serde(default, skip_serializing)]
    #[cfg_attr(feature = "ts", ts(skip))]
    pub locales: BTreeMap<String, Locale>,
}

/// Why an archetype could not be loaded.
#[derive(Debug)]
pub struct ArchetypeError(String);

impl fmt::Display for ArchetypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ArchetypeError {}

impl Archetype {
    /// Parses and validates an archetype. `origin` names the source in
    /// errors.
    pub fn from_toml(text: &str, origin: &str) -> Result<Self, ArchetypeError> {
        let archetype: Self =
            toml::from_str(text).map_err(|e| ArchetypeError(format!("{origin}: {e}")))?;
        archetype
            .validate()
            .map_err(|e| ArchetypeError(format!("{origin}: {e}")))?;
        Ok(archetype)
    }

    fn validate(&self) -> Result<(), String> {
        let valid_name = !self.name.is_empty()
            && self
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '-');
        if !valid_name {
            return Err(format!(
                "name {:?} must use lowercase letters and dashes only",
                self.name
            ));
        }
        let i = &self.inputs;
        let o = &self.orbit;
        if o.insolation.is_some() && o.period_days.is_some() {
            return Err("orbit: set insolation or period_days, not both".to_string());
        }
        if let Some(star) = &o.star {
            crate::star::parse_type(star).map_err(|e| format!("orbit.star: {e}"))?;
        }
        if let Some(moon) = &self.moon {
            self.validate_moon(moon)?;
        }
        let mass_limits = if self.moon.is_some() {
            MOON_MASS_EARTH
        } else {
            MASS_EARTH
        };
        let checks = [
            ("inputs", "mass_earth", Some(i.mass_earth), mass_limits),
            (
                "inputs",
                "core_mass_fraction",
                i.core_mass_fraction,
                CORE_MASS_FRACTION,
            ),
            ("inputs", "water_oceans", i.water_oceans, WATER_OCEANS),
            (
                "inputs",
                "envelope_fraction",
                i.envelope_fraction,
                ENVELOPE_FRACTION,
            ),
            ("inputs", "co2", i.co2, VOLATILE_FACTOR),
            ("inputs", "n2", i.n2, VOLATILE_FACTOR),
            ("inputs", "radiogenic", i.radiogenic, RADIOGENIC),
            ("orbit", "insolation", o.insolation, INSOLATION),
            ("orbit", "period_days", o.period_days, PERIOD_DAYS),
            ("orbit", "eccentricity", o.eccentricity, ECCENTRICITY),
            ("orbit", "axial_tilt", o.axial_tilt, AXIAL_TILT_DEG),
        ];
        for (table, field, value, limits) in checks {
            if let Some(v) = value {
                v.check(field, limits.min, limits.max)
                    .map_err(|e| e.replacen("defaults.", &format!("{table}."), 1))?;
            }
        }
        Ok(())
    }

    /// Checks the `[moon]` table: no `[orbit]` beside it, a host that is a
    /// built-in giant archetype (not a moon), and its ranges within their
    /// limits.
    fn validate_moon(&self, moon: &ArchetypeMoon) -> Result<(), String> {
        if self.orbit != ArchetypeOrbit::default() {
            return Err("orbit: a moon has none, its host's archetype places its host".to_string());
        }
        let not_a_giant = || {
            format!(
                "moon.host: {:?} is not a built-in giant archetype",
                moon.host
            )
        };
        if moon.host == self.name || !built_in_names().contains(&moon.host.as_str()) {
            return Err(not_a_giant());
        }
        let host = Archetype::load(&moon.host).map_err(|e| e.to_string())?;
        let lightest = match host.inputs.mass_earth {
            ParamDefault::Fixed(v) | ParamDefault::Range([v, _]) => f64::from(v),
        };
        if host.moon.is_some() || lightest < crate::planet::GIANT_MIN_MASS_EARTH {
            return Err(not_a_giant());
        }
        let checks = [
            (
                "semi_major_axis_radii",
                moon.semi_major_axis_radii,
                MOON_SEMI_MAJOR_AXIS_RADII,
            ),
            ("eccentricity", moon.eccentricity, ECCENTRICITY),
            ("ice_fraction", moon.ice_fraction, MOON_ICE_FRACTION),
        ];
        for (field, value, limits) in checks {
            value
                .check(field, limits.min, limits.max)
                .map_err(|e| e.replacen("defaults.", "moon.", 1))?;
        }
        Ok(())
    }

    /// Loads a built-in archetype by name, or an archetype file when `spec`
    /// ends with `.toml`.
    pub fn load(spec: &str) -> Result<Self, ArchetypeError> {
        if spec.ends_with(".toml") {
            let text = std::fs::read_to_string(Path::new(spec))
                .map_err(|e| ArchetypeError(format!("{spec}: {e}")))?;
            return Archetype::from_toml(&text, spec);
        }
        BUILT_IN
            .iter()
            .find(|(name, _)| *name == spec)
            .map(|(name, text)| Archetype::from_toml(text, &format!("built-in archetype {name}")))
            .unwrap_or_else(|| {
                Err(ArchetypeError(format!(
                    "unknown planet {spec:?}: use one of {} or a .toml archetype file",
                    built_in_names().join(", ")
                )))
            })
    }

    /// Every built-in archetype.
    pub fn built_in() -> Vec<Archetype> {
        BUILT_IN
            .iter()
            .map(|(name, _)| Archetype::load(name).expect("built-in archetypes are valid"))
            .collect()
    }

    /// The overrides this archetype sets, its ranges drawn from `rng`
    /// (one draw per range, in the order of the fields of `[inputs]`
    /// after the mass).
    pub fn overrides(&self, rng: &mut impl RngExt) -> InputOverrides {
        let i = &self.inputs;
        let mut pick = |p: Option<ParamDefault>| p.map(|p| f64::from(p.pick(rng)));
        InputOverrides {
            core_mass_fraction: pick(i.core_mass_fraction),
            water_oceans: pick(i.water_oceans),
            envelope_fraction: pick(i.envelope_fraction),
            co2: pick(i.co2),
            n2: pick(i.n2),
            radiogenic: pick(i.radiogenic),
            life: i.life,
        }
    }

    /// The mass drawn in the archetype's range, Earth masses.
    pub fn mass(&self, rng: &mut impl RngExt) -> f64 {
        f64::from(self.inputs.mass_earth.pick(rng))
    }
}

/// Names of the built-in archetypes.
pub fn built_in_names() -> Vec<&'static str> {
    BUILT_IN.iter().map(|(name, _)| *name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::{Context, PlanetModel, derive_model_with};
    use crate::star::{SUN, StarInfo};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const AGE_GYR: f32 = 4.6;

    /// The model of `a` at its own insolation (or period) round its own
    /// star (a G2V when it names none) of 4.6 Gyr and metallicity 0, drawn
    /// from `seed`. What the archetype leaves out is drawn by the
    /// formation context.
    fn model_of(a: &Archetype, seed: u32) -> PlanetModel {
        let mut rng = StdRng::seed_from_u64(u64::from(seed));
        let host = a.orbit.star.as_deref().map_or(SUN, |s| {
            crate::star::parse_type(s).expect("a valid spectral type")
        });
        let v = crate::star::values(host);
        let mass_earth = a.mass(&mut rng);
        let orbit_au = a
            .orbit
            .orbit_au(f64::from(v.luminosity), f64::from(v.mass), &mut rng)
            .unwrap_or_else(|| panic!("{} has no orbit", a.name)) as f32;
        let eccentricity = a.orbit.eccentricity.map_or(0.0, |e| e.pick(&mut rng));
        let star = StarInfo::new(host, orbit_au, eccentricity, AGE_GYR);
        let ice_line_au = 2.7 * star.luminosity.sqrt();
        let context = Context::of(&star, 0.0, ice_line_au);
        let overrides = a.overrides(&mut rng);
        derive_model_with(seed, "planet", &context, mass_earth, &overrides)
    }

    /// The model of the moon archetype `a` drawn from `seed`: its host's
    /// archetype at the middle of its own orbit round its own star (as
    /// [`model_of`]), the moon at the middle of its orbit round the
    /// middle of the host's mass, through [`crate::planet::derive_moon_model`].
    fn moon_model_of(a: &Archetype, seed: u32) -> PlanetModel {
        use crate::planet::{HostTide, MoonBirth, MoonFormation, derive_moon_model};
        let moon = a.moon.as_ref().expect("a moon archetype");
        let host = Archetype::load(&moon.host).unwrap();
        let mid = |p: ParamDefault| match p {
            ParamDefault::Fixed(v) => f64::from(v),
            ParamDefault::Range([lo, hi]) => f64::from(lo + hi) / 2.0,
        };
        let star = crate::star::parse_type(host.orbit.star.as_deref().unwrap()).unwrap();
        let v = crate::star::values(star);
        let (near, far) = host
            .orbit
            .orbit_range_au(f64::from(v.luminosity), f64::from(v.mass))
            .unwrap();
        let orbit_au = (near * far).sqrt() as f32;
        let info = StarInfo::new(
            star,
            orbit_au,
            mid(host.orbit.eccentricity.unwrap()) as f32,
            AGE_GYR,
        );
        let context = Context::of(&info, 0.0, 2.7 * info.luminosity.sqrt());
        let host_earth = mid(host.inputs.mass_earth);
        let radius_km = f64::from(crate::giants::giant_radius(
            host_earth as f32,
            info.equilibrium_k,
        )) * 6371.0;
        let mut rng = StdRng::seed_from_u64(u64::from(seed));
        let formation = MoonFormation {
            mass_earth: a.mass(&mut rng),
            ice_fraction: f64::from(moon.ice_fraction.pick(&mut rng)),
            birth: match moon.origin {
                ArchetypeMoonOrigin::Disk => MoonBirth::Disk,
                ArchetypeMoonOrigin::Captured => MoonBirth::Captured,
            },
            tide: HostTide {
                host_kg: host_earth * crate::exomoon::EARTH_KG,
                a_m: f64::from(moon.semi_major_axis_radii.pick(&mut rng)) * radius_km * 1000.0,
                eccentricity: f64::from(moon.eccentricity.pick(&mut rng)),
            },
        };
        let overrides = a.overrides(&mut rng);
        derive_moon_model(seed, "p1-1", &context, &formation, &overrides)
    }

    /// The class key each archetype must come out as.
    fn expected(name: &str) -> &'static str {
        match name {
            "mercury-like" | "earth-like" | "mars-like" => "rocky",
            "venus-like" => "runaway-greenhouse",
            "jupiter-like" | "saturn-like" => "gas-giant",
            "uranus-like" | "neptune-like" => "ice-giant",
            "super-earth" => "super-earth",
            "mini-neptune" => "mini-neptune",
            "ocean-world" => "ocean-world",
            "lava-world" => "lava-world",
            "hot-jupiter" => "hot-jupiter",
            // The moons, round their host at its own orbit (see
            // `moon_model_of`): Titan's liquid methane, the others' ice
            // over an ocean or none, Io's dry rock. Titan's class is
            // checked on most seeds, see `titan_like_is_mostly_a_hydrocarbon_world`.
            "titan-like" => "hydrocarbon-world",
            "europa-like" | "ganymede-like" | "callisto-like" | "triton-like"
            | "enceladus-like" => "ice-world",
            "io-like" => "rocky",
            other => panic!("no expected class for {other}"),
        }
    }

    #[test]
    fn every_archetype_comes_out_as_its_class() {
        for a in Archetype::built_in() {
            if a.name == "titan-like" {
                continue;
            }
            for seed in 0..20 {
                let model = if a.moon.is_some() {
                    moon_model_of(&a, seed)
                } else {
                    model_of(&a, seed)
                };
                assert_eq!(
                    model.class.key(),
                    expected(&a.name),
                    "{} seed {seed}: {model:?}",
                    a.name
                );
            }
        }
    }

    #[test]
    fn built_in_names_load() {
        for n in built_in_names() {
            Archetype::load(n).unwrap();
        }
        assert_eq!(built_in_names().len(), 20);
    }

    /// Every Titan that keeps methane in its air keeps liquid
    /// hydrocarbons, its nitrogen holding them liquid below pure methane's
    /// triple point (`planet::climate::methane_freezes_k`), and is a
    /// hydrocarbon world when its air passes the 1 bar from which
    /// `planet::classify` names one. Over seeds 0 to 19, 17 are; seeds 5
    /// and 10 keep their lakes under 0.61 and 0.57 bar and are classed ice
    /// worlds; seed 8, drawn with little methane (0.15 % of its mass), has
    /// lost it all to its star's Lyman α (`atmosphere::methane_photolysis`).
    #[test]
    fn titan_like_is_mostly_a_hydrocarbon_world() {
        let a = Archetype::load("titan-like").unwrap();
        let mut hydrocarbon = 0;
        for seed in 0..20 {
            let m = moon_model_of(&a, seed);
            let s = m.surface.expect("a moon has a surface");
            if s.gases.ch4 > 0.0 {
                assert_eq!(
                    s.climate.liquid,
                    crate::planet::SurfaceLiquid::Hydrocarbons,
                    "seed {seed}: {m:?}"
                );
            }
            if s.climate.liquid == crate::planet::SurfaceLiquid::Hydrocarbons
                && s.gases.total() > 1.0
            {
                assert_eq!(m.class.key(), expected(&a.name), "seed {seed}: {m:?}");
                hydrocarbon += 1;
            } else {
                assert_eq!(m.class.key(), "ice-world", "seed {seed}: {m:?}");
            }
        }
        assert_eq!(hydrocarbon, 17, "{hydrocarbon} of 20");
    }

    #[test]
    fn a_moon_archetype_names_a_giant_host() {
        let moon = |host: &str, extra: &str| {
            format!(
                "name = \"moonlet\"\n{extra}[inputs]\nmass_earth = 0.01\n[moon]\n\
                 host = \"{host}\"\nsemi_major_axis_radii = 10.0\neccentricity = 0.0\n\
                 ice_fraction = 0.4\norigin = \"disk\"\n"
            )
        };
        let a = Archetype::from_toml(&moon("saturn-like", ""), "test").unwrap();
        assert_eq!(a.moon.as_ref().unwrap().host, "saturn-like");
        for host in ["earth-like", "titan-like", "nowhere", "moonlet"] {
            let err = Archetype::from_toml(&moon(host, ""), "test")
                .unwrap_err()
                .to_string();
            assert!(err.contains("moon.host"), "{host}: {err}");
        }
        // A moon has no orbit of its own.
        let orbit = moon("saturn-like", "").replace("[moon]", "[orbit]\ninsolation = 0.01\n[moon]");
        assert!(Archetype::from_toml(&orbit, "test").is_err());
        // Its mass is a moon's: below a planet's least, within its own.
        let light = moon("saturn-like", "").replace("mass_earth = 0.01", "mass_earth = 2e-5");
        assert!(Archetype::from_toml(&light, "test").is_ok());
        let heavy = moon("saturn-like", "").replace("mass_earth = 0.01", "mass_earth = 0.5");
        let err = Archetype::from_toml(&heavy, "test")
            .unwrap_err()
            .to_string();
        assert!(err.contains("inputs.mass_earth"), "{err}");
        // Every built-in moon names its host.
        for a in Archetype::built_in().iter().filter(|a| a.moon.is_some()) {
            let host = Archetype::load(&a.moon.as_ref().unwrap().host).unwrap();
            assert!(host.moon.is_none(), "{}", a.name);
        }
    }

    #[test]
    fn a_file_with_both_insolation_and_period_is_refused() {
        let text = r#"
name = "both"
[inputs]
mass_earth = 1.0
[orbit]
insolation = 1.0
period_days = 10.0
"#;
        let err = Archetype::from_toml(text, "test").unwrap_err().to_string();
        assert!(err.contains("not both"), "{err}");
    }

    #[test]
    fn a_range_outside_the_limits_is_refused() {
        let text = r#"
name = "heavy"
[inputs]
mass_earth = 5000.0
"#;
        let err = Archetype::from_toml(text, "test").unwrap_err().to_string();
        assert!(err.contains("inputs.mass_earth"), "{err}");
    }

    #[test]
    fn a_bad_name_or_unknown_field_is_refused() {
        let bad_name = "name = \"Bad Name\"\n[inputs]\nmass_earth = 1.0\n";
        assert!(Archetype::from_toml(bad_name, "test").is_err());
        let unknown = "name = \"x\"\n[inputs]\nmass_earth = 1.0\nsea_level = 0.3\n";
        assert!(Archetype::from_toml(unknown, "test").is_err());
    }

    #[test]
    fn overrides_draw_ranges_and_keep_fixed_values() {
        let a = Archetype::load("earth-like").unwrap();
        let o = a.overrides(&mut StdRng::seed_from_u64(1));
        assert_eq!(o.core_mass_fraction, Some(0.325_f32 as f64));
        assert!((0.8..1.2).contains(&o.water_oceans.unwrap()));
        assert_eq!(o.life, Some(true));
        assert_eq!(o.co2, Some(1.0));
        assert_eq!(o.radiogenic, None);
    }

    /// Known departure: Earth's air is not an input of the model. The
    /// pressure comes from the N2 rule (set here, `n2 = 1`) plus the
    /// nitrogen that ammonia photolysis leaves, and the ammonia scatter
    /// (`10^N(0, 0.3)` of the formation rule) cannot be set by an
    /// archetype: single seeds come out between 0.5 and 3.6 bar. The test
    /// checks the ocean and the life per seed, and the pressure on the
    /// median of the seeds.
    #[test]
    fn earth_like_keeps_its_ocean_and_its_life() {
        let a = Archetype::load("earth-like").unwrap();
        let mut bars = Vec::new();
        for seed in 0..20 {
            let m = model_of(&a, seed);
            let s = m.surface.expect("a bare world has a surface");
            let oceans = s.water.oceans;
            assert!((0.5..2.0).contains(&oceans), "seed {seed}: {oceans} oceans");
            assert!(s.life.arose, "seed {seed}: {:?}", s.life);
            bars.push(s.gases.total());
        }
        bars.sort_by(f64::total_cmp);
        let median = (bars[9] + bars[10]) / 2.0;
        assert!((0.7..1.3).contains(&median), "median {median} bar");
    }

    /// Mars keeps about 0.004 oceans of ice (Carr & Head 2015, see
    /// `planet::classify::ICE_WORLD_MIN_OCEANS`), too little to show: the
    /// barren look, not the frozen one; and 6 mbar of air, which the
    /// archetype's lowered CO₂ and N₂ factors give on the median of the
    /// seeds (single seeds spread with the mass, which sets the escape).
    #[test]
    fn mars_like_is_a_barren_desert_under_a_thin_air() {
        let a = Archetype::load("mars-like").unwrap();
        let mut bars = Vec::new();
        for seed in 0..20 {
            let m = model_of(&a, seed);
            assert_eq!(crate::planet::interim::base_of(&m), "barren", "seed {seed}");
            let s = m.surface.expect("a bare world has a surface");
            assert!(
                s.water.oceans < 0.01,
                "seed {seed}: {} oceans",
                s.water.oceans
            );
            bars.push(s.gases.total());
        }
        bars.sort_by(f64::total_cmp);
        let median = (bars[9] + bars[10]) / 2.0;
        assert!(
            (0.004..0.012).contains(&median),
            "median {median} bar: {bars:?}"
        );
        assert!(bars[0] > 0.001 && bars[19] < 0.03, "{bars:?}");
    }

    #[test]
    fn venus_like_has_venus_air() {
        let a = Archetype::load("venus-like").unwrap();
        for seed in 0..20 {
            let m = model_of(&a, seed);
            let gases = m.surface.expect("a bare world has a surface").gases;
            let bar = gases.total();
            assert!((60.0..120.0).contains(&bar), "seed {seed}: {bar} bar");
            let share = gases.co2 / bar;
            assert!(share > 0.9, "seed {seed}: CO2 {share}");
        }
    }
}
