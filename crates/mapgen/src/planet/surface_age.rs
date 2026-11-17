//! The terrain units of a body's surface: the share of it that each
//! renewal process last resurfaced, and how old that part is.
//!
//! [`Terrain`] is what the model keeps (`PlanetModel::terrain`) and what
//! the stored surface age reads (`interim::params_for`). [`icy_estimate`]
//! gives an ice crust its units before the climate is known, [`terrain`]
//! gives every body its units after it.

use serde::{Deserialize, Serialize};

use super::PlanetModel;
use super::climate::SurfaceLiquid;
use super::heat::InternalHeat;
use super::inputs::HostTide;
use super::interim::{EPISODIC_SURFACE_AGE_GA, ERODING_BAR, HEAT_PIPE_SURFACE_AGE_GA};
use crate::archetype::ParamDefault;
use crate::geology::Regime;
use crate::request::fallback;

/// What last renewed a part of a surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Renewal {
    /// Not renewed since the body formed (a stagnant rock lid).
    Formation,
    /// Plate tectonics.
    Plates,
    /// Episodic global overturn (Venus).
    Episodic,
    /// Heat-pipe volcanism (Io).
    HeatPipe,
    /// A stagnant ice lid over warm ice, renewed by its internal heat.
    IceLid,
    /// A liquid cycle (rain, rivers, seas).
    LiquidCycle,
    /// Volatile ice that comes and goes with the seasons or the years.
    VolatileIce,
    /// A tidal episode (a resonance that heated the body for a while).
    TidalEpisode,
}

/// A part of the surface renewed the same way, `share` of the area, last
/// resurfaced `age_gyr` Ga ago.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TerrainUnit {
    pub age_gyr: f64,
    pub share: f64,
    pub renewal: Renewal,
}

/// Units sorted by share, largest first; shares sum to 1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Terrain {
    pub units: Vec<TerrainUnit>,
}

impl Terrain {
    /// The unit that covers the most of the surface.
    pub fn main(&self) -> Option<&TerrainUnit> {
        self.units.first()
    }
}

/// Internal heat flux, W/m², under which a stagnant ice lid keeps half the
/// surface age a rock lid would ((`interim::STAGNANT_SURFACE_AGE_SHARE`)), and the
/// steepness of the fall past it ([`ICE_LID_EXPONENT`]): the share kept is
/// `1 / (1 + (F / F½)^n)`. An ice shell resurfaces (convection, diapirs,
/// cryovolcanism, tectonics) far more readily than rock, ice flowing and
/// melting within a hundred kelvin of its surface, and the conductive lid
/// over its warm ice thins as `1 / F` (`ice_shell::conductive_shell_m`).
/// Calibration on the Galilean moons, no source for the law itself: 10
/// mW/m² and 4 keep Callisto's surface (about 2 to 5 mW/m² of radiogenic
/// heat in the model) about 4 Ga old, saturated with craters (Zahnle et
/// al. 2003, doi:10.1016/S0019-1035(03)00048-4), Ganymede's (its dark
/// terrain about 4 Ga, its grooved terrain about 2 Ga) 2 to 4 Ga, and
/// Europa's (35 mW/m² of tidal heat, see `heat::ICY_K2_OVER_Q`) under
/// 0.1 Ga, near its 40 to 90 Ma (Zahnle et al. 2003; Bierhaus et al.
/// 2009, *Europa*, University of Arizona Press). A rock lid keeps its
/// age (Mars and Mercury, about 20 mW/m² today, keep 4 Ga surfaces).
pub const ICE_LID_HALF_FLUX_W_M2: f64 = 0.010;
/// See [`ICE_LID_HALF_FLUX_W_M2`].
pub const ICE_LID_EXPONENT: i32 = 4;

/// Share of a rock lid's surface age that a stagnant ice lid pushing out
/// `flux_w_m2` keeps ([`ICE_LID_HALF_FLUX_W_M2`]).
pub fn ice_lid_age_share(flux_w_m2: f64) -> f64 {
    1.0 / (1.0 + (flux_w_m2 / ICE_LID_HALF_FLUX_W_M2).powi(ICE_LID_EXPONENT))
}

