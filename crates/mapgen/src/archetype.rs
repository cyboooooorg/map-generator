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
//! [reference]                         # optional: the body it stands for
//! name = "Mars"                       # a proper name, or theoretical = true
//! # discovered = 1995                 # exoplanets only: year of discovery
//!
//! [locales.fr]                        # optional: name and description per language
//! name = "Martienne"
//! description = "Un petit désert froid"
//! reference = "Mars"                  # the reference as French writes it ("la Terre")
//!
//! [inputs]                            # a fixed value or a [min, max] range
//! mass_earth = [0.096, 0.118]         # required, Earth masses (a moon's: 1e-6 to 0.1)
//! core_mass_fraction = 0.24           # iron share of the refractory mass
//! water_oceans = [0.002, 0.008]       # Earth oceans (1.4e21 kg each)
//! envelope_fraction = 0.0             # hydrogen and helium share of the mass
//! co2 = 1.0                           # factor on the formation rule, 1 typical
//! n2 = 1.0                            # idem
//! ch4 = 1.0                           # idem: the methane its ices carry
//! nh3 = 1.0                           # idem: the ammonia its ices carry
//! s = 1.0                             # idem: the sulphur its rock carries
//! radiogenic = 1.0                    # Earth's abundance = 1
//! age_gyr = 4.57                      # age of the system, Gyr; a typed age wins
//! life = true                         # true: arises where it can; false: never
//! snowball = false                    # true: frozen where it can; false: never
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
//! asked for as a world on its own. A system that names no star takes
//! the star its host's archetype names; a star named is honoured, and the
//! moon then comes out as the model says: an asked moon around a red
//! dwarf may lose its air.
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
//! (nssdc.gsfc.nasa.gov/planetary/factsheet; tilts read as seasons), and
//! the Solar System's age, 4.57 Gyr (Bouvier & Wadhwa 2010,
//! doi:10.1038/ngeo941). `earth-like`, `mars-like` and `titan-like` are
//! faithful: they land in their body's surface pressure on every seed
//! (Earth 0.7 to 1.3 bar, Mars 4 to 10 mbar, Titan 1.2 to 1.8 bar).
//! Each pins the fewest inputs that hold it, the measured values first,
//! then a volatile factor calibrated with the real body as its target
//! (the comments of each file give the sources):
//!
//! - `earth-like`: its mass, light and ocean fixed at 1, and `nh3 = 0.7`,
//!   the ammonia whose photolysis feeds its N₂;
//! - `mars-like`: its mass (0.1074) and light (0.431) fixed, whose
//!   escape sets its air, and `co2 = 0.4`; its water, 0.002 to 0.008
//!   oceans, about the 0.004 of its polar deposits and ground ice (Carr &
//!   Head 2015, see `planet::classify::ICE_WORLD_MIN_OCEANS`), stays
//!   under the 0.01 that shows as ice;
//! - `titan-like`: its mass fixed (0.0225), and `n2 = 0.82`, the nitrogen
//!   its ices carry, beside its methane factor.
//!
//! `venus-like` holds 60 to 120 bar without a pin.
//! The exoplanet kinds (`super-earth`, `mini-neptune`, `ocean-world`,
//! `lava-world`, `hot-jupiter`, `hot-neptune`) name a range typical of the kind:
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
//! `hot-neptune` keeps `envelope_fraction` from 0.08: below it
//! photoevaporation strips about one draw in thirteen down to a
//! `mini-neptune` (Calibration, no source).
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
use crate::exomoon::EARTH_KG;
use crate::planet::{Context, GIANT_MIN_MASS_EARTH, InputOverrides, OCEAN_KG, PlanetModel};
use crate::request::Limits;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// The built-in archetypes, in the order they are listed.
const BUILT_IN: [(&str, &str); 21] = [
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
        "hot-neptune",
        include_str!("../../../archetypes/hot-neptune.toml"),
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
/// Factor on the CO₂, N₂ or CH₄ formation rule. Calibration, no source: two
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

    /// The lowest and highest values (a fixed value is both).
    pub(crate) fn bounds(self) -> (f32, f32) {
        match self {
            ParamDefault::Fixed(v) => (v, v),
            ParamDefault::Range([lo, hi]) => (lo, hi),
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
    /// How the archetype's reference body reads in this language, with its
    /// article where the language needs one (`la Terre`).
    pub reference: Option<String>,
}

/// The body an archetype stands for (the `[reference]` table): the Solar
/// System body it takes its numbers from, the first exoplanet found of its
/// kind, or none because no such planet is known yet. The web app words a
/// world's "similar to" line from it.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
pub struct ArchetypeReference {
    /// Proper name, the same in every language unless a locale overrides it.
    pub name: Option<String>,
    /// Year an exoplanet was discovered; left out for a Solar System body.
    pub discovered: Option<u16>,
    /// No known planet is of this kind (instead of `name`).
    #[serde(default)]
    pub theoretical: bool,
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
    /// Factor on the CH₄ formation rule (1 typical).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ch4: Option<ParamDefault>,
    /// Factor on the NH₃ formation rule (1 typical).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nh3: Option<ParamDefault>,
    /// Factor on the sulphur formation rule (1 typical).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<ParamDefault>,
    /// Radiogenic abundance, Earth's = 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiogenic: Option<ParamDefault>,
    /// `true`: life arises wherever the world allows it; `false`: never.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life: Option<bool>,
    /// `true`: a snowball wherever the climate has a frozen state beside
    /// the temperate one; `false`: never (the temperate state).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snowball: Option<bool>,
    /// Age of the system the archetype is asked in, Gyr: the star's, when
    /// the request types none. Within `(0, star::lifetime_gyr(0.0)]`; a
    /// star that lives shorter refuses it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_gyr: Option<ParamDefault>,
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
    /// The moon it is in a 2:1 resonance with, when its archetype is
    /// defined by one (Enceladus and Dione).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub resonance: Option<ArchetypeResonance>,
}

/// The resonance partners of a moon archetype (`[moon.resonance]`):
/// regular moons of its host, one 2:1 step outward on the chain, one
/// step inward (Io inside Europa), or both, and a second step inward
/// (Io and Europa inside Ganymede).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ArchetypeResonance {
    /// The outer partner's mass over its host's mass, when the moon has
    /// one in 2:1 resonance outside it (none for Ganymede, the outermost
    /// of the Laplace chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub partner_mass_ratio: Option<ParamDefault>,
    /// The inner partner's mass over its host's mass, when the moon has
    /// one in 2:1 resonance inside it too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub inner_partner_mass_ratio: Option<ParamDefault>,
    /// The mass over its host's mass of the moon in 2:1 resonance inside
    /// its inner partner, two chain steps in (Io for Ganymede, the
    /// outermost of a three-moon chain); needs an inner partner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub second_inner_partner_mass_ratio: Option<ParamDefault>,
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
    /// The body it stands for, when its file says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reference: Option<ArchetypeReference>,
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
        if let Some(r) = &self.reference {
            match (&r.name, r.theoretical) {
                (Some(_), false) | (None, true) => {}
                _ => return Err("reference: set name or theoretical = true, not both".to_string()),
            }
            if r.discovered.is_some() && r.name.is_none() {
                return Err("reference: discovered needs a name".to_string());
            }
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
        if let Some(age) = i.age_gyr {
            let longest = crate::star::lifetime_gyr(0.0);
            let (lo, hi) = age.bounds();
            if lo > hi {
                return Err(format!("inputs.age_gyr: range [{lo}, {hi}] is reversed"));
            }
            if !(lo > 0.0 && hi <= longest) {
                return Err(format!(
                    "inputs.age_gyr: must stay within (0, {longest}] (the longest main-sequence life)"
                ));
            }
        }
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
            ("inputs", "ch4", i.ch4, VOLATILE_FACTOR),
            ("inputs", "nh3", i.nh3, VOLATILE_FACTOR),
            ("inputs", "s", i.s, VOLATILE_FACTOR),
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
        if let Some(resonance) = &moon.resonance {
            if resonance.partner_mass_ratio.is_none()
                && resonance.inner_partner_mass_ratio.is_none()
            {
                return Err("moon.resonance names no partner".to_string());
            }
            if let Some(outer) = resonance.partner_mass_ratio {
                outer
                    .check("resonance.partner_mass_ratio", f32::MIN_POSITIVE, 1e-3)
                    .map_err(|e| e.replacen("defaults.", "moon.", 1))?;
            }
            if let Some(second) = resonance.second_inner_partner_mass_ratio {
                if resonance.inner_partner_mass_ratio.is_none() {
                    return Err(
                        "moon.resonance.second_inner_partner_mass_ratio needs an inner partner"
                            .to_string(),
                    );
                }
                second
                    .check(
                        "resonance.second_inner_partner_mass_ratio",
                        f32::MIN_POSITIVE,
                        1e-3,
                    )
                    .map_err(|e| e.replacen("defaults.", "moon.", 1))?;
            }
            if let Some(inner) = resonance.inner_partner_mass_ratio {
                inner
                    .check(
                        "resonance.inner_partner_mass_ratio",
                        f32::MIN_POSITIVE,
                        1e-3,
                    )
                    .map_err(|e| e.replacen("defaults.", "moon.", 1))?;
            }
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
            ch4: pick(i.ch4),
            nh3: pick(i.nh3),
            s: pick(i.s),
            radiogenic: pick(i.radiogenic),
            life: i.life,
            snowball: i.snowball,
        }
    }

    /// The ages the archetype sets for its system, Gyr (`Fixed(x)` is
    /// `[x, x]`); `None` when it sets none.
    pub fn age_range(&self) -> Option<(f32, f32)> {
        self.inputs.age_gyr.map(ParamDefault::bounds)
    }

    /// The mass drawn in the archetype's range, Earth masses.
    pub fn mass(&self, rng: &mut impl RngExt) -> f64 {
        f64::from(self.inputs.mass_earth.pick(rng))
    }
}

/// The age the archetypes asked together set for their system, Gyr: the
/// intersection of their age ranges, a value drawn in it from the
/// [`ARCHETYPE_AGE`](crate::noise::stream::ARCHETYPE_AGE) stream of
/// `seed` (a fixed age draws nothing). `None` when none sets an age;
/// [`RequestError::ArchetypeAges`] when the ranges do not meet.
pub fn asked_age_gyr(
    archetypes: &[&Archetype],
    seed: u32,
) -> Result<Option<f32>, crate::request::RequestError> {
    use rand::SeedableRng;
    let aged: Vec<(&Archetype, (f32, f32))> = archetypes
        .iter()
        .filter_map(|a| a.age_range().map(|r| (*a, r)))
        .collect();
    if aged.is_empty() {
        return Ok(None);
    }
    let lo = aged.iter().map(|(_, r)| r.0).fold(f32::MIN, f32::max);
    let hi = aged.iter().map(|(_, r)| r.1).fold(f32::MAX, f32::min);
    if lo > hi {
        let mut names: Vec<String> = aged.iter().map(|(a, _)| a.name.clone()).collect();
        names.sort();
        names.dedup();
        return Err(crate::request::RequestError::ArchetypeAges { archetypes: names });
    }
    if lo == hi {
        return Ok(Some(lo));
    }
    let stream = crate::noise::stream_seed(seed, crate::noise::stream::ARCHETYPE_AGE);
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(stream));
    Ok(Some(rng.random_range(lo..hi)))
}

