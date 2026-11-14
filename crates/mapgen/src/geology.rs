//! The geology of a world: its tectonic regime and the history of its
//! surface, which the impact craters (`crate::craters`) and the plates
//! (`crate::plates`) follow (`docs/analysis/08-geology.md`).
//!
//! The age of the surface is a generation parameter (`surface_age_ga`);
//! the rest comes from the `[geology]` table of the world's look
//! ([`crate::look`]):
//!
//! ```toml
//! [geology]
//! regime = "stagnant"                # plates | stagnant | episodic | heat_pipe
//! wet_until = 3.5                    # Ga: end of the era of liquid water
//! maria = [3.9, 3.1]                 # Ga: when lava flooded large basins
//! ```
//!
//! Every field is optional; a look without the table gets Earth's.
//!
//! **The world decides plates.** A world's regime is the model's, or the look's
//! for a body whose surface the model does not derive; a world of a plate
//! look that cannot hold plates gets another ([`regime_of`]), so a
//! Mars-sized `terran` world has a stagnant lid.
use crate::atmosphere::LIQUID_WATER_BAR;
use crate::request::SURFACE_AGE_GA;
use crate::world::WorldParams;
use serde::{Deserialize, Serialize};

/// Smallest mass of a world with plate tectonics, Earth masses: the app's
/// choice between Mars (0.107, a stagnant lid) and Venus (0.815, Earth's
/// size; NASA fact sheets), where the science is split. Small planets cool
/// fast and their lids thicken: models with a damaged lithosphere favour
/// plates on larger planets (Foley, Bercovici & Landuyt 2012,
/// doi:10.1016/j.epsl.2012.03.028), and the tectonic activity of the
/// Solar System's bodies grows with the vigour of their mantle's
/// convection (Stern, Gerya & Tackley 2018, doi:10.1016/j.gsf.2017.06.004).
/// The geometric mean of Mars and Venus, about 0.72 Earth radii.
pub const PLATES_MIN_MASS_EARTH: f32 = 0.3;

/// The tectonic regime of the world `p`: its look's, except that a plate
/// look's world without what plates need has none.
///
/// - **Dry**: under [`LIQUID_WATER_BAR`], no water weakens its lithosphere,
///   a stagnant lid: the likelihood of plates "is controlled largely by the
///   presence of surface water" (Korenaga 2010,
///   doi:10.1088/2041-8205/725/1/L43; Foley 2018,
///   doi:10.1098/rsta.2017.0409).
/// - **Small**: under [`PLATES_MIN_MASS_EARTH`], a stagnant lid.
///
/// Super-Earths keep their plates: Valencia, O'Connell & Sasselov (2007,
/// doi:10.1086/524012) find them inevitable, O'Neill & Lenardic (2007,
/// doi:10.1029/2007GL030598) unlikely; the app follows the first, with
/// Korenaga (2010), for whom size plays a minor role (which argues against
/// any size threshold: the one above is the app's choice). Other looks
/// keep their regime: a stagnant look's world does not gain plates.
///
/// A world near its star keeps its plates: no source ties the inner edge
/// of the habitable zone to a regime (Venus's present mode is "stagnant
/// lid-like" after early plates, Weller et al. 2023,
/// doi:10.1038/s41550-023-02102-w, a history, not a threshold), and
/// switching there would cut its carbon cycle at a hard edge.
pub fn regime_of(p: &WorldParams) -> Regime {
    let preset = p.look.geology.regime;
    if preset != Regime::Plates || p.giant_mass_earth.is_some() {
        return preset;
    }
    if p.pressure_bar < LIQUID_WATER_BAR || p.mass_earth() < PLATES_MIN_MASS_EARTH {
        Regime::Stagnant
    } else {
        Regime::Plates
    }
}

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
    /// Tectonic regime. `plates` gives the world tectonic plates whose
    /// boundaries shape its relief (`crate::plates`) and, with liquid
    /// water, a carbon cycle that steadies its climate; the others shape it
    /// through the age of its surface and its craters. A plate look's
    /// world can lose its plates (see [`regime_of`]).
    pub regime: Regime,
    /// End of the era of liquid water, in Ga before present. 0 means still
    /// wet; a value at or above the surface age means the surface never saw
    /// liquid water (Mercury). Craters older than it will be worn by the
    /// hydraulic erosion, younger ones stay fresh.
    pub wet_until: f32,
    /// When lava flooded large impact basins, `[start, end]` in Ga before
    /// present (the Moon's maria: 3.9 to 3.1), or none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maria: Option<[f32; 2]>,
    /// Impactors hitting the body per unit area and time, against the
    /// Moon: 1 for a world of any other system, whose rate comes from its
    /// giants and belts. Set for Solar System look-alikes where a source
    /// gives it (`mars-like`: 2.6, Hartmann's Mars/Moon `R_bolide` for
    /// asteroids, PSI isochron page).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub impact_ratio: f32,
    /// Diameter of the crater the same impactor digs on the body, against
    /// the Moon (impact speed and gravity): 1 elsewhere; `mars-like`
    /// 0.751 (Hartmann, as above).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub crater_scale: f32,
}

