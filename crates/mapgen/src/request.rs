//! Generation requests: what a user asks for, before archetypes, the planet
//! model and random draws turn it into complete [`WorldParams`].
//!
//! A request names an [`Archetype`] (or none) and may set inputs of the
//! planet model (mass, composition, life); the world's ground, air and
//! water follow from the model ([`crate::planet::interim::params_for`]).
//! Shared by the command-line tool and the web server, so both accept
//! exactly the same parameters, ranges and defaults.
use crate::archetype::Archetype;
use crate::giants::GiantsMode;
use crate::planet::{InputOverrides, OCEAN_KG, Origin, PlanetModel};
use crate::world::WorldParams;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::fmt;

pub use crate::archetype::{
    AXIAL_TILT_DEG, CORE_MASS_FRACTION, ECCENTRICITY, ENVELOPE_FRACTION, MASS_EARTH, RADIOGENIC,
    VOLATILE_FACTOR, WATER_OCEANS,
};

/// Inclusive bounds of a numeric parameter.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Limits {
    pub min: f32,
    pub max: f32,
}

impl Limits {
    pub fn contains(self, v: f32) -> bool {
        (self.min..=self.max).contains(&v)
    }
}

// The ranges of the world parameters below (sea level to surface age, and
// the erosion) bound what the planet model and the looks give a world;
// a request no longer sets them.

/// Waterline shift: positive means more ocean.
pub const SEA_LEVEL: Limits = Limits {
    min: -1.0,
    max: 1.0,
};
/// Share of the surface above the waterline, by area.
pub const LAND_FRACTION: Limits = Limits { min: 0.0, max: 1.0 };
/// Share of mountain chains that become volcanic.
pub const VOLCANIC: Limits = Limits { min: 0.0, max: 1.0 };
/// Equatorial circumference in km.
pub const CIRCUMFERENCE_KM: Limits = Limits {
    min: 1.0,
    max: 1_000_000.0,
};
/// Surface pressure in bar: 0 is airless, Venus has 92.
pub const PRESSURE_BAR: Limits = Limits {
    min: 0.0,
    max: 1000.0,
};
/// Age of the oldest surviving surface, in Ga (the Solar System is 4.57 Ga
/// old).
pub const SURFACE_AGE_GA: Limits = Limits { min: 0.0, max: 4.6 };
/// Semi-major axis of the planet's orbit, AU.
pub const ORBIT_AU: Limits = Limits {
    min: 0.005,
    max: 1000.0,
};
/// Longitude of the perihelion, degrees from the spring equinox.
pub const PERIHELION_LONGITUDE_DEG: Limits = Limits {
    min: 0.0,
    max: 360.0,
};
/// Mass of a giant planet's world, Earth masses: from a small ice giant to
/// the deuterium-burning limit (13 Jupiter masses). A giant's world takes
/// no less (a mini-Neptune renders as an ice giant of that mass).
pub const GIANT_MASS: Limits = Limits {
    min: 5.0,
    max: 4_131.0,
};
/// Number of moons a request can ask for.
pub const MOON_COUNT: Limits = Limits {
    min: 1.0,
    max: crate::moons::MAX_MOON_COUNT as f32,
};
/// Star and orbit draws tried for a planet with room for the moons asked
/// for.
const MOON_ORBIT_ATTEMPTS: usize = 64;
/// Metallicity of the star, dex (see `crate::giants`).
pub const METALLICITY: Limits = Limits {
    min: crate::giants::METALLICITY.0,
    max: crate::giants::METALLICITY.1,
};
/// Epoch of the plates, Ma from the present (see `crate::plates`).
pub const DRIFT_MA: Limits = Limits {
    min: -crate::plates::DRIFT_LIMIT_MA as f32,
    max: crate::plates::DRIFT_LIMIT_MA as f32,
};
/// Position in the host star's habitable zone: 0 inner edge, 1 outer edge.
pub const HZ_POSITION: Limits = Limits {
    min: -2.0,
    max: 6.0,
};
/// Hydraulic erosion strength (droplets per erosion-grid pixel).
pub const EROSION: Limits = Limits { min: 0.0, max: 5.0 };
/// Map width and height in pixels.
pub const MAP_SIZE: (u32, u32) = (2, 16_384);

/// Map sizes the generation form offers, width and height in pixels. A
/// planet map spans 360° by 180°: every size is 2:1 and the default
/// ([`fallback::WIDTH`] × [`fallback::HEIGHT`]) is one of them.
pub const SIZE_PRESETS: [(u32, u32); 4] = [(1024, 512), (2048, 1024), (4096, 2048), (8192, 4096)];

/// Generator defaults for world parameters that neither the planet model
/// nor the look of its world sets (see `planet::interim`).
pub mod fallback {
    use crate::archetype::ParamDefault;
    /// About 10–70 % land (mostly 20–55 %); Earth has 29 %.
    pub const SEA_LEVEL: ParamDefault = ParamDefault::Range([0.10, 0.40]);
    pub const VOLCANIC: ParamDefault = ParamDefault::Range([0.0, 1.0]);
    /// Sub-Earth to ~2× Earth (Earth ≈ 40 075 km).
    pub const CIRCUMFERENCE_KM: ParamDefault = ParamDefault::Range([20_000.0, 80_000.0]);
    pub const AXIAL_TILT_DEG: ParamDefault = ParamDefault::Range([5.0, 40.0]);
    pub const EROSION: ParamDefault = ParamDefault::Fixed(1.0);
    /// Around Earth's 1 bar.
    pub const PRESSURE_BAR: ParamDefault = ParamDefault::Range([0.7, 1.5]);
    /// Earth's: erosion and subduction keep few craters older than this.
    pub const SURFACE_AGE_GA: ParamDefault = ParamDefault::Range([0.2, 0.6]);
    /// Inside the habitable zone of the star.
    pub const HZ_POSITION: ParamDefault = ParamDefault::Range([0.1, 0.9]);
    pub const WIDTH: u32 = 2048;
    pub const HEIGHT: u32 = 1024;
}