/// Names of the built-in archetypes.
pub fn built_in_names() -> Vec<&'static str> {
    BUILT_IN.iter().map(|(name, _)| *name).collect()
}

/// How far apart, in tolerances, a body and an archetype may be to
/// resemble each other (see [`similar`]). Calibrated by the tests
/// `solar_system_archetypes_are_recognised` and
/// `ordinary_drawn_bodies_are_mostly_unnamed`. Measured with
/// [`TOLERANCES`]: every built-in archetype names itself in 20 of 20
/// draws, and 56 of 300 bodies (19 %) of 50 drawn G2V systems get a name.
const SIMILAR_THRESHOLD: f64 = 1.0;

/// The tolerance of each axis of the distance of [`similar`]: `log10` of
/// the mass, the core mass fraction, the water mass fraction, `log10` of
/// the insolation (or of the period). Calibration, no source: tuned with
/// [`SIMILAR_THRESHOLD`] by the two tests named there (the ranges of the
/// archetypes count whole, so the tolerance only widens them).
const TOLERANCES: [f64; 4] = [0.15, 0.1, 0.1, 0.3];

/// The ends of a parameter's range.
fn ends(p: ParamDefault) -> (f64, f64) {
    match p {
        ParamDefault::Fixed(v) => (f64::from(v), f64::from(v)),
        ParamDefault::Range([lo, hi]) => (f64::from(lo), f64::from(hi.max(lo))),
    }
}