/// Age under which a liquid cycle (rain, rivers, seas) leaves a surface,
/// Ga, in air of `10 * ERODING_BAR` or more: the upper bound of Titan's
/// crater retention age, 0.2 to 1 Ga (Neish & Lorenz 2012,
/// doi:10.1016/j.pss.2011.02.016). Titan keeps about 60 craters, as if
/// its methane rain and its dunes buried the rest.
pub const LIQUID_CYCLE_RETENTION_GYR: f64 = 1.0;

/// Share of [`LIQUID_CYCLE_RETENTION_GYR`] a liquid cycle's surface keeps,
/// a range, drawn by `interim::params_for` in the regime draw's place:
/// Titan's 0.2 to 1 Ga (Neish & Lorenz 2012).
pub(super) const LIQUID_CYCLE_AGE_SHARE: ParamDefault = ParamDefault::Range([0.2, 1.0]);

/// Age of the surface of a world whose liquid cycle renews it, Ga, or
/// `None` without one (a plate or heat-pipe regime renews faster, see
/// [`terrain`]): air of `ERODING_BAR` or more over a surface liquid
/// of water or hydrocarbons. The age is [`LIQUID_CYCLE_RETENTION_GYR`] in
/// air of ten times `ERODING_BAR`, and rises with the log of the pressure
/// down to the system's `age_gyr` at `ERODING_BAR` (where the cycle just
/// starts to carve, see `interim::params_for`'s erosion), so that it is
/// continuous in pressure. Calibration, no source for the interpolation.
pub fn liquid_cycle_age_gyr(pressure_bar: f64, liquid: SurfaceLiquid, age_gyr: f64) -> Option<f64> {
    if !matches!(liquid, SurfaceLiquid::Water | SurfaceLiquid::Hydrocarbons)
        || pressure_bar < ERODING_BAR
    {
        return None;
    }
    let t = (pressure_bar / ERODING_BAR).log10().clamp(0.0, 1.0);
    let ramp = (age_gyr / LIQUID_CYCLE_RETENTION_GYR)
        .max(1.0)
        .powf(1.0 - t);
    Some(age_gyr.min(LIQUID_CYCLE_RETENTION_GYR * ramp))
}

/// Middle of a range (a fixed value is its own).
fn midpoint(range: ParamDefault) -> f64 {
    let (lo, hi) = range.bounds();
    f64::from(lo + hi) / 2.0
}

/// A single unit covering the whole surface.
fn one(age_gyr: f64, renewal: Renewal) -> Terrain {
    Terrain {
        units: vec![TerrainUnit {
            age_gyr,
            share: 1.0,
            renewal,
        }],
    }
}

/// Before the climate: the units an ice crust gets from its heat and its
/// resonance history alone (an ice lid, else the formation age; the tidal
/// episode of `tide` joins them later).
pub fn icy_estimate(
    heat: &InternalHeat,
    radius_earth: f64,
    age_gyr: f64,
    _tide: Option<&HostTide>,
) -> Terrain {
    let flux = heat.flux_w_m2(radius_earth);
    one(age_gyr * ice_lid_age_share(flux), Renewal::IceLid)
}