/// A request to generate a world. `None` fields come from the archetype
/// when it sets them, otherwise from the formation context or random
/// draws.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
pub struct WorldRequest {
    /// Built-in archetype name (`earth-like`) or archetype file path
    /// (`.toml`); `None` draws the planet from its formation context.
    pub planet: Option<String>,
    pub seed: Option<u32>,
    pub axial_tilt_deg: Option<f32>,
    /// Spectral type of the host star (`G2V`).
    pub star: Option<String>,
    /// Semi-major axis of the orbit in AU. Exclusive with `hz_position`.
    pub orbit_au: Option<f32>,
    /// Position of the orbit in the star's habitable zone (0 inner edge, 1
    /// outer edge). Exclusive with `orbit_au`.
    pub hz_position: Option<f32>,
    /// Eccentricity of the orbit, in `[0, 0.9]`.
    pub eccentricity: Option<f32>,
    /// Longitude of the orbit's perihelion, degrees from the spring equinox,
    /// in `[0, 360]` (Earth: 283): where in the seasons the planet is
    /// closest to its star. Drawn from the seed when left out; it matters
    /// on eccentric orbits only.
    pub perihelion_longitude_deg: Option<f32>,
    /// Age of the star, Gyr, in `(0, lifetime]` of its spectral type
    /// (about 10 Gyr for the Sun): drawn from the seed between 0.5 and 10
    /// Gyr (or the star's life when shorter) when left out. An older star
    /// has had longer to lock its planets and to cool its giants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_gyr: Option<f32>,
    /// Epoch of the plates, Ma from the present (negative in the past), in
    /// `[-300, 300]`; the present when left out (never drawn).
    pub drift_ma: Option<f32>,
    /// `auto` (the moons the seed draws, the default) or `none`.
    pub moons: Option<crate::moons::MoonsMode>,
    /// Metallicity of the star, `[Fe/H]` in dex, in `[-1, 0.5]`; drawn like
    /// the stars nearby when left out.
    pub metallicity: Option<f32>,
    /// `auto` (the giant planets the seed draws, the default) or `none`.
    pub giants: Option<crate::giants::GiantsMode>,
    /// Mass of the planet, Earth masses, in `[0.01, 4131]`: from the
    /// archetype's range, else drawn like a small planet's.
    pub mass_earth: Option<f64>,
    /// Iron share of the refractory (iron and rock) mass, in `[0, 0.9]`
    /// (Earth 0.325, Mercury about 0.7).
    pub core_mass_fraction: Option<f64>,
    /// Water and ices, Earth oceans (1.4e21 kg each), in `[0, 1e6]`.
    pub water_oceans: Option<f64>,
    /// Hydrogen and helium share of the mass at formation, in `[0, 0.99]`.
    pub envelope_fraction: Option<f64>,
    /// Factor on the CO₂ formation rule, in `[0, 100]`: 1 typical (the
    /// rule's scatter draws about 0.5 to 2).
    pub co2: Option<f64>,
    /// Factor on the N₂ formation rule, in `[0, 100]`: 1 typical.
    pub n2: Option<f64>,
    /// Radiogenic abundance at formation, Earth's = 1, in `[0.1, 10]`.
    pub radiogenic: Option<f64>,
    /// `true`: life arises wherever the world allows it; `false`: never;
    /// left out, drawn.
    pub life: Option<bool>,
    /// `auto` (rarely the moon of a giant, as the seed draws, the default),
    /// `giant` (always) or `none`.
    pub host: Option<crate::exomoon::HostMode>,
    /// Only meaningful for a system around this planet: the system then
    /// holds no other planet (a lone planet: its star, the planet and its
    /// moons, or the giant it orbits). The planet is then resolved with no
    /// giant neighbours, whatever `giants` says (it is overridden).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alone: Option<bool>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Why a request was rejected.
#[derive(Debug)]
pub enum RequestError {
    /// A parameter is outside its [`Limits`].
    OutOfRange { field: &'static str, limits: Limits },
    /// The map size is outside [`MAP_SIZE`].
    BadSize { field: &'static str, value: u32 },
    /// The star is not a spectral type the model knows.
    BadStar(String),
    /// Both `orbit_au` and `hz_position` are set.
    OrbitAndPosition,
    /// The moons asked for find no stable orbits around the planet: it is
    /// too close to its star (a Hill sphere too small, or despun by its
    /// star's tides with no moon light enough to last), and its star and
    /// orbit were set (otherwise they are drawn again).
    NoRoomForMoons { count: u8 },
    /// Moons asked for on the moon of a giant (`host: giant`), which keeps
    /// none of its own: its Hill sphere is too small.
    MoonsOfAMoon,
    /// Moons asked for on a giant too close to its star to keep that many:
    /// a hot giant keeps none, one its star has locked only light ones, and
    /// a small Hill sphere fewer.
    HotGiantMoons { count: u8 },
    /// A planet that is neither a built-in archetype nor a `.toml` file.
    UnknownArchetype(String),
    /// An archetype file that does not load (unreadable or invalid).
    BadArchetype(String),
    /// More water than the planet weighs (with its envelope).
    WaterHeavierThanPlanet { water_oceans: f64, mass_earth: f64 },
    /// The age asked is not between 0 and the star's main-sequence life.
    BadAge { age_gyr: f32, lifetime_gyr: f32 },
    /// More planets than a system holds.
    TooManyPlanets { asked: u32, max: u32 },
    /// More planets of a type than its band of distances holds at the
    /// spacing that keeps them stable.
    ZoneFull {
        body_type: String,
        asked: u32,
        fits: u32,
    },
    /// A type the star leaves no room for.
    TypeImpossible { body_type: String },
    /// A planet type that is not one of the old preset names (no longer
    /// produced: a system names archetypes, see `UnknownArchetype`).
    UnknownType(String),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::OutOfRange { field, limits } => {
                write!(
                    f,
                    "{field} must be between {} and {}",
                    limits.min, limits.max
                )
            }
            RequestError::BadSize { field, value } => write!(
                f,
                "{field} is {value}, it must be between {} and {}",
                MAP_SIZE.0, MAP_SIZE.1
            ),
            RequestError::BadStar(e) => write!(f, "star: {e}"),
            RequestError::NoRoomForMoons { count } => write!(
                f,
                "no stable orbits for {count} moon(s) around this planet: it is too close to \
                 its star, whose tides have despun it and pull down all but the lightest \
                 moons (pick a farther orbit or a hotter star)"
            ),
            RequestError::OrbitAndPosition => {
                write!(f, "set orbit_au or hz_position, not both")
            }
            RequestError::MoonsOfAMoon => write!(
                f,
                "the moon of a giant keeps no moons of its own (its Hill sphere is too small): \
                 ask for moons or for host giant, not both"
            ),
            RequestError::HotGiantMoons { count } => write!(
                f,
                "not that many moons for a giant this close to its star (asked for {count}): \
                 its Hill sphere is too small, or its star's tides have locked it and its \
                 moons spiral in (pick a farther orbit or fewer moons)"
            ),
            RequestError::BadAge {
                age_gyr,
                lifetime_gyr,
            } => write!(
                f,
                "age_gyr is {age_gyr}, it must be above 0 and at most {lifetime_gyr:.2}, the \
                 main-sequence life of this star"
            ),
            RequestError::UnknownArchetype(name) => write!(
                f,
                "unknown planet {name:?}: use one of {} or a .toml archetype file",
                crate::archetype::built_in_names().join(", ")
            ),
            RequestError::BadArchetype(e) => write!(f, "planet: {e}"),
            RequestError::WaterHeavierThanPlanet {
                water_oceans,
                mass_earth,
            } => write!(
                f,
                "water_oceans {water_oceans} weighs more than a planet of mass_earth \
                 {mass_earth} holds (one Earth ocean is 1.4e21 kg, Earth 5.97e24 kg): ask \
                 for less water or a heavier planet"
            ),
            RequestError::TooManyPlanets { asked, max } => {
                write!(f, "{asked} planets asked, a system holds at most {max}")
            }
            RequestError::ZoneFull {
                body_type,
                asked,
                fits,
            } => write!(
                f,
                "{asked} {body_type} planets asked, but only {fits} fit where they can orbit, \
                 spaced enough to stay stable"
            ),
            RequestError::TypeImpossible { body_type } => {
                write!(f, "this star leaves no room for a {body_type} planet")
            }
            RequestError::UnknownType(t) => write!(f, "unknown planet type {t:?}"),
        }
    }
}

impl std::error::Error for RequestError {}

/// A resolved request: complete parameters, which carry the planet's model
/// (`params.model`, derived under `body_seed(seed, "world/model")` in the
/// context of its star and orbit; for the moon of a giant, the giant's
/// orbit round the star) and the archetype the request named
/// (`params.archetype`, `None` for a planet drawn from its formation
/// context).
#[derive(Clone, Debug)]
pub struct Resolved {
    pub params: WorldParams,
    /// Fields of the request the planet has no use for, ignored: a giant
    /// has no giant neighbours and is nobody's moon (`giants`, `host`); a
    /// lone planet has no giant neighbours (`giants`).
    pub ignored: Vec<&'static str>,
}

impl Resolved {
    /// The planet's model.
    ///
    /// # Panics
    ///
    /// Never for a resolved request: only a moon has no model.
    pub fn model(&self) -> &PlanetModel {
        self.params.model.as_ref().expect("a planet has a model")
    }
}

/// Where and around what a world sits, and what it asks of its world, once
/// a request is resolved: what [`crate::planet::interim::params_for`] reads
/// beside the planet's model.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub seed: u32,
    /// Star code (see [`crate::star`]).
    pub star: f32,
    pub age_gyr: f32,
    pub orbit_au: f32,
    pub eccentricity: f32,
    pub metallicity: f32,
    /// Degrees; `None` draws it (and lets the moons and the tides set it).
    pub axial_tilt_deg: Option<f32>,
    /// Degrees; `None` draws it from the seed.
    pub perihelion_longitude_deg: Option<f32>,
    pub drift_ma: f32,
    pub width: u32,
    pub height: u32,
    pub moons: crate::moons::MoonsMode,
    pub giants: GiantsMode,
    pub host: crate::exomoon::HostMode,
}