/// How far `x` is outside `[lo, hi]`: 0 inside.
fn gap(x: f64, (lo, hi): (f64, f64)) -> f64 {
    if x < lo {
        lo - x
    } else if x > hi {
        x - hi
    } else {
        0.0
    }
}

/// The name of the archetype of `archetypes` a body resembles, if one is
/// close enough: the distance between them in the archetypes' input space
/// (`log10` of the mass, the core mass fraction, the water mass fraction,
/// `log10` of the insolation, or of the period for an archetype that sets
/// one; each axis over its tolerance, the axes combined by their root
/// mean square) is at most a threshold, and the nearest archetype wins.
/// An archetype's range counts as the whole interval (no distance inside
/// it: a lava world's mass runs from 1 to 5 Earth masses), an input it
/// leaves out is skipped. A moon archetype has no orbit round a star: its
/// light is its host archetype's. Moon archetypes are for a giant's moon
/// only (`moon_of_giant`), planet archetypes for the others; a giant is
/// compared with giants by mass and insolation only. An archetype pinned
/// to a single value (earth-like, mars-like, titan-like) only matches
/// bodies close to that value, so drawn planets get its label less often.
pub fn similar<'a>(
    context: &Context,
    model: &PlanetModel,
    moon_of_giant: bool,
    archetypes: impl IntoIterator<Item = &'a Archetype>,
) -> Option<&'a str> {
    let all: Vec<&'a Archetype> = archetypes.into_iter().collect();
    let inputs = &model.inputs;
    let body_giant = inputs.mass_earth >= GIANT_MIN_MASS_EARTH;
    let log = |ends: (f64, f64)| (ends.0.log10(), ends.1.log10());
    let mut best: Option<(f64, &'a str)> = None;
    for &a in &all {
        if a.moon.is_some() != moon_of_giant {
            continue;
        }
        let (mass_lo, mass_hi) = ends(a.inputs.mass_earth);
        let giants = body_giant || mass_hi >= GIANT_MIN_MASS_EARTH;
        let mut axes = vec![(
            gap(inputs.mass_earth.log10(), log((mass_lo, mass_hi))),
            TOLERANCES[0],
        )];
        if !giants {
            if let Some(c) = a.inputs.core_mass_fraction {
                axes.push((gap(inputs.core_mass_fraction, ends(c)), TOLERANCES[1]));
            }
            let water = match (&a.moon, a.inputs.water_oceans) {
                (Some(m), _) => Some(ends(m.ice_fraction)),
                (None, Some(w)) => {
                    let (lo, hi) = ends(w);
                    Some((
                        lo * OCEAN_KG / (mass_hi * EARTH_KG),
                        hi * OCEAN_KG / (mass_lo * EARTH_KG),
                    ))
                }
                (None, None) => None,
            };
            if let Some(w) = water {
                axes.push((gap(inputs.water_fraction, w), TOLERANCES[2]));
            }
        }
        let host = a
            .moon
            .as_ref()
            .and_then(|m| all.iter().find(|h| h.name == m.host));
        let light = host.map_or(a.orbit.insolation, |h| h.orbit.insolation);
        match (light, a.orbit.period_days) {
            (Some(s), _) => {
                axes.push((gap(context.insolation.log10(), log(ends(s))), TOLERANCES[3]))
            }
            (None, Some(p)) => {
                let days = (context.orbit_au.powi(3) / context.star_mass).sqrt() * 365.25;
                axes.push((gap(days.log10(), log(ends(p))), TOLERANCES[3]));
            }
            (None, None) => {}
        }
        let distance =
            (axes.iter().map(|(d, t)| (d / t).powi(2)).sum::<f64>() / axes.len() as f64).sqrt();
        if distance <= SIMILAR_THRESHOLD && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, a.name.as_str()));
        }
    }
    best.map(|(_, name)| name)
}