/// After the climate: every unit of the body, in a system `age_gyr` Ga
/// old. For now one unit: a liquid cycle's ([`liquid_cycle_age_gyr`]), else the mean of the regime's range, or for a
/// stagnant lid the system's age, shortened over an ice crust by
/// [`ice_lid_age_share`].
pub fn terrain(model: &PlanetModel, age_gyr: f64) -> Terrain {
    let regime = model.surface.as_ref().map(|s| s.regime);
    if let (Some(surface), Some(regime)) = (model.surface.as_ref(), regime)
        && regime == Regime::Stagnant
        && let Some(age) =
            liquid_cycle_age_gyr(surface.gases.total(), surface.climate.liquid, age_gyr)
    {
        return one(age, Renewal::LiquidCycle);
    }
    match regime {
        Some(Regime::Plates) | None => one(midpoint(fallback::SURFACE_AGE_GA), Renewal::Plates),
        Some(Regime::Episodic) => one(midpoint(EPISODIC_SURFACE_AGE_GA), Renewal::Episodic),
        Some(Regime::HeatPipe) => one(midpoint(HEAT_PIPE_SURFACE_AGE_GA), Renewal::HeatPipe),
        Some(Regime::Stagnant) => {
            if crate::generation::model_crust_kind(model) == crate::generation::CrustKind::Ice {
                icy_estimate(&model.heat, model.interior.radius_earth, age_gyr, None)
            } else {
                one(age_gyr, Renewal::Formation)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::Renewal;

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::system::{SystemRequest, WholeSystemRequest, draw, generation_params};

    fn params_of(arch: &str, seed: u32) -> Option<crate::world::WorldParams> {
        let req = SystemRequest::System(WholeSystemRequest {
            planets: Some(BTreeMap::from([(arch.to_string(), 1)])),
            seed: Some(seed),
            ..WholeSystemRequest::default()
        });
        let sys = draw(&req, &mut StdRng::seed_from_u64(u64::from(seed))).ok()?;
        let b = sys
            .bodies
            .iter()
            .find(|b| b.params.archetype.as_deref() == Some(arch))?;
        Some(generation_params(b))
    }

    #[test]
    fn one_unit_per_body_today() {
        for arch in [
            "io-like",
            "europa-like",
            "callisto-like",
            "mars-like",
            "earth-like",
        ] {
            for seed in 1..=5 {
                let p = params_of(arch, seed).expect("drawn");
                let t = &p.model.as_ref().expect("a model").terrain;
                assert_eq!(t.units.len(), 1, "{arch} seed {seed}");
                let sum: f64 = t.units.iter().map(|u| u.share).sum();
                assert!((sum - 1.0).abs() < 1e-12, "{arch} seed {seed}");
            }
        }
    }

    #[test]
    fn titan_is_young() {
        for seed in 1..=40 {
            let p = params_of("titan-like", seed).expect("drawn");
            let m = p.model.as_ref().expect("a model");
            assert_eq!(m.terrain.main().unwrap().renewal, Renewal::LiquidCycle);
            assert!(
                (0.2..=1.0).contains(&p.surface_age_ga),
                "seed {seed}: {}",
                p.surface_age_ga
            );
        }
    }

    #[test]
    fn liquid_cycle_age_is_continuous_in_pressure() {
        use super::{SurfaceLiquid, liquid_cycle_age_gyr};
        let ages: Vec<f64> = (0..=50)
            .map(|i| {
                let p = 0.0099 * (0.11f64 / 0.0099).powf(f64::from(i) / 50.0);
                liquid_cycle_age_gyr(p, SurfaceLiquid::Hydrocarbons, 4.5).unwrap_or(4.5)
            })
            .collect();
        for w in ages.windows(2) {
            assert!((w[1] / w[0] - 1.0).abs() < 0.1, "{w:?}");
        }
        assert!((ages[50] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn mars_and_mercury_stay_old() {
        for arch in ["mars-like", "mercury-like"] {
            for seed in 1..=40 {
                let p = params_of(arch, seed).expect("drawn");
                assert!(
                    p.surface_age_ga >= 3.0,
                    "{arch} {seed}: {}",
                    p.surface_age_ga
                );
            }
        }
    }

    /// The stored surface ages of `main` 99690d2, bit for bit (an ice lid's
    /// within two ulps, see below): the terrain units change where the age
    /// comes from, not what it is.
    #[test]
    fn stored_ages_do_not_move() {
        let fixture: Vec<(String, u32, u32)> = serde_json::from_str(include_str!(
            "../../tests/fixtures/surface_ages_99690d2.json"
        ))
        .unwrap();
        assert_eq!(fixture.len(), 210);
        let mut drifted = vec![];
        for (arch, seed, bits) in fixture {
            let p = params_of(&arch, seed).expect("drawn");
            let now = p.surface_age_ga.to_bits();
            let ice_lid = p.model.as_ref().is_some_and(|m| {
                m.terrain
                    .main()
                    .is_some_and(|u| u.renewal == Renewal::IceLid)
            });
            // An ice lid's age is now shortened before the regime's share
            // draw, not after it: the same product in the other order of
            // f32 roundings, up to two ulps apart.
            let slack = if ice_lid { 2 } else { 0 };
            // A liquid cycle renews the surface on purpose (#491): the
            // Titan-like rows move to 0.2-1 Ga, see `titan_is_young`.
            if p.model.as_ref().is_some_and(|m| {
                m.terrain
                    .main()
                    .is_some_and(|u| u.renewal == Renewal::LiquidCycle)
            }) {
                continue;
            }
            if now.abs_diff(bits) > slack {
                drifted.push(format!("{arch} {seed}: {bits} -> {now}"));
            }
        }
        assert!(drifted.is_empty(), "{drifted:#?}");
    }
}