/// Whether `v` lies within `limits`.
fn within(v: f64, limits: Limits) -> bool {
    (f64::from(limits.min)..=f64::from(limits.max)).contains(&v)
}

/// Earth's mass, kg, as the planet model takes it.
const EARTH_KG: f64 = crate::exomoon::EARTH_KG;

impl WorldRequest {
    /// Checks every explicit value against its limits.
    pub fn validate(&self) -> Result<(), RequestError> {
        if self.orbit_au.is_some() && self.hz_position.is_some() {
            return Err(RequestError::OrbitAndPosition);
        }
        if let Some(crate::moons::MoonsMode::Count(n)) = self.moons
            && !(1..=crate::moons::MAX_MOON_COUNT).contains(&n)
        {
            return Err(RequestError::OutOfRange {
                field: "moons",
                limits: MOON_COUNT,
            });
        }
        if let Some(star) = &self.star {
            crate::star::parse_type(star).map_err(RequestError::BadStar)?;
        }
        for (field, value, limits) in [
            ("axial_tilt_deg", self.axial_tilt_deg, AXIAL_TILT_DEG),
            ("orbit_au", self.orbit_au, ORBIT_AU),
            ("hz_position", self.hz_position, HZ_POSITION),
            ("eccentricity", self.eccentricity, ECCENTRICITY),
            (
                "perihelion_longitude_deg",
                self.perihelion_longitude_deg,
                PERIHELION_LONGITUDE_DEG,
            ),
            ("drift_ma", self.drift_ma, DRIFT_MA),
            ("metallicity", self.metallicity, METALLICITY),
        ] {
            if let Some(v) = value
                && !limits.contains(v)
            {
                return Err(RequestError::OutOfRange { field, limits });
            }
        }
        for (field, value, limits) in [
            ("mass_earth", self.mass_earth, MASS_EARTH),
            (
                "core_mass_fraction",
                self.core_mass_fraction,
                CORE_MASS_FRACTION,
            ),
            ("water_oceans", self.water_oceans, WATER_OCEANS),
            (
                "envelope_fraction",
                self.envelope_fraction,
                ENVELOPE_FRACTION,
            ),
            ("co2", self.co2, VOLATILE_FACTOR),
            ("n2", self.n2, VOLATILE_FACTOR),
            ("radiogenic", self.radiogenic, RADIOGENIC),
        ] {
            if let Some(v) = value
                && !within(v, limits)
            {
                return Err(RequestError::OutOfRange { field, limits });
            }
        }
        for (field, value) in [("width", self.width), ("height", self.height)] {
            if let Some(v) = value
                && !(MAP_SIZE.0..=MAP_SIZE.1).contains(&v)
            {
                return Err(RequestError::BadSize { field, value: v });
            }
        }
        Ok(())
    }

    /// The model inputs set instead of drawn: the request's value, else the
    /// archetype's (its ranges drawn from `rng`, all of them whatever the
    /// request sets, so that the draws stay in step), else `None`.
    pub fn overrides(
        &self,
        archetype: Option<&Archetype>,
        rng: &mut impl RngExt,
    ) -> InputOverrides {
        let a = archetype.map(|a| a.overrides(rng)).unwrap_or_default();
        InputOverrides {
            core_mass_fraction: self.core_mass_fraction.or(a.core_mass_fraction),
            water_oceans: self.water_oceans.or(a.water_oceans),
            envelope_fraction: self.envelope_fraction.or(a.envelope_fraction),
            co2: self.co2.or(a.co2),
            n2: self.n2.or(a.n2),
            radiogenic: self.radiogenic.or(a.radiogenic),
            life: self.life.or(a.life),
        }
    }

    /// Validates the request, loads its archetype and resolves it (see
    /// [`resolve_with_archetype`](Self::resolve_with_archetype)).
    pub fn resolve(&self, rng: &mut impl RngExt) -> Result<Resolved, RequestError> {
        self.validate()?;
        let archetype = self.planet.as_deref().map(load_archetype).transpose()?;
        self.resolve_with_archetype(archetype, rng)
    }