#[cfg(test)]
mod tests {
    fn aged(name: &str, age: &str) -> Archetype {
        let text = format!("name = \"{name}\"\n[inputs]\nmass_earth = 1.0\nage_gyr = {age}\n");
        Archetype::from_toml(&text, "test").unwrap()
    }

    fn unaged(name: &str) -> Archetype {
        let text = format!("name = \"{name}\"\n[inputs]\nmass_earth = 1.0\n");
        Archetype::from_toml(&text, "test").unwrap()
    }

    #[test]
    fn no_age_asked_gives_none() {
        let (a, b) = (unaged("a"), unaged("b"));
        assert_eq!(asked_age_gyr(&[&a, &b], 7).unwrap(), None);
        assert_eq!(asked_age_gyr(&[], 7).unwrap(), None);
    }

    #[test]
    fn fixed_ages_agree() {
        let (a, b) = (aged("a", "4.57"), aged("b", "4.57"));
        assert_eq!(asked_age_gyr(&[&a, &b], 7).unwrap(), Some(4.57));
        // An archetype with no age does not disturb it.
        let c = unaged("c");
        assert_eq!(asked_age_gyr(&[&a, &c], 7).unwrap(), Some(4.57));
    }

    #[test]
    fn ranges_intersect() {
        let (a, b) = (aged("a", "[3.0, 5.0]"), aged("b", "[4.0, 6.0]"));
        for seed in 0..20 {
            let age = asked_age_gyr(&[&a, &b], seed).unwrap().unwrap();
            assert!((4.0..=5.0).contains(&age), "{age}");
            assert_eq!(asked_age_gyr(&[&a, &b], seed).unwrap(), Some(age));
        }
    }

