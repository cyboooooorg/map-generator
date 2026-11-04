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
/// Mass of a giant planet, Earth masses: from a small ice giant to the
/// deuterium-burning limit (13 Jupiter masses).
pub const GIANT_MASS: Limits = Limits {
    min: 5.0,
    max: 4_131.0,
};
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
    Preset(PresetError),
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
            RequestError::OrbitAndPosition => {
                write!(f, "set orbit_au or hz_position, not both")
            }
            RequestError::Preset(e) => e.fmt(f),
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
        let s = &planet.star;
        let star = match self.star.as_deref().or(s.kind.as_deref()) {
            Some(kind) => crate::star::parse_type(kind).map_err(RequestError::BadStar)?,
            None => crate::star::draw_type(rng),
        };
        let orbit_au = match (self.orbit_au, self.hz_position, s.orbit_au) {
            (Some(au), _, _) => au,
            (None, Some(position), _) => crate::star::orbit_at(star, position),
            (None, None, Some(au)) => au,
            (None, None, None) => match planet.giant.as_ref().and_then(|g| g.period_days) {
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
        // Not drawn from the main stream: the values above keep their draws.
        let metallicity = self
            .metallicity
            .or(s.metallicity)
            .unwrap_or_else(|| crate::giants::draw_metallicity(seed));
        let locked = crate::star::StarInfo::new(star, orbit_au, 0.0).tidally_locked;
        let eccentricity_drawn = self.eccentricity.is_none() && s.eccentricity.is_none();
        // The eccentricity last; tides circularise the orbits of the planets
        // they lock.
        let eccentricity = match (self.eccentricity, s.eccentricity) {
            (Some(e), _) => e,
            (None, Some(e)) => e.pick(rng),
            (None, None) => {
                let drawn = crate::star::draw_eccentricity(rng);
                if locked { 0.0 } else { drawn }
            }
        };
        // A giant: its mass (drawn last), its radius from the mass and the
        // heat it gets; no ground, so no erosion, volcanoes, moons of the
        // rocky kind or giant neighbours (not modelled for a giant).
        let giant_mass_earth = planet
            .giant
            .as_ref()
            .map(|g| self.mass_earth.unwrap_or_else(|| g.mass.pick(rng)));
        let giant = giant_mass_earth.is_some();
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
            moons: if giant {
                crate::moons::MoonsMode::None
            } else {
                self.moons.unwrap_or_default()
            },
            shape: None,
            metallicity,
            giants: if giant {
                crate::giants::GiantsMode::None
            } else {
                self.giants.unwrap_or_default()
            },
            giant_mass_earth,
        };
        // What the moons do to their planet: a large one keeps a drawn tilt
        // in the usual range (a free one ranges wider), and the tides
        // (moons and star) widen or narrow the tidal flats. Applied to the
        // parameters, so `world.json` (its tilt, its preset's shore width)
        // explains the world as generated.
        let system = crate::moons::MoonSystem::draw(&params);
        let effects = crate::moons::PlanetEffects::of(&params, system.as_ref());
        if tilt_drawn && let ParamDefault::Range(range) = fallback::AXIAL_TILT_DEG {
            params.axial_tilt_deg = effects.tilt(params.axial_tilt_deg, (range[0], range[1]));
        }
        params.planet.rules.shore_width_km *= effects.shore_factor;
        // Eccentric giants force an eccentricity on the world: a drawn one is
        // at least that (the giants are placed around the final orbit).
        if eccentricity_drawn && !locked {
            for _ in 0..3 {
                let forced = crate::giants::GiantSystem::draw(&params)
                    .map_or(0.0, |g| g.forced_eccentricity);
                if forced <= params.eccentricity {
                    break;
                }
                params.eccentricity = forced.min(ECCENTRICITY.max);
            }
        }
        if let Some(fraction) = land_fraction {
            params.sea_level = crate::generation::sea_level_for_land_fraction(&params, fraction);
        }
        Ok(Resolved {
            params,
            planet_spec,
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
        // 5 to 40°, or 0 to 60° without a moon to steady it.
        assert!((0.0..=60.0).contains(&p.axial_tilt_deg));
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
        // Most Earths have no large moon: their tilt ranges from 0 to 60°.
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
}