    /**
     * Like [`resolve`](Self::resolve) with an already loaded archetype
     * (`self.planet` is ignored), in this order:
     *
     * 1. the seed: the request's, else drawn;
     * 2. the archetype's mass and inputs, drawn in its ranges from the
     *    seed's own stream (`body_seed(seed, "world/archetype")`), so that
     *    a request pinning the seed and the archetype draws them again;
     * 3. the star: the request's, else the archetype's, else drawn;
     * 4. its age;
     * 5. the orbit: the request's `orbit_au` or `hz_position`, else the
     *    archetype's insolation (`a = sqrt(L / S)`) or period (Kepler),
     *    else drawn in the habitable zone;
     * 6. the metallicity: the request's, else from the seed;
     * 7. the mass: the request's, else the archetype's, else drawn like a
     *    small planet's ([`crate::planet::draw_mass`]);
     * 8. the eccentricity: the request's, else the archetype's, else drawn
     *    (a giant's by Kipping 2013, a small planet's by Van Eylen et al.
     *    2019);
     * 9. the model, by [`crate::planet::derive_model_with`] under
     *    `body_seed(seed, "world/model")`; a drawn eccentricity the star's
     *    tides circularise for the model's mass and radius becomes 0, and
     *    the model is derived again;
     * 10. the world, by [`crate::planet::interim::params_for`], its own
     *     draws from the seed's stream `body_seed(seed, "world/params")`.
     *
     * Eccentric giants then force a drawn eccentricity up (the model is
     * derived again at it), and moons asked for that find no room draw a
     * star and orbit left open again.
     */
    pub fn resolve_with_archetype(
        &self,
        archetype: Option<Archetype>,
        rng: &mut impl RngExt,
    ) -> Result<Resolved, RequestError> {
        self.validate()?;
        let a = archetype.as_ref();
        let seed = self.seed.unwrap_or_else(|| rng.random());
        let mut stream = seeded(seed, "world/archetype");
        let archetype_mass = a.map(|a| a.mass(&mut stream));
        let overrides = self.overrides(a, &mut stream);
        let star_kind = self
            .star
            .as_deref()
            .or_else(|| a.and_then(|a| a.orbit.star.as_deref()));
        let star_set = star_kind.is_some();
        let orbit_set = self.orbit_au.is_some() || self.hz_position.is_some();
        let place = |mut rng: &mut dyn rand::Rng| -> Result<(f32, f32, f32), RequestError> {
            let rng = &mut rng;
            let star = match star_kind {
                Some(kind) => crate::star::parse_type(kind).map_err(RequestError::BadStar)?,
                None => crate::star::draw_type_living(self.age_gyr, rng),
            };
            // The age, from its own stream: no other draw shifts.
            let age_gyr = crate::star::resolve_age_gyr(self.age_gyr, star, seed)?;
            let v = crate::star::values(star);
            let orbit_au = match (self.orbit_au, self.hz_position) {
                (Some(au), _) => au,
                (None, Some(position)) => crate::star::orbit_at(star, position),
                (None, None) => match a.and_then(|a| {
                    a.orbit
                        .orbit_au(f64::from(v.luminosity), f64::from(v.mass), rng)
                }) {
                    Some(au) => (au as f32).clamp(ORBIT_AU.min, ORBIT_AU.max),
                    None => crate::star::orbit_at(star, fallback::HZ_POSITION.pick(rng)),
                },
            };
            Ok((star, age_gyr, orbit_au))
        };
        // Not drawn from the main stream: the values above keep their draws.
        let metallicity = self
            .metallicity
            .unwrap_or_else(|| crate::giants::draw_metallicity(seed));
        let mass_earth = match self.mass_earth.or(archetype_mass) {
            Some(mass) => mass,
            None => crate::planet::draw_mass(Origin::Small, rng),
        };
        if let Some(water_oceans) = overrides.water_oceans {
            let envelope = overrides.envelope_fraction.unwrap_or(0.0);
            if water_oceans * OCEAN_KG > mass_earth * EARTH_KG * (1.0 - envelope) {
                return Err(RequestError::WaterHeavierThanPlanet {
                    water_oceans,
                    mass_earth,
                });
            }
        }
        let asked_eccentricity = self.eccentricity.or_else(|| {
            a.and_then(|a| a.orbit.eccentricity)
                .map(|e| e.pick(&mut stream))
        });
        let tilt = self.axial_tilt_deg.or_else(|| {
            a.and_then(|a| a.orbit.axial_tilt)
                .map(|t| t.pick(&mut stream))
        });
        let moons = self.moons.unwrap_or_default();
        let mut attempts = 0;
        let mut params = loop {
            let (star, age_gyr, orbit_au) = place(rng)?;
            let eccentricity = match asked_eccentricity {
                Some(e) => e,
                // A giant follows Kipping 2013, a small planet alone in
                // its system Van Eylen et al. 2019.
                None if mass_earth >= crate::planet::GIANT_MIN_MASS_EARTH => {
                    crate::star::draw_eccentricity(rng)
                }
                None => crate::star::draw_small_eccentricity(false, rng),
            };
            let at = |e: f32| World {
                seed,
                star,
                age_gyr,
                orbit_au,
                eccentricity: e,
                metallicity,
                mass_earth,
                overrides: &overrides,
            };
            let mut model = at(eccentricity).model();
            // The star's tides circularise the closest orbits (a planet
            // they despin on an eccentric orbit keeps it, in the 3:2
            // resonance: see `crate::spin`).
            let eccentricity = if asked_eccentricity.is_none() && circularised(&at(0.0), &model) {
                model = at(0.0).model();
                0.0
            } else {
                eccentricity
            };
            let giant = crate::planet::interim::renders_as_giant(&model);
            let mut host = if giant {
                crate::exomoon::HostMode::None
            } else {
                self.host.unwrap_or_default()
            };
            // Moons asked for: the world is not a giant's moon, which
            // keeps none (an error when that is asked for too).
            if moons.count().is_some() {
                match host {
                    crate::exomoon::HostMode::Giant => return Err(RequestError::MoonsOfAMoon),
                    crate::exomoon::HostMode::Auto => host = crate::exomoon::HostMode::None,
                    crate::exomoon::HostMode::None => {}
                }
            }
            let placed = Placed {
                seed,
                star,
                age_gyr,
                orbit_au,
                eccentricity,
                metallicity,
                axial_tilt_deg: tilt,
                perihelion_longitude_deg: self
                    .perihelion_longitude_deg
                    .map(|v| v.rem_euclid(360.0)),
                drift_ma: self.drift_ma.unwrap_or(0.0),
                width: self.width.unwrap_or(fallback::WIDTH),
                height: self.height.unwrap_or(fallback::HEIGHT),
                moons,
                giants: if giant || self.alone == Some(true) {
                    GiantsMode::None
                } else {
                    self.giants.unwrap_or_default()
                },
                host,
            };
            let world = |model: &PlanetModel, placed: &Placed| {
                crate::planet::interim::params_for(model, placed, &mut seeded(seed, "world/params"))
            };
            let mut params = world(&model, &placed);
            // Eccentric giants force an eccentricity on the world, before
            // the moons and the tides see its orbit: the model and the
            // world follow it, so that a request pinning the final
            // eccentricity resolves to the same world.
            if asked_eccentricity.is_none() {
                force_eccentricity(&mut params, giant);
                if params.eccentricity != eccentricity {
                    model = at(params.eccentricity).model();
                    let forced = Placed {
                        eccentricity: params.eccentricity,
                        ..placed
                    };
                    params = world(&model, &forced);
                }
            }
            // Moons asked for need room: a Hill sphere wide enough, and on
            // a planet its star despun, moons light enough to outlast its
            // tides. A drawn star and orbit are drawn again until they
            // leave it (the generation stays coherent); set ones that do
            // not are an error.
            if let Some(n) = moons.count()
                && !crate::moons::MoonSystem::draw(&params)
                    .is_some_and(|m| m.moons.len() >= usize::from(n))
            {
                // A giant far from its star has room for any number of
                // moons; close in, a hot one keeps none, one its star has
                // locked only light ones, and a small Hill sphere fewer.
                // Drawing it another orbit would make it another kind of
                // giant than the one asked for: an error.
                if giant {
                    return Err(RequestError::HotGiantMoons { count: n });
                }
                if (star_set && orbit_set) || attempts >= MOON_ORBIT_ATTEMPTS {
                    return Err(RequestError::NoRoomForMoons { count: n });
                }
                attempts += 1;
                continue;
            }
            break params;
        };
        let giant = params.giant_mass_earth.is_some();
        let ignored: Vec<&'static str> = [
            ("giants", giant && self.giants.is_some()),
            ("host", giant && self.host.is_some()),
            (
                "giants",
                !giant
                    && self.alone == Some(true)
                    && self.giants.is_some_and(|g| g != GiantsMode::None),
            ),
        ]
        .into_iter()
        .filter_map(|(field, set)| set.then_some(field))
        .collect();
        params.archetype = archetype.map(|a| a.name);
        Ok(Resolved { params, ignored })
    }

    /**
     * This request with every field it left open pinned to the value
     * `params` and `model` resolved it to: a request that resolves to the
     * same world whatever the random generator (the CLI's "Reproduce
     * with" line, the `create` of an export around a planet).
     *
     * The archetype stays as asked (its draws come from the seed's own
     * stream); the model's mass, core share and radiogenic abundance are
     * pinned, and its life when it was set. The other inputs stay as the
     * request set them: the seed draws them again exactly, which a value
     * read back from the model (the water after escape, a CO₂ factor
     * divided out of its rule) would not. A position in the habitable
     * zone asked for stays, the orbit it gave stays open. A giant gets no
     * giant neighbours or host pinned: [`resolve`](Self::resolve) would
     * ignore them.
     */
    pub fn pinned(&self, params: &WorldParams, model: Option<&PlanetModel>) -> WorldRequest {
        let giant = params.giant_mass_earth.is_some();
        let life = model.and_then(|m| match m.inputs.life_draw {
            0.0 => Some(true),
            1.0 => Some(false),
            _ => None,
        });
        WorldRequest {
            planet: self.planet.clone(),
            seed: Some(params.seed),
            axial_tilt_deg: self.axial_tilt_deg.or(Some(params.axial_tilt_deg)),
            star: self
                .star
                .clone()
                .or_else(|| Some(crate::star::type_name(params.star))),
            orbit_au: match self.hz_position {
                Some(_) => self.orbit_au,
                None => self.orbit_au.or(Some(params.orbit_au)),
            },
            hz_position: self.hz_position,
            eccentricity: self.eccentricity.or(Some(params.eccentricity)),
            perihelion_longitude_deg: self
                .perihelion_longitude_deg
                .or(params.perihelion_longitude_deg),
            age_gyr: self.age_gyr.or(Some(params.age_gyr)),
            drift_ma: self.drift_ma.or(Some(params.drift_ma)),
            moons: self.moons.or(Some(params.moons)),
            metallicity: self.metallicity.or(Some(params.metallicity)),
            giants: if giant {
                self.giants
            } else {
                self.giants.or(Some(params.giants))
            },
            mass_earth: self
                .mass_earth
                .or_else(|| model.map(|m| m.drawn_mass_earth)),
            core_mass_fraction: self
                .core_mass_fraction
                .or_else(|| model.map(|m| m.inputs.core_mass_fraction)),
            water_oceans: self.water_oceans,
            envelope_fraction: self.envelope_fraction,
            co2: self.co2,
            n2: self.n2,
            radiogenic: self
                .radiogenic
                .or_else(|| model.map(|m| m.inputs.radiogenic)),
            life: self.life.or(life),
            host: if giant {
                self.host
            } else {
                self.host.or(Some(params.host))
            },
            alone: self.alone,
            width: Some(params.width),
            height: Some(params.height),
        }
    }
}