    #[test]
    fn disjoint_ages_are_refused() {
        let (a, b) = (aged("mars-ish", "4.57"), aged("young", "[1.0, 2.0]"));
        match asked_age_gyr(&[&a, &b], 7) {
            Err(crate::request::RequestError::ArchetypeAges { archetypes }) => {
                assert_eq!(
                    archetypes,
                    vec!["mars-ish".to_string(), "young".to_string()]
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_age_out_of_the_stars_lives_is_refused_at_load() {
        for age in ["0.0", "1e5", "[2.0, 1.0]"] {
            let text = format!("name = \"x\"\n[inputs]\nmass_earth = 1.0\nage_gyr = {age}\n");
            let err = Archetype::from_toml(&text, "test").unwrap_err().to_string();
            assert!(err.contains("inputs.age_gyr"), "{age}: {err}");
        }
    }

    #[test]
    fn every_built_in_archetype_names_its_reference() {
        for a in Archetype::built_in() {
            let r = a
                .reference
                .unwrap_or_else(|| panic!("{} has no [reference]", a.name));
            assert!(r.name.is_some() != r.theoretical, "{}", a.name);
            // An exoplanet carries its year, a Solar System body does not.
            assert_eq!(
                r.discovered.is_some(),
                [
                    "hot-jupiter",
                    "hot-neptune",
                    "super-earth",
                    "mini-neptune",
                    "lava-world"
                ]
                .contains(&a.name.as_str()),
                "{}",
                a.name
            );
        }
        let earth = Archetype::load("earth-like").unwrap();
        assert_eq!(earth.locales["fr"].reference.as_deref(), Some("la Terre"));
    }

    #[test]
    fn a_reference_names_a_body_or_is_theoretical_not_both() {
        let file = |reference: &str| {
            Archetype::from_toml(
                &format!("name = \"x-like\"\n{reference}\n[inputs]\nmass_earth = 1.0\n"),
                "test",
            )
        };
        assert!(file("").is_ok(), "the table is optional");
        assert!(file("[reference]\nname = \"Earth\"").is_ok());
        assert!(file("[reference]\ntheoretical = true").is_ok());
        assert!(file("[reference]\nname = \"Earth\"\ntheoretical = true").is_err());
        assert!(file("[reference]").is_err());
        assert!(file("[reference]\ntheoretical = true\ndiscovered = 2000").is_err());
    }

    use super::*;
    use crate::planet::{Context, PlanetModel, derive_model_with};
    use crate::star::{SUN, StarInfo};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const AGE_GYR: f32 = 4.6;

    /// The system age `a` asks for, else [`AGE_GYR`].
    fn age_of(a: &Archetype, seed: u32) -> f32 {
        asked_age_gyr(&[a], seed).unwrap().unwrap_or(AGE_GYR)
    }

    /// The model of `a` at its own insolation (or period) round its own
    /// star (a G2V when it names none) of its own age (else 4.6 Gyr) and
    /// metallicity 0, drawn
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
        let star = StarInfo::new(host, orbit_au, eccentricity, age_of(a, seed));
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
            age_of(a, seed),
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
                resonance: None,
            },
            capture_age_share: None,
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
            "hot-neptune" => "hot-neptune",
            // The moons, round their host at its own orbit (see
            // `moon_model_of`): Titan's liquid methane, the others' ice
            // over an ocean or none, Io's dry rock.
            "titan-like" => "hydrocarbon-world",
            "europa-like" | "ganymede-like" | "callisto-like" | "triton-like"
            | "enceladus-like" => "ice-world",
            "io-like" => "volcanic-world",
            other => panic!("no expected class for {other}"),
        }
    }

    #[test]
    fn every_archetype_comes_out_as_its_class() {
        for a in Archetype::built_in() {
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
        assert_eq!(built_in_names().len(), 21);
    }

    #[test]
    fn titan_like_is_a_hydrocarbon_world() {
        let a = Archetype::load("titan-like").unwrap();
        for seed in 0..20 {
            let m = moon_model_of(&a, seed);
            assert_eq!(m.class.key(), expected(&a.name), "seed {seed}: {m:?}");
        }
    }

    #[test]
    fn moon_resonance_parses_and_defaults() {
        let enceladus = Archetype::load("enceladus-like").unwrap();
        let partner = enceladus.moon.unwrap().resonance.expect("a resonance");
        assert_eq!(
            partner.partner_mass_ratio,
            Some(ParamDefault::Range([1.85e-6, 2.0e-6]))
        );
        assert_eq!(partner.inner_partner_mass_ratio, None);
        let europa = Archetype::load("europa-like").unwrap();
        let partners = europa.moon.unwrap().resonance.expect("a resonance");
        assert_eq!(
            partners.inner_partner_mass_ratio,
            Some(ParamDefault::Range([4.6e-5, 4.8e-5]))
        );
        // Ganymede: Europa inside it, and Io inside Europa.
        let ganymede = Archetype::load("ganymede-like").unwrap();
        let inner = ganymede.moon.unwrap().resonance.expect("a resonance");
        assert_eq!(inner.partner_mass_ratio, None);
        assert_eq!(
            inner.inner_partner_mass_ratio,
            Some(ParamDefault::Range([2.4e-5, 2.6e-5]))
        );
        assert_eq!(
            inner.second_inner_partner_mass_ratio,
            Some(ParamDefault::Range([4.6e-5, 4.8e-5]))
        );
        assert!(
            Archetype::load("titan-like")
                .unwrap()
                .moon
                .unwrap()
                .resonance
                .is_none()
        );
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
        assert_eq!(o.radiogenic, Some(1.0));
    }

    /// The `earth-like` models of seeds 0 to 20, derived once (in parallel)
    /// for the three tests below that read them: each derivation takes
    /// about 5 s in release since the spectral column, and the tests
    /// derived the same 20 models three times over.
    fn earth_models() -> &'static [PlanetModel] {
        use rayon::prelude::*;
        static MODELS: std::sync::OnceLock<Vec<PlanetModel>> = std::sync::OnceLock::new();
        MODELS.get_or_init(|| {
            let a = Archetype::load("earth-like").unwrap();
            (0..=20u32)
                .into_par_iter()
                .map(|seed| model_of(&a, seed))
                .collect()
        })
    }

    /// Earth's 1.013 bar, within 0.7 to 1.3 on every seed: the archetype
    /// pins the mass, the light and the ammonia factor whose photolysis
    /// feeds the N₂.
    #[test]
    fn earth_like_keeps_its_ocean_and_its_life() {
        for (seed, m) in earth_models()[..20].iter().enumerate() {
            let s = m.surface.as_ref().expect("a bare world has a surface");
            let oceans = s.water.oceans;
            assert!((0.5..2.0).contains(&oceans), "seed {seed}: {oceans} oceans");
            assert!(s.life.arose, "seed {seed}: {:?}", s.life);
            let bar = s.gases.total();
            assert!((0.7..=1.3).contains(&bar), "seed {seed}: {bar} bar");
        }
    }

    /// Earth's internal heat flux, 0.087 to 0.092 W/m² (Davies & Davies
    /// 2010, the reference Earth's range), on every seed: the archetype
    /// pins Earth's radiogenic abundance as well as its mass and age.
    #[test]
    fn earth_like_has_earths_heat_flux() {
        for (seed, m) in earth_models()[..20].iter().enumerate() {
            let flux = m.heat.flux_w_m2(m.interior.radius_earth);
            assert!((0.087..=0.092).contains(&flux), "seed {seed}: {flux} W/m²");
        }
    }

    /// An asked Earth is the Earth of today, never a snowball, though its
    /// climate has a frozen state at 1 AU (`snowball = false`).
    #[test]
    fn earth_like_never_snowballs() {
        for (seed, m) in earth_models().iter().enumerate().skip(1) {
            let s = m.surface.as_ref().expect("a bare world has a surface");
            assert!(!s.climate.snowball, "seed {seed}: {:?}", s.climate);
            assert_eq!(m.inputs.snowball_draw, 1.0, "seed {seed}");
        }
    }

    /// Mars keeps about 0.004 oceans of ice (Carr & Head 2015, see
    /// `planet::classify::ICE_WORLD_MIN_OCEANS`), too little to show: the
    /// barren look, not the frozen one; and its 4 to 10 mbar of air
    /// (6.36 mbar on average, seasonal 4 to 9) on every seed, which the
    /// archetype's pinned mass, light and CO₂ factor give.
    #[test]
    fn mars_like_is_a_barren_desert_under_a_thin_air() {
        let a = Archetype::load("mars-like").unwrap();
        for seed in 0..20 {
            let m = model_of(&a, seed);
            assert_eq!(crate::planet::interim::base_of(&m), "barren", "seed {seed}");
            let s = m.surface.expect("a bare world has a surface");
            assert!(
                s.water.oceans < 0.01,
                "seed {seed}: {} oceans",
                s.water.oceans
            );
            let bar = s.gases.total();
            assert!((0.004..=0.010).contains(&bar), "seed {seed}: {bar} bar");
        }
    }

    /// Titan-like's surface pressure, bar: the spread of
    /// `archetype_census titan-like 500` over seeds 1-500 (min 1.263,
    /// 5th percentile 1.339, median 1.493, 95th 1.615, max 1.660 bar at
    /// `n2 = 0.82`), rounded outward to 0.05 bar (#565). Titan's own is
    /// 1.47 bar (Fulchignoni et al. 2005, doi:10.1038/nature04314).
    const TITAN_BAR: std::ops::RangeInclusive<f64> = 1.25..=1.70;

    /// Titan's air: a surface pressure within [`TITAN_BAR`] on every
    /// seed, of which 1 to 10 % is methane.
    #[test]
    fn titan_like_has_titan_air() {
        let a = Archetype::load("titan-like").unwrap();
        for seed in 0..40 {
            let m = moon_model_of(&a, seed);
            assert_eq!(m.class.key(), "hydrocarbon-world", "seed {seed}: {m:?}");
            let gases = m.surface.expect("a moon has a surface").gases;
            let bar = gases.total();
            assert!(TITAN_BAR.contains(&bar), "seed {seed}: {bar} bar");
            let ch4 = gases.ch4 / bar;
            assert!((0.01..=0.10).contains(&ch4), "seed {seed}: CH4 {ch4}");
        }
    }

    /// The same ranges when a whole system asks for the archetype, round
    /// the orbit and the age it draws there.
    #[test]
    fn faithful_archetypes_hold_in_a_whole_system() {
        for (name, lo, hi, seeds) in [
            ("earth-like", 0.7, 1.3, 20),
            ("mars-like", 0.004, 0.010, 20),
            ("venus-like", 60.0, 120.0, 20),
            ("titan-like", *TITAN_BAR.start(), *TITAN_BAR.end(), 40),
        ] {
            let a = Archetype::load(name).unwrap();
            for seed in 0..seeds {
                let (context, model, _) = drawn(&a, seed);
                assert!((context.age_gyr - 4.57).abs() < 1e-6, "{name} seed {seed}");
                let bar = model.surface.expect("a surface").gases.total();
                assert!((lo..=hi).contains(&bar), "{name} seed {seed}: {bar} bar");
            }
        }
    }

    #[test]
    fn solar_system_archetypes_carry_the_solar_age() {
        for name in [
            "mercury-like",
            "venus-like",
            "earth-like",
            "mars-like",
            "jupiter-like",
            "saturn-like",
            "uranus-like",
            "neptune-like",
            "io-like",
            "europa-like",
            "ganymede-like",
            "callisto-like",
            "titan-like",
            "enceladus-like",
            "triton-like",
        ] {
            let a = Archetype::load(name).unwrap();
            assert_eq!(a.inputs.age_gyr, Some(ParamDefault::Fixed(4.57)), "{name}");
        }
        for name in [
            "hot-jupiter",
            "hot-neptune",
            "mini-neptune",
            "super-earth",
            "ocean-world",
            "lava-world",
        ] {
            let a = Archetype::load(name).unwrap();
            assert_eq!(a.inputs.age_gyr, None, "{name}");
        }
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

    /// The context and model of the body of `a` in a whole system of
    /// seed `seed` that asks for it alone, round a G2V.
    fn drawn(a: &Archetype, seed: u32) -> (Context, PlanetModel, bool) {
        use crate::system::{SystemRequest, WholeSystemRequest, body_context, draw};
        let request = SystemRequest::System(WholeSystemRequest {
            age_gyr: None,
            star: Some("G2V".into()),
            metallicity: Some(0.0),
            planets: Some(BTreeMap::from([(a.name.clone(), 1)])),
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
        });
        let system = draw(&request, &mut StdRng::seed_from_u64(0)).unwrap();
        let body = system
            .bodies
            .iter()
            .find(|b| b.params.archetype.as_deref() == Some(a.name.as_str()))
            .unwrap_or_else(|| panic!("{} seed {seed}: no body", a.name));
        let context = body_context(&system, body).unwrap();
        let giant_moon = context.tide.is_some_and(|t| {
            t.host_kg >= crate::planet::GIANT_MIN_MASS_EARTH * crate::exomoon::EARTH_KG
        });
        (context, body.model.clone().unwrap(), giant_moon)
    }

    #[test]
    fn solar_system_archetypes_are_recognised() {
        let all = Archetype::built_in();
        let mut weak = Vec::new();
        for a in &all {
            let hits = (1..=20)
                .filter(|&seed| {
                    let (context, model, moon) = drawn(a, seed);
                    similar(&context, &model, moon, &all) == Some(a.name.as_str())
                })
                .count();
            eprintln!("similar {}: {hits}/20", a.name);
            if hits < 16 {
                weak.push(format!("{}: {hits}/20", a.name));
            }
        }
        assert!(weak.is_empty(), "named itself in under 16 of 20: {weak:?}");
    }

    #[test]
    fn ordinary_drawn_bodies_are_mostly_unnamed() {
        use crate::system::{SystemRequest, WholeSystemRequest, body_context, draw};
        let all = Archetype::built_in();
        let (mut named, mut total) = (0, 0);
        for seed in 1..=50 {
            let request = SystemRequest::System(WholeSystemRequest {
                age_gyr: None,
                star: Some("G2V".into()),
                metallicity: Some(0.0),
                planets: None,
                seed: Some(seed),
                width: Some(64),
                height: Some(32),
            });
            let system = draw(&request, &mut StdRng::seed_from_u64(0)).unwrap();
            for body in &system.bodies {
                let (Some(model), Some(context)) = (&body.model, body_context(&system, body))
                else {
                    continue;
                };
                let moon = context.tide.is_some_and(|t| {
                    t.host_kg >= crate::planet::GIANT_MIN_MASS_EARTH * crate::exomoon::EARTH_KG
                });
                total += 1;
                named += usize::from(similar(&context, model, moon, &all).is_some());
            }
        }
        eprintln!("similar: {named} of {total} bodies named");
        assert!(named * 10 <= total * 3, "{named} of {total} bodies named");
    }
}
