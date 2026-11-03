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
        for (field, value, limits) in [
            ("sea_level", self.sea_level, SEA_LEVEL),
            ("land_fraction", self.land_fraction, LAND_FRACTION),
            ("volcanic", self.volcanic, VOLCANIC),
            ("circumference_km", self.circumference_km, CIRCUMFERENCE_KM),
            ("axial_tilt_deg", self.axial_tilt_deg, AXIAL_TILT_DEG),
            ("erosion", self.erosion, EROSION),
            ("pressure_bar", self.pressure_bar, PRESSURE_BAR),
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
                names[rng.random_range(0..names.len())].to_string()
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
        let mut params = WorldParams {
            width: self.width.unwrap_or(fallback::WIDTH),
            height: self.height.unwrap_or(fallback::HEIGHT),
            seed,
            planet,
            sea_level,
            volcanic_intensity,
            circumference_km,
            axial_tilt_deg,
            erosion,
            pressure_bar,
        };
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
        assert!((5.0..40.0).contains(&p.axial_tilt_deg));
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
                planet: Some("jupiter".into()),
                ..Default::default()
            },
        ];
        for request in bad {
            assert!(request.resolve(&mut rand::rng()).is_err(), "{request:?}");
        }
    }
}