/// The archetype `spec` names: a built-in name, or a `.toml` file.
pub fn load_archetype(spec: &str) -> Result<Archetype, RequestError> {
    if !spec.ends_with(".toml") && !crate::archetype::built_in_names().contains(&spec) {
        return Err(RequestError::UnknownArchetype(spec.to_string()));
    }
    Archetype::load(spec).map_err(|e| RequestError::BadArchetype(e.to_string()))
}

/// A generator seeded by the world's `seed` and `id`, whose draws shift no
/// other.
fn seeded(seed: u32, id: &str) -> StdRng {
    StdRng::seed_from_u64(u64::from(crate::system::body_seed(seed, id)))
}

/// What the model of a requested world is derived from.
struct World<'a> {
    seed: u32,
    star: f32,
    age_gyr: f32,
    orbit_au: f32,
    eccentricity: f32,
    metallicity: f32,
    mass_earth: f64,
    overrides: &'a InputOverrides,
}

impl World<'_> {
    /// The model, in the context of its star seen from its orbit, the ice
    /// line of the star (2.7 AU √L, as a system places it) and the
    /// metallicity, from the seed's stream `world/model`.
    fn model(&self) -> PlanetModel {
        let star =
            crate::star::StarInfo::new(self.star, self.orbit_au, self.eccentricity, self.age_gyr);
        let ice_line_au = 2.7 * star.luminosity.sqrt();
        let context = crate::planet::Context::of(&star, self.metallicity, ice_line_au);
        crate::planet::derive_model_with(
            self.seed,
            "world",
            &context,
            self.mass_earth,
            self.overrides,
        )
    }
}

/// Whether the star's tides have circularised the orbit of `model` (on
/// `at`'s circular orbit): a giant with a giant's tides, any other body with
/// a rock's, each at the model's mass and radius.
fn circularised(at: &World<'_>, model: &PlanetModel) -> bool {
    let star = crate::star::StarInfo::new(at.star, at.orbit_au, 0.0, at.age_gyr);
    let mass = model.inputs.mass_earth as f32;
    let radius = model.interior.radius_earth as f32;
    if crate::planet::interim::renders_as_giant(model) {
        crate::spin::giant_circularised(&star, mass, radius)
    } else {
        crate::spin::rocky_circularised(&star, mass, radius)
    }
}

