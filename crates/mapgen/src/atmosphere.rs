//! The atmosphere of a planet type: how its sky looks and, from later
//! steps of the atmosphere work (`docs/analysis/07-atmospheres.md`), how it
//! shapes the climate and the clouds.
//!
//! The surface pressure is a generation parameter (`pressure_bar`), drawn
//! from the preset's `[defaults]` like the sea level; the rest comes from the
//! preset's `[atmosphere]` table:
//!
//! ```toml
//! [atmosphere]
//! rayleigh = [0.17, 0.41, 1.0]       # relative scattering per channel (sky colour)
//! haze = 0.1                         # aerosol density, 0 (clear) to 1 (opaque)
//! haze_colour = [1.0, 1.0, 1.0]      # aerosol tint
//! greenhouse = 0.0                   # added to temperature, °C
//! clouds = "water"                   # water | sulfuric | dust | none
//! cloud_cover = 0.0                  # shift of the cloud cover, -1 to 1
//! ```
//!
//! Every field is optional; a preset without the table gets Earth's.
use crate::geo::Grid;
use serde::{Deserialize, Serialize};

/// Below this surface pressure (bar) a world has no atmosphere to speak of:
/// no sky, no clouds.
pub const AIRLESS_BAR: f32 = 1e-4;

/// Below this surface pressure (bar) no water stays liquid: no rain, no
/// rivers or lakes, and no hydraulic erosion unless the preset asks for it
/// (valleys carved when the planet was wetter). Water's triple point is at
/// 0.006 bar.
pub const LIQUID_WATER_BAR: f32 = 0.01;

/// How much of Earth's equator-to-pole temperature difference a planet
/// keeps: dense air carries heat to the poles. 1 up to 1 bar, then
/// `1 / (1 + 0.1 (p − 1))`: 0.45 at 13 bar, 0.1 on Venus (92 bar). Thin
/// atmospheres keep Earth's contrast: their cold comes from the climate
/// shift.
pub fn heat_contrast(pressure_bar: f32) -> f32 {
    1.0 / (1.0 + 0.1 * (pressure_bar - 1.0).max(0.0))
}

/// Pulls each row temperature (°C) towards their area-weighted mean by
/// [`heat_contrast`]: the global mean does not move.
pub fn transport_heat(rows: &mut [f32], grid: &Grid, pressure_bar: f32) {
    let contrast = heat_contrast(pressure_bar);
    if contrast >= 1.0 {
        return;
    }
    let weights: Vec<f32> = (0..rows.len() as u32)
        .map(|y| grid.latitude(y).cos() as f32)
        .collect();
    let mean =
        rows.iter().zip(&weights).map(|(t, w)| t * w).sum::<f32>() / weights.iter().sum::<f32>();
    for t in rows {
        *t = mean + (*t - mean) * contrast;
    }
}

/// What clouds are made of, which sets where they form and how they look.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum CloudKind {
    /// Water droplets and ice: where air rises, rains and cools.
    #[default]
    Water,
    /// Sulfuric acid: a high deck in zonal bands (total on Venus).
    Sulfuric,
    /// Dust lifted by the wind: low, patchy, with regional storms.
    Dust,
    /// No clouds.
    None,
}

/// The sky and clouds of a planet type. See the module documentation.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Atmosphere {
    /// Rayleigh scattering of the gas per channel (red, green, blue),
    /// relative: the colour of a clear sky. Earth's air scatters as λ⁻⁴,
    /// about `[0.17, 0.41, 1.0]`.
    pub rayleigh: [f32; 3],
    /// Aerosol (Mie) density, from 0 (clear) to 1 (opaque haze).
    pub haze: f32,
    /// Tint of the aerosols, each channel in `[0, 1]`.
    pub haze_colour: [f32; 3],
    /// Greenhouse warming added to every temperature, °C.
    pub greenhouse: f32,
    /// What the clouds are made of.
    pub clouds: CloudKind,
    /// Shift of the cloud cover, from −1 (clear) to 1 (overcast).
    pub cloud_cover: f32,
}

impl Default for Atmosphere {
    /// Earth's.
    fn default() -> Self {
        Self {
            rayleigh: [0.17, 0.41, 1.0],
            haze: 0.1,
            haze_colour: [1.0, 1.0, 1.0],
            greenhouse: 0.0,
            clouds: CloudKind::Water,
            cloud_cover: 0.0,
        }
    }
}

impl Atmosphere {
    /// Checks every field is within its range.
    pub fn validate(&self) -> Result<(), String> {
        let within = |field: &str, v: f32, min: f32, max: f32| {
            if v.is_finite() && (min..=max).contains(&v) {
                Ok(())
            } else {
                Err(format!(
                    "atmosphere.{field}: must stay within [{min}, {max}]"
                ))
            }
        };
        for v in self.rayleigh {
            within("rayleigh", v, 0.0, 10.0)?;
        }
        within("haze", self.haze, 0.0, 1.0)?;
        for v in self.haze_colour {
            within("haze_colour", v, 0.0, 1.0)?;
        }
        within("greenhouse", self.greenhouse, -200.0, 600.0)?;
        within("cloud_cover", self.cloud_cover, -1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_air_evens_temperatures_out_and_keeps_their_mean() {
        let grid = Grid {
            width: 4,
            height: 90,
            circumference_km: 40_000.0,
        };
        let rows = crate::climate::row_temperatures(&grid, 23.44);
        let mean = |r: &[f32]| {
            let w: Vec<f32> = (0..r.len() as u32)
                .map(|y| grid.latitude(y).cos() as f32)
                .collect();
            r.iter().zip(&w).map(|(t, w)| t * w).sum::<f32>() / w.iter().sum::<f32>()
        };
        let mut earth = rows.clone();
        transport_heat(&mut earth, &grid, 1.0);
        assert_eq!(earth, rows);
        let mut venus = rows.clone();
        transport_heat(&mut venus, &grid, 92.0);
        let spread = |r: &[f32]| r[44] - r[0];
        assert!(spread(&venus) < 0.11 * spread(&rows));
        assert!((mean(&venus) - mean(&rows)).abs() < 1e-3);
    }

    #[test]
    fn earth_is_the_default_and_valid() {
        let earth = Atmosphere::default();
        assert_eq!(earth.clouds, CloudKind::Water);
        assert!(earth.validate().is_ok());
    }

    #[test]
    fn out_of_range_fields_are_rejected() {
        let bad = Atmosphere {
            haze: 2.0,
            ..Atmosphere::default()
        };
        assert!(bad.validate().unwrap_err().contains("haze"));
    }
}
