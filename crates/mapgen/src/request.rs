//! Generation requests: what a user asks for, before presets and random draws
//! turn it into complete [`WorldParams`].
//!
//! Shared by the command-line tool and the web server, so both accept exactly
//! the same parameters, ranges and defaults.
use crate::preset::{ParamDefault, Preset, PresetError, built_in_names};
use crate::world::WorldParams;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::fmt;

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
/// Axial tilt in degrees.
pub const AXIAL_TILT_DEG: Limits = Limits {
    min: 0.0,
    max: 90.0,
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
/// Eccentricity of the planet's orbit: 0 circular; beyond 0.9 the planet
/// would graze its star or leave its habitable zone for most of its year.
pub const ECCENTRICITY: Limits = Limits { min: 0.0, max: 0.9 };
/// Longitude of the perihelion, degrees from the spring equinox.
pub const PERIHELION_LONGITUDE_DEG: Limits = Limits {
    min: 0.0,
    max: 360.0,
};
/// Mass of a giant planet, Earth masses: from a small ice giant to the
/// deuterium-burning limit (13 Jupiter masses).
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

/// Generator defaults for parameters that neither the request nor the
/// preset sets.
pub mod fallback {
    use crate::preset::ParamDefault;
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
    pub const WIDTH: u32 = 1920;
    pub const HEIGHT: u32 = 1080;
}

/// A request to generate a world. `None` fields come from the preset when it
/// sets them, otherwise from [`fallback`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
pub struct WorldRequest {
    /// Built-in preset name or preset file path; `None` picks a random built-in.
    pub planet: Option<String>,
    pub seed: Option<u32>,
    pub sea_level: Option<f32>,
    /// Share of the surface that is land, by area, in `[0, 1]`: the sea
    /// level is computed to match it. Exclusive with `sea_level`.
    pub land_fraction: Option<f32>,
    pub volcanic: Option<f32>,
    pub circumference_km: Option<f32>,
    pub axial_tilt_deg: Option<f32>,
    pub erosion: Option<f32>,
    /// Surface pressure in bar.
    pub pressure_bar: Option<f32>,
    /// Age of the oldest surviving surface, in Ga.
    pub surface_age_ga: Option<f32>,
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
    /// Mass of a giant planet (giant presets only), Earth masses, in
    /// `[5, 4131]`; drawn in the preset's range when left out.
    pub mass_earth: Option<f32>,
    /// `auto` (rarely the moon of a giant, as the seed draws, the default),
    /// `giant` (always) or `none`.
    pub host: Option<crate::exomoon::HostMode>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Why a request was rejected.
#[derive(Debug)]
pub enum RequestError {
    /// A parameter is outside its [`Limits`].
    OutOfRange {
        field: &'static str,
        limits: Limits,
    },
    /// The map size is outside [`MAP_SIZE`].
    BadSize {
        field: &'static str,
        value: u32,
    },
    /// Both `sea_level` and `land_fraction` are set.
    SeaLevelAndLandFraction,
    /// The star is not a spectral type the model knows.
    BadStar(String),
    /// Both `orbit_au` and `hz_position` are set.
    OrbitAndPosition,
    /// The moons asked for find no stable orbits around the planet: it is
    /// too close to its star (a Hill sphere too small, or despun by its
    /// star's tides with no moon light enough to last), and its star and
    /// orbit were set (otherwise they are drawn again).
    NoRoomForMoons {
        count: u8,
    },
    /// Moons asked for on the moon of a giant (`host: giant`), which keeps
    /// none of its own: its Hill sphere is too small.
    MoonsOfAMoon,
    /// Moons asked for on a giant too close to its star to keep that many:
    /// a hot giant keeps none, one its star has locked only light ones, and
    /// a small Hill sphere fewer.
    HotGiantMoons {
        count: u8,
    },
    Preset(PresetError),
    /// More planets than a system holds.
    TooManyPlanets {
        asked: u32,
        max: u32,
    },
    /// More planets of a type than its band of distances holds at the
    /// spacing that keeps them stable.
    ZoneFull {
        body_type: String,
        asked: u32,
        fits: u32,
    },
    /// A type the star leaves no room for.
    TypeImpossible {
        body_type: String,
    },
    /// A planet type that is not one of `system::PLANET_TYPES`.
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
            RequestError::SeaLevelAndLandFraction => {
                write!(f, "set sea_level or land_fraction, not both")
            }
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
            RequestError::Preset(e) => e.fmt(f),
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

impl From<PresetError> for RequestError {
    fn from(e: PresetError) -> Self {
        RequestError::Preset(e)
    }
}

/// A resolved request: complete parameters plus the planet spec to pass back
/// to `--planet` to regenerate the same world.
pub struct Resolved {
    pub params: WorldParams,
    pub planet_spec: String,
    /// Fields of the request the planet has no use for, ignored: a giant
    /// has no ground (`circumference_km`, `erosion`, `volcanic`,
    /// `land_fraction`), no giant neighbours and is nobody's moon (`giants`,
    /// `host`); a rocky world has no giant's mass (`mass_earth`).
    pub ignored: Vec<&'static str>,
}

impl WorldRequest {
    /// Checks every explicit value against its limits.
    pub fn validate(&self) -> Result<(), RequestError> {
        if self.sea_level.is_some() && self.land_fraction.is_some() {
            return Err(RequestError::SeaLevelAndLandFraction);
        }
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
            ("sea_level", self.sea_level, SEA_LEVEL),
            ("land_fraction", self.land_fraction, LAND_FRACTION),
            ("volcanic", self.volcanic, VOLCANIC),
            ("circumference_km", self.circumference_km, CIRCUMFERENCE_KM),
            ("axial_tilt_deg", self.axial_tilt_deg, AXIAL_TILT_DEG),
            ("erosion", self.erosion, EROSION),
            ("pressure_bar", self.pressure_bar, PRESSURE_BAR),
            ("surface_age_ga", self.surface_age_ga, SURFACE_AGE_GA),
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
            ("mass_earth", self.mass_earth, GIANT_MASS),
        ] {
            if let Some(v) = value
                && !limits.contains(v)
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

    /// Validates the request, loads its preset and fills every missing value
    /// from the preset defaults, the generator fallbacks or random draws.
    pub fn resolve(&self, rng: &mut impl RngExt) -> Result<Resolved, RequestError> {
        self.validate()?;
        let planet_spec = match &self.planet {
            Some(spec) => spec.clone(),
            None => {
                let names = built_in_names();
                names[rng.random_range(0..crate::preset::DRAWN)].to_string()
            }
        };
        let planet = Preset::load(&planet_spec)?;
        self.resolve_with_preset(planet, planet_spec, rng)
    }

    /// Like [`resolve`](Self::resolve) with an already loaded preset, whose
    /// spec is `planet_spec`; `self.planet` is ignored.
    pub fn resolve_with_preset(
        &self,
        planet: Preset,
        planet_spec: String,
        rng: &mut impl RngExt,
    ) -> Result<Resolved, RequestError> {
        self.validate()?;
        let d = planet.defaults.clone();
        // Draw order matters for reproducibility with a seeded RNG: keep it stable.
        // An explicit value wins over the preset, a sea level over a land
        // fraction at the same level (a preset never sets both).
        let (sea_level, land_fraction) = match (self.sea_level, self.land_fraction) {
            (Some(level), _) => (level, None),
            (None, Some(fraction)) => (0.0, Some(fraction)),
            (None, None) => match (d.sea_level, d.land_fraction) {
                (None, Some(fraction)) => (0.0, Some(fraction.pick(rng))),
                (level, _) => (level.unwrap_or(fallback::SEA_LEVEL).pick(rng), None),
            },
        };
        let mut pick = |value: Option<f32>, preset: Option<ParamDefault>, fb: ParamDefault| {
            value.unwrap_or_else(|| preset.unwrap_or(fb).pick(rng))
        };
        let volcanic_intensity = pick(self.volcanic, d.volcanic, fallback::VOLCANIC);
        let circumference_km = pick(
            self.circumference_km,
            d.circumference,
            fallback::CIRCUMFERENCE_KM,
        );
        let tilt_drawn = self.axial_tilt_deg.is_none() && d.axial_tilt.is_none();
        let axial_tilt_deg = pick(self.axial_tilt_deg, d.axial_tilt, fallback::AXIAL_TILT_DEG);
        let erosion_set = self.erosion.is_some() || d.erosion.is_some();
        let erosion = pick(self.erosion, d.erosion, fallback::EROSION);
        // Drawn after the others, so their draws do not change.
        let pressure_bar = pick(self.pressure_bar, d.pressure, fallback::PRESSURE_BAR);
        // Without liquid water, rain carves nothing unless asked for (a
        // planet that was wetter once).
        let erosion = if !erosion_set && pressure_bar < crate::atmosphere::LIQUID_WATER_BAR {
            0.0
        } else {
            erosion
        };
        let seed = self.seed.unwrap_or_else(|| rng.random());
        // Drawn last, so the draws above (seed included) do not change.
        let surface_age_ga = self
            .surface_age_ga
            .unwrap_or_else(|| d.surface_age.unwrap_or(fallback::SURFACE_AGE_GA).pick(rng));
        // The star, then the orbit, drawn after everything else (above).
        // A giant's mass, before the star and orbit (which the moons may
        // draw again).
        let giant_mass_earth = planet
            .giant
            .as_ref()
            .map(|g| self.mass_earth.unwrap_or_else(|| g.mass.pick(rng)));
        let star_spec = planet.star.clone();
        let s = &star_spec;
        let period_days = planet.giant.as_ref().and_then(|g| g.period_days);
        let star_set = self.star.is_some() || s.kind.is_some();
        let orbit_set =
            self.orbit_au.is_some() || self.hz_position.is_some() || s.orbit_au.is_some();
        let mut draw_orbit = || -> Result<(f32, f32, f32), RequestError> {
            let star = match self.star.as_deref().or(s.kind.as_deref()) {
                Some(kind) => crate::star::parse_type(kind).map_err(RequestError::BadStar)?,
                None => crate::star::draw_type(rng),
            };
            let orbit_au = match (self.orbit_au, self.hz_position, s.orbit_au) {
                (Some(au), _, _) => au,
                (None, Some(position), _) => crate::star::orbit_at(star, position),
                (None, None, Some(au)) => au,
                (None, None, None) => match period_days {
                    // A hot giant is placed by its period (Kepler's third law).
                    Some(period) => {
                        let years = period.pick(rng) / 365.25;
                        (crate::star::values(star).mass * years * years).cbrt()
                    }
                    None => crate::star::orbit_at(
                        star,
                        s.position.unwrap_or(fallback::HZ_POSITION).pick(rng),
                    ),
                },
            };
            // The eccentricity last; the star's tides circularise the
            // closest orbits (a planet they despin on an eccentric orbit
            // keeps it, in the 3:2 resonance: see `crate::spin`).
            let eccentricity = match (self.eccentricity, s.eccentricity) {
                (Some(e), _) => e,
                (None, Some(e)) => e.pick(rng),
                (None, None) => {
                    let drawn = crate::star::draw_eccentricity(rng);
                    let at = crate::star::StarInfo::new(star, orbit_au, 0.0);
                    let circular = match giant_mass_earth {
                        Some(mass) => crate::spin::giant_circularised(
                            &at,
                            mass,
                            crate::giants::giant_radius(mass, at.equilibrium_k),
                        ),
                        None => crate::spin::circularised(&at),
                    };
                    if circular { 0.0 } else { drawn }
                }
            };
            Ok((star, orbit_au, eccentricity))
        };
        let (star, orbit_au, eccentricity) = draw_orbit()?;
        // Not drawn from the main stream: the values above keep their draws.
        let metallicity = self
            .metallicity
            .or(s.metallicity)
            .unwrap_or_else(|| crate::giants::draw_metallicity(seed));
        let eccentricity_drawn = self.eccentricity.is_none() && s.eccentricity.is_none();
        // A giant: its radius from its mass and the heat it gets; no ground,
        // so no erosion, volcanoes, moons of the rocky kind or giant
        // neighbours (not modelled for a giant).
        let giant = giant_mass_earth.is_some();
        let ignored: Vec<&'static str> = if giant {
            [
                ("circumference_km", self.circumference_km.is_some()),
                ("erosion", self.erosion.is_some()),
                ("volcanic", self.volcanic.is_some()),
                ("land_fraction", self.land_fraction.is_some()),
                ("giants", self.giants.is_some()),
                ("host", self.host.is_some()),
            ]
            .into_iter()
            .filter_map(|(field, set)| set.then_some(field))
            .collect()
        } else if self.mass_earth.is_some() {
            vec!["mass_earth"]
        } else {
            Vec::new()
        };
        let circumference_km = match giant_mass_earth {
            Some(mass) => {
                let at = crate::star::StarInfo::new(star, orbit_au, eccentricity).equilibrium_k;
                crate::giants::giant_radius(mass, at) * 40_075.0
            }
            None => circumference_km,
        };
        let mut params = WorldParams {
            width: self.width.unwrap_or(fallback::WIDTH),
            height: self.height.unwrap_or(fallback::HEIGHT),
            seed,
            planet,
            sea_level,
            volcanic_intensity: if giant { 0.0 } else { volcanic_intensity },
            circumference_km,
            axial_tilt_deg,
            erosion: if giant { 0.0 } else { erosion },
            pressure_bar,
            surface_age_ga,
            star,
            orbit_au,
            eccentricity,
            drift_ma: self.drift_ma.unwrap_or(0.0),
            // A giant has moons of its own kind (see `MoonSystem::draw`).
            moons: self.moons.unwrap_or_default(),
            shape: None,
            metallicity,
            giants: if giant {
                crate::giants::GiantsMode::None
            } else {
                self.giants.unwrap_or_default()
            },
            giant_mass_earth,
            host: if giant {
                crate::exomoon::HostMode::None
            } else {
                self.host.unwrap_or_default()
            },
            given_rings: None,
            impact_factor: None,
            locked_to_star: None,
            gravity: None,
            // Drawn from its own stream: the other draws keep their values.
            perihelion_longitude_deg: Some(self.perihelion_longitude_deg.map_or_else(
                || {
                    use rand::SeedableRng;
                    let stream = crate::noise::stream_seed(seed, crate::noise::stream::PERIHELION);
                    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(stream));
                    rng.random::<f32>() * 360.0
                },
                |v| v.rem_euclid(360.0),
            )),
        };
        // Moons asked for: the world is not a giant's moon, which keeps none
        // (an error when that is asked for too).
        if params.moons.count().is_some() {
            match params.host {
                crate::exomoon::HostMode::Giant => return Err(RequestError::MoonsOfAMoon),
                crate::exomoon::HostMode::Auto => params.host = crate::exomoon::HostMode::None,
                crate::exomoon::HostMode::None => {}
            }
        }
        // The moon of a giant: no moons of its own (the giant's tides take
        // them). Whether it is one depends on the seed and request only, not
        // on the orbit drawn below.
        if crate::exomoon::HostGiant::draw(&params).is_some() {
            params.moons = crate::moons::MoonsMode::None;
        }
        // What the moons do to their planet: a large one keeps a drawn tilt
        // in the usual range (a free one ranges wider), and the tides
        // (moons and star) widen or narrow the tidal flats. Applied to the
        // parameters, so `world.json` (its tilt, its preset's shore width)
        // explains the world as generated.
        // Moons asked for need room: a Hill sphere wide enough, and on a
        // planet its star despun, moons light enough to outlast its tides.
        // A drawn star and orbit are drawn again until they leave it (the
        // generation stays coherent); set ones that do not are an error.
        if let Some(n) = params.moons.count() {
            let room = |p: &WorldParams| {
                crate::moons::MoonSystem::draw(p).is_some_and(|m| m.moons.len() >= usize::from(n))
            };
            let mut attempts = 0;
            while !room(&params) {
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
                let (star, orbit_au, eccentricity) = draw_orbit()?;
                params.star = star;
                params.orbit_au = orbit_au;
                params.eccentricity = eccentricity;
                attempts += 1;
            }
        }
        let system = crate::moons::MoonSystem::draw(&params);
        let effects = crate::moons::PlanetEffects::of(&params, system.as_ref());
        if tilt_drawn && let ParamDefault::Range(range) = fallback::AXIAL_TILT_DEG {
            params.axial_tilt_deg = effects.tilt(params.axial_tilt_deg, (range[0], range[1]));
        }
        // The tides that despin a planet also stand it upright, its axis
        // along its orbit's (see `crate::spin::upright`).
        if tilt_drawn && !giant {
            params.axial_tilt_deg = crate::spin::upright(&params, params.axial_tilt_deg);
        }
        params.planet.rules.shore_width_km *= effects.shore_factor;
        // Eccentric giants force an eccentricity on the world: a drawn one is
        // at least that (the giants are placed around the final orbit, which
        // the moons may have drawn again).
        let circular = crate::spin::circularised(&crate::star::StarInfo::new(
            params.star,
            params.orbit_au,
            0.0,
        ));
        if eccentricity_drawn && !circular {
            for _ in 0..3 {
                let forced = crate::giants::GiantSystem::draw(&params)
                    .map_or(0.0, |g| g.forced_eccentricity);
                if forced <= params.eccentricity {
                    break;
                }
                params.eccentricity = forced.min(ECCENTRICITY.max);
            }
        }
        // Tidal heat raises a drawn volcanism, from the giant as the final
        // orbit places it (the giants may have forced its eccentricity), so
        // that `host.tidal_flux` in `world.json` explains it.
        if self.volcanic.is_none()
            && let Some(host) = crate::exomoon::HostGiant::draw(&params)
        {
            params.volcanic_intensity = params.volcanic_intensity.max(host.volcanism());
        }
        // The world decides whether it has plates (its size, water and
        // star), applied to its preset so that `world.json` tells the
        // regime it was generated with.
        params.planet.geology.regime = crate::geology::regime_of(&params);
        if let Some(fraction) = land_fraction
            && !giant
        {
            params.sea_level = crate::generation::sea_level_for_land_fraction(&params, fraction);
        }
        Ok(Resolved {
            params,
            planet_spec,
            ignored,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_star_and_orbit_come_from_the_request_then_the_preset() {
        use crate::star::{SUN, StarInfo, parse_type};
        let resolve = |request: WorldRequest| request.resolve(&mut rand::rng()).unwrap().params;
        // The preset's star and distance (Mars around the Sun).
        let mars = resolve(WorldRequest {
            planet: Some(
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../presets/examples/mars-like.toml"
                )
                .into(),
            ),
            ..Default::default()
        });
        assert_eq!((mars.star, mars.orbit_au), (SUN, 1.524));
        // A request's star and position win; the position gives the distance.
        let red = resolve(WorldRequest {
            planet: Some("terran".into()),
            star: Some("M4V".into()),
            hz_position: Some(0.5),
            // The position is that of a circular orbit.
            eccentricity: Some(0.0),
            ..Default::default()
        });
        assert_eq!(red.star, parse_type("M4V").unwrap());
        let info = StarInfo::new(red.star, red.orbit_au, red.eccentricity);
        assert!(
            (info.hz_position - 0.5).abs() < 1e-3,
            "{}",
            info.hz_position
        );
        // A terran world lands inside the habitable zone of whatever star.
        for _ in 0..20 {
            let p = resolve(WorldRequest {
                planet: Some("terran".into()),
                ..Default::default()
            });
            let position = StarInfo::new(p.star, p.orbit_au, 0.0).hz_position;
            assert!((0.04..0.76).contains(&position), "{position}");
        }
        // Bad stars and both orbits at once are refused.
        let bad = WorldRequest {
            star: Some("Q1".into()),
            ..Default::default()
        };
        assert!(matches!(
            bad.resolve(&mut rand::rng()),
            Err(RequestError::BadStar(_))
        ));
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
    fn explicit_values_are_kept() {
        let request = WorldRequest {
            planet: Some("frozen".into()),
            seed: Some(7),
            sea_level: Some(0.2),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        let resolved = request.resolve(&mut rand::rng()).unwrap();
        assert_eq!(resolved.params.seed, 7);
        assert_eq!(resolved.params.sea_level, 0.2);
        assert_eq!(resolved.params.planet.name, "frozen");
        assert_eq!(resolved.params.erosion, 1.0);
        assert_eq!(resolved.planet_spec, "frozen");

        // The same with the preset already loaded: its defaults apply, the
        // request's planet is ignored.
        let preset = Preset::load("caustic").unwrap();
        let resolved = request
            .resolve_with_preset(preset, "caustic".into(), &mut rand::rng())
            .unwrap();
        assert_eq!(resolved.params.planet.name, "caustic");
        assert_eq!(resolved.planet_spec, "caustic");
        assert_eq!(resolved.params.sea_level, 0.2);
    }

    #[test]
    fn land_fraction_sets_the_sea_level() {
        let request = |fraction| WorldRequest {
            planet: Some("terran".into()),
            seed: Some(3),
            land_fraction: Some(fraction),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        let level = |fraction| {
            request(fraction)
                .resolve(&mut rand::rng())
                .unwrap()
                .params
                .sea_level
        };
        // More land needs a lower sea.
        assert!(level(0.6) < level(0.3));
        assert!(level(0.3) < level(0.1));
        // And the world gets the land asked for.
        for fraction in [0.1, 0.29, 0.6] {
            let mut params = WorldRequest {
                width: Some(256),
                height: Some(128),
                ..request(fraction)
            }
            .resolve(&mut rand::rng())
            .unwrap()
            .params;
            params.erosion = 0.0;
            let world = crate::generation::generate(&params).world;
            let waterline = params.sea_level + params.planet.rules.waterline;
            let grid = params.grid();
            let (mut land, mut all) = (0.0, 0.0);
            for (i, &e) in world.elevation.iter().enumerate() {
                let w = grid.latitude((i / world.width as usize) as u32).cos();
                all += w;
                if e > waterline {
                    land += w;
                }
            }
            let share = land / all;
            assert!(
                (share - f64::from(fraction)).abs() < 0.03,
                "{fraction}: {share}"
            );
        }
    }

    #[test]
    fn worlds_without_liquid_water_erode_only_when_asked() {
        let resolve = |planet: &str, pressure: f32, erosion: Option<f32>| {
            WorldRequest {
                planet: Some(planet.into()),
                pressure_bar: Some(pressure),
                erosion,
                width: Some(16),
                height: Some(8),
                ..Default::default()
            }
            .resolve(&mut rand::rng())
            .unwrap()
            .params
            .erosion
        };
        assert_eq!(resolve("terran", 1.0, None), 1.0);
        assert_eq!(resolve("terran", 0.005, None), 0.0);
        assert_eq!(resolve("terran", 0.005, Some(2.0)), 2.0);
        // Barren worlds set their erosion: valleys from a wetter past.
        assert_eq!(resolve("barren", 0.005, None), 1.0);
    }

    #[test]
    fn surface_age_comes_from_the_request_then_the_preset() {
        let age = |planet: &str, surface_age_ga| {
            WorldRequest {
                planet: Some(planet.into()),
                surface_age_ga,
                width: Some(16),
                height: Some(8),
                ..Default::default()
            }
            .resolve(&mut rand::rng())
            .unwrap()
            .params
            .surface_age_ga
        };
        assert_eq!(age("barren", Some(1.5)), 1.5);
        assert!((3.0..4.2).contains(&age("barren", None)));
    }

    #[test]
    fn sea_level_and_land_fraction_are_exclusive() {
        let request = WorldRequest {
            sea_level: Some(0.2),
            land_fraction: Some(0.3),
            ..Default::default()
        };
        assert!(matches!(
            request.validate(),
            Err(RequestError::SeaLevelAndLandFraction)
        ));
    }

    #[test]
    fn missing_values_are_drawn_within_fallback_ranges() {
        let resolved = WorldRequest::default().resolve(&mut rand::rng()).unwrap();
        let p = resolved.params;
        assert!((0.10..0.40).contains(&p.sea_level));
        // 5 to 40°, or 0 to 85° without a moon to steady it.
        assert!((0.0..=85.0).contains(&p.axial_tilt_deg));
        // The planet is random: its preset may set its own range.
        assert!(SURFACE_AGE_GA.contains(p.surface_age_ga));
        assert_eq!((p.width, p.height), (1920, 1080));
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        let bad = [
            WorldRequest {
                sea_level: Some(2.0),
                ..Default::default()
            },
            WorldRequest {
                width: Some(1),
                ..Default::default()
            },
            WorldRequest {
                surface_age_ga: Some(5.0),
                ..Default::default()
            },
            WorldRequest {
                planet: Some("jupiter".into()),
                ..Default::default()
            },
        ];
        for request in bad {
            assert!(request.resolve(&mut rand::rng()).is_err(), "{request:?}");
        }
    }

    #[test]
    fn moons_steady_the_drawn_tilt_and_set_the_shores() {
        let (mut free, mut steady) = (0, 0);
        for seed in 0..400 {
            let request = WorldRequest {
                planet: Some("terran".into()),
                seed: Some(seed),
                circumference_km: Some(40_075.0),
                star: Some("G2V".into()),
                orbit_au: Some(1.0),
                eccentricity: Some(0.0),
                // Not the moon of a giant (which has no moons of its own).
                host: Some(crate::exomoon::HostMode::None),
                width: Some(64),
                height: Some(32),
                ..Default::default()
            };
            let p = request.resolve(&mut rand::rng()).unwrap().params;
            let system = crate::moons::MoonSystem::draw(&p).unwrap();
            let effects = crate::moons::PlanetEffects::of(&p, Some(&system));
            assert!((p.planet.rules.shore_width_km - 40.0 * effects.shore_factor).abs() < 1e-3);
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
            planet: Some("terran".into()),
            seed: Some(1),
            axial_tilt_deg: Some(55.0),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        assert_eq!(
            asked
                .resolve(&mut rand::rng())
                .unwrap()
                .params
                .axial_tilt_deg,
            55.0
        );
    }

    #[test]
    fn moons_asked_for_get_a_star_and_orbit_with_room_for_them() {
        use rand::SeedableRng;
        let mut despun = 0;
        for seed in 0..40 {
            let request = WorldRequest {
                planet: Some("terran".into()),
                seed: Some(seed),
                moons: Some(crate::moons::MoonsMode::Count(3)),
                width: Some(64),
                height: Some(32),
                ..Default::default()
            };
            // The draws the seed leaves open come from a seeded generator,
            // so that the test checks the same worlds on every run.
            let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed));
            let p = request.resolve(&mut rng).unwrap().params;
            let system = crate::moons::MoonSystem::draw(&p).unwrap();
            assert_eq!(system.moons.len(), 3);
            // A planet its star despun keeps only moons that spiral in, and
            // an Earth-sized one only light ones. Barnes & O'Brien's bound
            // grows as the planet's mass to the 8/3 over its radius to the
            // fifth: a super-Earth of 11 Earth masses at 0.46 AU keeps moons
            // of a few thousandths of its mass.
            if crate::spin::despun(&crate::star::StarInfo::new(
                p.star,
                p.orbit_au,
                p.eccentricity,
            )) {
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
            planet: Some("terran".into()),
            seed: Some(1),
            star: Some("M5V".into()),
            orbit_au,
            hz_position,
            eccentricity: Some(0.0),
            moons: Some(crate::moons::MoonsMode::Count(1)),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        let p = locked(None, Some(0.5))
            .resolve(&mut rand::rng())
            .unwrap()
            .params;
        let moons = crate::moons::MoonSystem::draw(&p).unwrap().moons;
        assert!(moons.len() == 1 && moons[0].mass_ratio < 1e-6, "{moons:?}");
        assert!(matches!(
            locked(Some(0.006), None).resolve(&mut rand::rng()),
            Err(RequestError::NoRoomForMoons { count: 1 })
        ));
        let too_many = WorldRequest {
            moons: Some(crate::moons::MoonsMode::Count(9)),
            ..Default::default()
        };
        assert!(matches!(
            too_many.resolve(&mut rand::rng()),
            Err(RequestError::OutOfRange { field: "moons", .. })
        ));
        // Moons and a giant's moon cannot both be asked for; moons asked for
        // keep a drawn world from becoming one.
        let both = WorldRequest {
            moons: Some(crate::moons::MoonsMode::Count(2)),
            host: Some(crate::exomoon::HostMode::Giant),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        assert!(matches!(
            both.resolve(&mut rand::rng()),
            Err(RequestError::MoonsOfAMoon)
        ));
        for seed in 0..300 {
            let p = WorldRequest {
                planet: Some("terran".into()),
                seed: Some(seed),
                moons: Some(crate::moons::MoonsMode::Count(1)),
                width: Some(64),
                height: Some(32),
                ..Default::default()
            }
            .resolve(&mut rand::rng())
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
            let p = WorldRequest {
                planet: Some("terran".into()),
                seed: Some(seed),
                host: Some(crate::exomoon::HostMode::Giant),
                metallicity: Some(0.5),
                width: Some(64),
                height: Some(32),
                ..Default::default()
            }
            .resolve(&mut rand::rng())
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
        let giant = WorldRequest {
            planet: Some("gas-giant".into()),
            seed: Some(3),
            erosion: Some(0.5),
            circumference_km: Some(30_000.0),
            host: Some(crate::exomoon::HostMode::None),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        }
        .resolve(&mut rand::rng())
        .unwrap();
        assert_eq!(giant.ignored, ["circumference_km", "erosion", "host"]);
        let rocky = WorldRequest {
            planet: Some("terran".into()),
            seed: Some(3),
            mass_earth: Some(300.0),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        }
        .resolve(&mut rand::rng())
        .unwrap();
        assert_eq!(rocky.ignored, ["mass_earth"]);
    }

    #[test]
    fn a_hot_giant_keeps_no_moons_even_when_asked() {
        let hot = WorldRequest {
            planet: Some("hot-jupiter".into()),
            seed: Some(2),
            // Set, so the giant is hot whatever the draws (about 1 400 K).
            star: Some("G2V".into()),
            orbit_au: Some(0.04),
            moons: Some(crate::moons::MoonsMode::Count(2)),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        assert!(matches!(
            hot.resolve(&mut rand::rng()),
            Err(RequestError::HotGiantMoons { count: 2 })
        ));
    }
}