/// Eccentric giants force an eccentricity on a world whose eccentricity was
/// drawn: it becomes at least that, unless its star's tides have
/// circularised its orbit. The giants are placed around the world's orbit,
/// whose eccentricity they then change: settled in three rounds at most.
fn force_eccentricity(params: &mut WorldParams, giant: bool) {
    let at = crate::star::StarInfo::new(params.star, params.orbit_au, 0.0, params.age_gyr);
    let circular = if giant {
        crate::spin::circularised(&at)
    } else {
        crate::spin::rocky_circularised(
            &at,
            params.mass_earth(),
            params.circumference_km / crate::noise::EARTH_CIRCUMFERENCE_KM,
        )
    };
    if circular {
        return;
    }
    for _ in 0..3 {
        let forced =
            crate::giants::GiantSystem::draw(params).map_or(0.0, |g| g.forced_eccentricity);
        if forced <= params.eccentricity {
            break;
        }
        params.eccentricity = forced.min(ECCENTRICITY.max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::PlanetClass;
    use rand::SeedableRng;

    /// A request for `planet` with `seed`, 64×32.
    fn named(planet: Option<&str>, seed: u32) -> WorldRequest {
        WorldRequest {
            planet: planet.map(Into::into),
            seed: Some(seed),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        }
    }

    fn resolve(request: &WorldRequest) -> Result<Resolved, RequestError> {
        request.resolve(&mut StdRng::seed_from_u64(7))
    }

    #[test]
    fn earth_like_resolves_through_the_model() {
        let resolved = resolve(&named(Some("earth-like"), 3)).unwrap();
        assert_eq!(resolved.model().class, PlanetClass::Rocky);
        assert_eq!(resolved.params.archetype.as_deref(), Some("earth-like"));
        let p = &resolved.params;
        assert!(
            (p.circumference_km - 40_075.0).abs() < 4_007.5,
            "{}",
            p.circumference_km
        );
        assert!((0.5..2.0).contains(&p.pressure_bar), "{}", p.pressure_bar);
        // Its star and tilt are the archetype's.
        assert_eq!(p.star, crate::star::SUN);
        assert_eq!(p.axial_tilt_deg, 23.44);
    }

    #[test]
    fn an_override_wins_over_the_archetype() {
        let request = WorldRequest {
            water_oceans: Some(0.0),
            ..named(Some("earth-like"), 3)
        };
        let resolved = resolve(&request).unwrap();
        let surface = resolved.model().surface.expect("a surface");
        assert_eq!(surface.water.oceans, 0.0);
        let mut params = resolved.params.clone();
        crate::planet::interim::calibrate(&mut params, resolved.model());
        assert_eq!(params.sea_level, SEA_LEVEL.min);
        // The other inputs stay the archetype's.
        assert_eq!(resolved.model().inputs.core_mass_fraction, 0.325_f32 as f64);
    }

    #[test]
    fn water_heavier_than_the_planet_is_refused() {
        let request = WorldRequest {
            mass_earth: Some(0.1),
            water_oceans: Some(5000.0),
            ..named(None, 1)
        };
        let err = resolve(&request).unwrap_err();
        assert!(
            matches!(
                err,
                RequestError::WaterHeavierThanPlanet {
                    water_oceans: 5000.0,
                    mass_earth: 0.1
                }
            ),
            "{err:?}"
        );
        let message = err.to_string();
        assert!(
            message.contains("5000") && message.contains("0.1"),
            "{message}"
        );
    }

    #[test]
    fn a_giant_archetype_makes_a_giant_world() {
        for seed in 0..4 {
            let resolved = resolve(&named(Some("jupiter-like"), seed)).unwrap();
            let mass = resolved.params.giant_mass_earth.expect("a giant world");
            assert!((286.0..350.0).contains(&mass), "seed {seed}: {mass}");
            assert_eq!(resolved.model().class, PlanetClass::GasGiant, "seed {seed}");
            assert_eq!(resolved.params.gravity, None);
        }
    }

    #[test]
    fn a_hot_jupiter_sits_at_its_period() {
        for seed in 0..8 {
            let p = resolve(&named(Some("hot-jupiter"), seed)).unwrap().params;
            let mass = crate::star::values(p.star).mass;
            let period = 365.25 * (p.orbit_au.powi(3) / mass).sqrt();
            assert!((0.99..5.01).contains(&period), "seed {seed}: {period} d");
            assert!(p.giant_mass_earth.is_some(), "seed {seed}");
        }
    }

    #[test]
    fn an_unnamed_world_is_drawn_from_its_context() {
        let a = resolve(&named(None, 9)).unwrap();
        let b = resolve(&named(None, 9)).unwrap();
        assert_eq!(a.params.archetype, None);
        assert_eq!(
            serde_json::to_value(&a.params).unwrap(),
            serde_json::to_value(&b.params).unwrap()
        );
        assert_eq!(a.model(), b.model());
        // Drawn like a small planet.
        let (lo, hi) = crate::planet::SMALL_MASS;
        assert!((lo..=hi).contains(&a.model().drawn_mass_earth));
    }

    #[test]
    fn an_unknown_archetype_is_named() {
        let err = resolve(&named(Some("terran"), 1)).unwrap_err();
        assert!(
            matches!(&err, RequestError::UnknownArchetype(name) if name == "terran"),
            "{err:?}"
        );
        assert!(err.to_string().contains("earth-like"), "{err}");
    }

    #[test]
    fn the_pinned_request_reproduces_the_world() {
        let requests = [
            named(None, 4),
            named(Some("earth-like"), 4),
            named(Some("mars-like"), 4),
            named(Some("jupiter-like"), 4),
            named(Some("hot-jupiter"), 4),
            named(Some("mini-neptune"), 4),
            WorldRequest {
                mass_earth: Some(1.0),
                moons: Some(crate::moons::MoonsMode::Count(2)),
                ..named(None, 5)
            },
            WorldRequest {
                host: Some(crate::exomoon::HostMode::Giant),
                ..named(Some("earth-like"), 6)
            },
            // A position asked for stays one (the orbit it gives is not
            // pinned beside it, which the request would refuse).
            WorldRequest {
                hz_position: Some(0.3),
                water_oceans: Some(3.0),
                ..named(None, 7)
            },
            WorldRequest {
                seed: None,
                ..named(Some("super-earth"), 0)
            },
        ];
        for request in requests {
            for draw in 0..3u64 {
                let rng = &mut StdRng::seed_from_u64(draw);
                let resolved = request.resolve(rng).unwrap();
                let pinned = request.pinned(&resolved.params, Some(resolved.model()));
                let again = pinned
                    .resolve(&mut StdRng::seed_from_u64(draw + 1000))
                    .unwrap();
                assert!(again.ignored.is_empty(), "{pinned:?}: {:?}", again.ignored);
                assert_eq!(again.model(), resolved.model(), "{pinned:?}");
                assert_eq!(
                    serde_json::to_value(&again.params).unwrap(),
                    serde_json::to_value(&resolved.params).unwrap(),
                    "{pinned:?}"
                );
            }
        }
    }

    #[test]
    fn knob_fields_are_refused() {
        for knob in [
            r#"{"sea_level":0.1}"#,
            r#"{"land_fraction":0.3}"#,
            r#"{"volcanic":0.5}"#,
            r#"{"circumference_km":40000}"#,
            r#"{"erosion":1}"#,
            r#"{"pressure_bar":1}"#,
            r#"{"surface_age_ga":0.4}"#,
        ] {
            assert!(
                serde_json::from_str::<WorldRequest>(knob).is_err(),
                "{knob}"
            );
        }
        let inputs: WorldRequest = serde_json::from_str(
            r#"{"planet":"earth-like","mass_earth":1.2,"water_oceans":2,"life":false}"#,
        )
        .unwrap();
        assert_eq!(inputs.life, Some(false));
    }

    #[test]
    fn inputs_outside_their_limits_are_refused() {
        let bad = [
            WorldRequest {
                mass_earth: Some(5000.0),
                ..Default::default()
            },
            WorldRequest {
                core_mass_fraction: Some(0.95),
                ..Default::default()
            },
            WorldRequest {
                radiogenic: Some(0.0),
                ..Default::default()
            },
            WorldRequest {
                width: Some(1),
                ..Default::default()
            },
        ];
        for request in bad {
            assert!(
                matches!(
                    request.validate(),
                    Err(RequestError::OutOfRange { .. } | RequestError::BadSize { .. })
                ),
                "{request:?}"
            );
        }
    }

    #[test]
    fn the_star_and_orbit_come_from_the_request_then_the_archetype() {
        use crate::star::{SUN, StarInfo, parse_type};
        // The archetype's star and light (Mars around the Sun, 0.39 to
        // 0.47 Earth's).
        let mars = resolve(&named(Some("mars-like"), 2)).unwrap().params;
        assert_eq!(mars.star, SUN);
        let light = StarInfo::new(mars.star, mars.orbit_au, 0.0, mars.age_gyr).insolation;
        assert!((0.38..0.48).contains(&light), "{light}");
        // A request's star and position win; the position gives the distance.
        let red = resolve(&WorldRequest {
            star: Some("M4V".into()),
            hz_position: Some(0.5),
            // The position is that of a circular orbit.
            eccentricity: Some(0.0),
            ..named(Some("earth-like"), 2)
        })
        .unwrap()
        .params;
        assert_eq!(red.star, parse_type("M4V").unwrap());
        let info = StarInfo::new(
            red.star,
            red.orbit_au,
            red.eccentricity,
            crate::star::DEFAULT_AGE_GYR,
        );
        assert!(
            (info.hz_position - 0.5).abs() < 1e-3,
            "{}",
            info.hz_position
        );
        // A world with no archetype lands inside the habitable zone of
        // whatever star.
        for seed in 0..20 {
            let p = resolve(&named(None, seed)).unwrap().params;
            let position =
                StarInfo::new(p.star, p.orbit_au, 0.0, crate::star::DEFAULT_AGE_GYR).hz_position;
            assert!((0.04..0.96).contains(&position), "{position}");
        }
        // Bad stars and both orbits at once are refused.
        let bad = WorldRequest {
            star: Some("Q1".into()),
            ..Default::default()
        };
        assert!(matches!(resolve(&bad), Err(RequestError::BadStar(_))));
        let both = WorldRequest {
            orbit_au: Some(1.0),
            hz_position: Some(0.5),
            ..Default::default()
        };
        assert!(matches!(
            both.validate(),
            Err(RequestError::OrbitAndPosition)
        ));
    }

    #[test]
    fn size_presets_are_two_to_one_and_include_the_default() {
        assert!(SIZE_PRESETS.iter().all(|&(w, h)| w == 2 * h));
        assert!(SIZE_PRESETS.contains(&(fallback::WIDTH, fallback::HEIGHT)));
    }

    #[test]
    fn missing_values_are_drawn_within_fallback_ranges() {
        let resolved = WorldRequest::default().resolve(&mut rand::rng()).unwrap();
        let p = resolved.params;
        // 5 to 40°, or 0 to 85° without a moon to steady it.
        assert!((0.0..=85.0).contains(&p.axial_tilt_deg));
        assert!(SURFACE_AGE_GA.contains(p.surface_age_ga));
        assert_eq!((p.width, p.height), (2048, 1024));
    }

    /// A planet of Earth's mass at 1 AU round the Sun on a circular orbit,
    /// not a giant's moon, its tilt left to the draws.
    fn earth_mass(seed: u32) -> WorldRequest {
        WorldRequest {
            mass_earth: Some(1.0),
            // Bare: its surface shows.
            envelope_fraction: Some(0.0),
            star: Some("G2V".into()),
            orbit_au: Some(1.0),
            eccentricity: Some(0.0),
            host: Some(crate::exomoon::HostMode::None),
            ..named(None, seed)
        }
    }

    #[test]
    fn moons_steady_the_drawn_tilt_and_set_the_shores() {
        let (mut free, mut steady) = (0, 0);
        for seed in 0..400 {
            let resolved = resolve(&earth_mass(seed)).unwrap();
            let p = resolved.params.clone();
            let system = crate::moons::MoonSystem::draw(&p).unwrap();
            let effects = crate::moons::PlanetEffects::of(&p, Some(&system));
            let base = crate::planet::interim::look_for(resolved.model())
                .rules
                .shore_width_km;
            assert!((p.look.rules.shore_width_km - base * effects.shore_factor).abs() < 1e-3);
            if effects.stabilised {
                steady += 1;
                assert!(
                    (5.0..=40.0).contains(&p.axial_tilt_deg),
                    "{}",
                    p.axial_tilt_deg
                );
            } else {
                free += usize::from(!(5.0..=40.0).contains(&p.axial_tilt_deg));
            }
        }
        // Most Earths have no large moon: their tilt ranges from 0 to 85°.
        assert!(steady > 10 && free > 100, "{steady} {free}");
        // A tilt asked for stays.
        let asked = WorldRequest {
            axial_tilt_deg: Some(55.0),
            ..earth_mass(1)
        };
        assert_eq!(resolve(&asked).unwrap().params.axial_tilt_deg, 55.0);
    }

    #[test]
    fn moons_asked_for_get_a_star_and_orbit_with_room_for_them() {
        let mut despun = 0;
        for seed in 0..40 {
            // A small planet: a giant drawn close in keeps fewer moons.
            let request = WorldRequest {
                mass_earth: Some(1.0),
                moons: Some(crate::moons::MoonsMode::Count(3)),
                ..named(None, seed)
            };
            // The draws the seed leaves open come from a seeded generator,
            // so that the test checks the same worlds on every run.
            let mut rng = StdRng::seed_from_u64(u64::from(seed));
            let p = request.resolve(&mut rng).unwrap().params;
            let system = crate::moons::MoonSystem::draw(&p).unwrap();
            assert_eq!(system.moons.len(), 3);
            // A planet its star despun keeps only moons that spiral in, and
            // an Earth-sized one only light ones.
            if crate::spin::despun(&crate::spin::star_info(&p)) {
                despun += 1;
                assert!(
                    system
                        .moons
                        .iter()
                        .all(|m| m.migration == crate::moons::Migration::Decaying),
                    "seed {seed}"
                );
                if p.mass_earth() < 2.0 {
                    assert!(
                        system.moons.iter().all(|m| m.mass_ratio < 1e-6),
                        "seed {seed}"
                    );
                }
            }
        }
        assert!(despun > 0, "no planet despun by its star");
        // A planet locked to its star in the habitable zone of a red dwarf
        // keeps a moonlet light enough to outlast the system (Barnes &
        // O'Brien 2002); one in a close orbit has no room for any.
        let locked = |orbit_au: Option<f32>, hz_position: Option<f32>| WorldRequest {
            mass_earth: Some(1.0),
            star: Some("M5V".into()),
            orbit_au,
            hz_position,
            eccentricity: Some(0.0),
            moons: Some(crate::moons::MoonsMode::Count(1)),
            ..named(None, 1)
        };
        let p = resolve(&locked(None, Some(0.5))).unwrap().params;
        let moons = crate::moons::MoonSystem::draw(&p).unwrap().moons;
        assert!(moons.len() == 1 && moons[0].mass_ratio < 1e-6, "{moons:?}");
        assert!(matches!(
            resolve(&locked(Some(0.006), None)),
            Err(RequestError::NoRoomForMoons { count: 1 })
        ));
        let too_many = WorldRequest {
            moons: Some(crate::moons::MoonsMode::Count(9)),
            ..Default::default()
        };
        assert!(matches!(
            resolve(&too_many),
            Err(RequestError::OutOfRange { field: "moons", .. })
        ));
        // Moons and a giant's moon cannot both be asked for; moons asked for
        // keep a drawn world from becoming one.
        let both = WorldRequest {
            mass_earth: Some(1.0),
            moons: Some(crate::moons::MoonsMode::Count(2)),
            host: Some(crate::exomoon::HostMode::Giant),
            ..named(None, 1)
        };
        assert!(matches!(resolve(&both), Err(RequestError::MoonsOfAMoon)));
        for seed in 0..300 {
            let p = resolve(&WorldRequest {
                mass_earth: Some(1.0),
                moons: Some(crate::moons::MoonsMode::Count(1)),
                ..named(None, seed)
            })
            .unwrap()
            .params;
            assert_eq!(p.host, crate::exomoon::HostMode::None);
        }
    }

    #[test]
    fn a_giants_moon_takes_its_volcanism_from_its_final_orbit() {
        // The giants may force the eccentricity after the host is known:
        // the volcanism follows the tides of the final orbit.
        for seed in 0..40 {
            let p = resolve(&WorldRequest {
                mass_earth: Some(1.0),
                host: Some(crate::exomoon::HostMode::Giant),
                metallicity: Some(0.5),
                ..named(None, seed)
            })
            .unwrap()
            .params;
            let host = crate::exomoon::HostGiant::draw(&p).expect("a giant's moon");
            assert!(
                p.volcanic_intensity >= host.volcanism(),
                "seed {seed}: {} < {}",
                p.volcanic_intensity,
                host.volcanism()
            );
            assert_eq!(p.moons, crate::moons::MoonsMode::None);
        }
    }

    #[test]
    fn fields_a_planet_has_no_use_for_are_reported() {
        let giant = resolve(&WorldRequest {
            host: Some(crate::exomoon::HostMode::None),
            ..named(Some("jupiter-like"), 3)
        })
        .unwrap();
        assert_eq!(giant.ignored, ["host"]);
        let lone = resolve(&WorldRequest {
            alone: Some(true),
            giants: Some(GiantsMode::Auto),
            ..named(Some("earth-like"), 3)
        })
        .unwrap();
        assert_eq!(lone.ignored, ["giants"]);
    }

    #[test]
    fn a_hot_giant_keeps_no_moons_even_when_asked() {
        let hot = WorldRequest {
            // Set, so the giant is hot whatever the draws (about 1 400 K).
            star: Some("G2V".into()),
            orbit_au: Some(0.04),
            moons: Some(crate::moons::MoonsMode::Count(2)),
            ..named(Some("hot-jupiter"), 2)
        };
        assert!(matches!(
            resolve(&hot),
            Err(RequestError::HotGiantMoons { count: 2 })
        ));
    }

    #[test]
    fn a_world_its_giants_make_eccentric_replays_its_effects_from_its_pinned_request() {
        // Moons left to the seed and moons asked for: both see the orbit.
        let requests = [
            WorldRequest {
                mass_earth: Some(1.0),
                ..named(None, 0)
            },
            WorldRequest {
                mass_earth: Some(1.0),
                moons: Some(crate::moons::MoonsMode::Count(2)),
                ..named(None, 0)
            },
        ];
        for request in requests {
            let mut forced = 0;
            for draw in 0..400u32 {
                let request = WorldRequest {
                    seed: Some(draw),
                    ..request.clone()
                };
                let resolve = |request: &WorldRequest| {
                    request.resolve(&mut StdRng::seed_from_u64(draw.into()))
                };
                let Ok(resolved) = resolve(&request) else {
                    continue;
                };
                // The same draws without giants: the eccentricity before
                // they force it.
                let alone = WorldRequest {
                    giants: Some(GiantsMode::None),
                    ..request.clone()
                };
                let Ok(free) = resolve(&alone) else {
                    continue;
                };
                if (free.params.star, free.params.orbit_au)
                    != (resolved.params.star, resolved.params.orbit_au)
                    || free.params.eccentricity >= resolved.params.eccentricity
                {
                    continue;
                }
                forced += 1;
                let p = &resolved.params;
                let pinned = request.pinned(p, Some(resolved.model()));
                let again = pinned
                    .resolve(&mut StdRng::seed_from_u64(u64::from(draw) + 1000))
                    .unwrap_or_else(|e| panic!("{pinned:?}: {e}"));
                assert_eq!(again.model(), resolved.model(), "{pinned:?}");
                assert_eq!(
                    crate::spin::star_info(&again.params),
                    crate::spin::star_info(p),
                    "{pinned:?}"
                );
                assert_eq!(
                    serde_json::to_value(&again.params).unwrap(),
                    serde_json::to_value(p).unwrap(),
                    "{pinned:?}"
                );
            }
            assert!(forced >= 3, "{request:?}: only {forced} forced draws");
        }
    }
}