fn one() -> f32 {
    1.0
}

fn is_one(v: &f32) -> bool {
    *v == 1.0
}

impl Default for Geology {
    /// Earth's.
    fn default() -> Self {
        Self {
            regime: Regime::Plates,
            wet_until: 0.0,
            maria: None,
            impact_ratio: 1.0,
            crater_scale: 1.0,
        }
    }
}

impl Geology {
    /// Checks every field is within its range.
    pub fn validate(&self) -> Result<(), String> {
        let (min, max) = (SURFACE_AGE_GA.min, SURFACE_AGE_GA.max);
        let within = |v: f32| v.is_finite() && (min..=max).contains(&v);
        if !within(self.wet_until) {
            return Err(format!(
                "geology.wet_until: must stay within [{min}, {max}]"
            ));
        }
        for (name, v) in [
            ("impact_ratio", self.impact_ratio),
            ("crater_scale", self.crater_scale),
        ] {
            if !(v.is_finite() && (0.1..=10.0).contains(&v)) {
                return Err(format!("geology.{name}: must stay within [0.1, 10]"));
            }
        }
        if let Some([start, end]) = self.maria {
            if !within(start) || !within(end) {
                return Err(format!("geology.maria: must stay within [{min}, {max}]"));
            }
            if start < end {
                return Err("geology.maria: [start, end] with start the older (larger)".into());
            }
        }
        Ok(())
    }

    /// End of the wet era of a world of this surface pressure, in Ga: 0 for
    /// a world with liquid water today, whatever the look says.
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
    fn the_world_decides_whether_a_plate_look_has_plates() {
        let world = |planet: &str, circumference_km: f32, pressure_bar: f32, orbit_au: f32| {
            let p = WorldParams {
                circumference_km,
                pressure_bar,
                orbit_au,
                ..crate::generation::tests::params(0, planet)
            };
            regime_of(&p)
        };
        let earth = crate::noise::EARTH_CIRCUMFERENCE_KM;
        // An Earth keeps its plates, and so does a super-Earth.
        assert_eq!(world("terran", earth, 1.0, 1.0), Regime::Plates);
        assert_eq!(world("terran", 2.0 * earth, 1.0, 1.0), Regime::Plates);
        // A Mars-sized one has a stagnant lid, as Mars.
        assert_eq!(world("terran", 21_344.0, 1.0, 1.0), Regime::Stagnant);
        // Without liquid water, too.
        assert_eq!(world("terran", earth, 0.005, 1.0), Regime::Stagnant);
        // Near its star too: no jump of its climate at the habitable
        // zone's inner edge.
        assert_eq!(world("terran", earth, 1.0, 0.72), Regime::Plates);
        // A stagnant look never gains plates.
        assert_eq!(world("barren", earth, 1.0, 1.0), Regime::Stagnant);
        // A world of the planet model carries its model's regime: Mars's
        // is a stagnant lid.
        let small = crate::request::WorldRequest {
            planet: Some("mars-like".into()),
            seed: Some(1),
            width: Some(64),
            height: Some(32),
            ..Default::default()
        };
        let params = small.resolve(&mut rand::rng()).unwrap().params;
        assert_eq!(params.look.geology.regime, Regime::Stagnant);
    }

    #[test]
    fn a_wet_stagnant_lid_has_a_weaker_carbon_cycle() {
        use crate::generation::carbon_cycle;
        assert_eq!(carbon_cycle(Regime::Plates, 1.0), 1.0);
        let lid = carbon_cycle(Regime::Stagnant, 1.0);
        assert!(lid > 0.0 && lid < 1.0);
        assert_eq!(carbon_cycle(Regime::Stagnant, 0.006), 0.0);
        assert_eq!(carbon_cycle(Regime::Episodic, 90.0), 0.0);
    }

    #[test]
    fn worlds_with_liquid_water_are_still_wet() {
        let mars = Geology {
            regime: Regime::Stagnant,
            wet_until: 3.5,
            maria: None,
            ..Geology::default()
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
        for (maria, ok) in [([3.9, 3.1], true), ([3.1, 3.9], false), ([5.0, 3.0], false)] {
            let g = Geology {
                maria: Some(maria),
                ..Geology::default()
            };
            assert_eq!(g.validate().is_ok(), ok, "{maria:?}");
        }
    }
}
