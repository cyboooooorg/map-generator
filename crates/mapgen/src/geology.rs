//! The geology of a planet type: its tectonic regime and the history of its
//! surface, which the impact craters of `docs/analysis/08-geology.md` will
//! follow.
//!
//! The age of the surface is a generation parameter (`surface_age_ga`),
//! drawn from the preset's `[defaults]` like the pressure; the rest comes
//! from the preset's `[geology]` table:
//!
//! ```toml
//! [geology]
//! regime = "stagnant"                # plates | stagnant | episodic | heat_pipe
//! wet_until = 3.5                    # Ga: end of the era of liquid water
//! ```
//!
//! Every field is optional; a preset without the table gets Earth's.
use crate::atmosphere::LIQUID_WATER_BAR;
use crate::request::SURFACE_AGE_GA;
use serde::{Deserialize, Serialize};

/// How the planet loses its internal heat, which shapes its surface.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Regime {
    /// Mobile lid: the lithosphere is broken into plates recycled by
    /// subduction (Earth).
    #[default]
    Plates,
    /// A single rigid plate; heat escapes by conduction and a few giant
    /// volcanoes (Mars, Mercury, the Moon).
    Stagnant,
    /// Stagnant for long spans, then overturned globally (Venus).
    Episodic,
    /// Volcanism so intense that lava resurfaces the planet continuously (Io).
    HeatPipe,
}

impl Regime {
    /// Human-readable name, e.g. "stagnant lid".
    pub fn label(self) -> &'static str {
        match self {
            Regime::Plates => "plate tectonics",
            Regime::Stagnant => "stagnant lid",
            Regime::Episodic => "episodic lid",
            Regime::HeatPipe => "heat pipe",
        }
    }
}

/// The geology of a planet type. See the module documentation.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Geology {
    /// Tectonic regime. Recorded for now: it does not shape the relief yet.
    pub regime: Regime,
    /// End of the era of liquid water, in Ga before present. 0 means still
    /// wet; a value at or above the surface age means the surface never saw
    /// liquid water (Mercury). Craters older than it will be worn by the
    /// hydraulic erosion, younger ones stay fresh.
    pub wet_until: f32,
}

impl Default for Geology {
    /// Earth's.
    fn default() -> Self {
        Self {
            regime: Regime::Plates,
            wet_until: 0.0,
        }
    }
}

impl Geology {
    /// Checks every field is within its range.
    pub fn validate(&self) -> Result<(), String> {
        let (min, max) = (SURFACE_AGE_GA.min, SURFACE_AGE_GA.max);
        if self.wet_until.is_finite() && (min..=max).contains(&self.wet_until) {
            Ok(())
        } else {
            Err(format!(
                "geology.wet_until: must stay within [{min}, {max}]"
            ))
        }
    }

    /// End of the wet era of a world of this surface pressure, in Ga: 0 for
    /// a world with liquid water today, whatever the preset says.
    pub fn wet_until_ga(&self, pressure_bar: f32) -> f32 {
        if pressure_bar >= LIQUID_WATER_BAR {
            0.0
        } else {
            self.wet_until
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worlds_with_liquid_water_are_still_wet() {
        let mars = Geology {
            regime: Regime::Stagnant,
            wet_until: 3.5,
        };
        assert_eq!(mars.wet_until_ga(0.006), 3.5);
        assert_eq!(mars.wet_until_ga(1.0), 0.0);
    }

    #[test]
    fn wet_until_stays_within_the_age_of_a_surface() {
        assert!(Geology::default().validate().is_ok());
        for bad in [-0.1, 5.0, f32::NAN] {
            let g = Geology {
                wet_until: bad,
                ..Geology::default()
            };
            assert!(g.validate().is_err(), "{bad}");
        }
    }
}
