//! The terrain units of a body's surface: the share of it that each
//! renewal process last resurfaced, and how old that part is.
//!
//! [`Terrain`] is what the model keeps (`PlanetModel::terrain`) and what
//! the stored surface age reads (`interim::params_for`). [`icy_estimate`]
//! gives an ice crust its units before the climate is known, [`terrain`]
//! gives every body its units after it.

use serde::{Deserialize, Serialize};

use super::albedo::cover;
use super::climate::SurfaceLiquid;
use super::heat::{ICY_K2_OVER_Q, InternalHeat, tidal_power_w};
use super::inputs::HostTide;
use super::interim::{EPISODIC_SURFACE_AGE_GA, ERODING_BAR, HEAT_PIPE_SURFACE_AGE_GA};
use super::{Gases, PlanetModel};
use crate::archetype::ParamDefault;
use crate::exomoon::EARTH_M;
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
/// surface age a rock lid would (`interim::STAGNANT_SURFACE_AGE_SHARE`), and the
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

/// Age under which the mobile ice of a surface leaves it, Ga: its frost
/// of nitrogen, methane or carbon monoxide sublimes and re-condenses over
/// the seasons and the years, and buries what lies under it. Triton's
/// surface keeps about 10 to 100 Ma of craters (Schenk & Zahnle 2007,
/// Icarus 192, 135, doi:10.1016/j.icarus.2007.07.004).
pub const VOLATILE_ICE_RETENTION_GYR: f64 = 0.1;

/// Share of [`VOLATILE_ICE_RETENTION_GYR`] a surface under mobile ice
/// keeps, a range, drawn by `interim::params_for` in the regime draw's
/// place: Triton's 10 to 100 Ma (Schenk & Zahnle 2007).
pub(super) const VOLATILE_ICE_AGE_SHARE: ParamDefault = ParamDefault::Range([0.1, 1.0]);

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

/// Eccentricity a moon's passage through Laplace-like resonances pumps
/// it to: Ganymede's, through the three-body resonances that precede the
/// Laplace one, about 0.07, tidal heating several hundred times today's
/// (Showman & Malhotra 1997, Icarus 127, 93, doi:10.1006/icar.1996.5669,
/// abstract). The tide at that eccentricity on its present orbit is far
/// above what keeps an ice lid's surface (Ganymede about 0.3 W/m² with
/// [`super::heat::ICY_K2_OVER_Q`]), so the share it renews is
/// [`TIDAL_EPISODE_SHARE`]'s.
pub const TIDAL_EPISODE_ECCENTRICITY: f64 = 0.07;

/// Most of a surface a tidal episode renews: Ganymede's light terrain,
/// grooved and smooth, about 65 % of it, its dark terrain about 35 %
/// (Patterson et al. 2010, Icarus 207, 845,
/// doi:10.1016/j.icarus.2009.11.035; USGS map SIM 3237). Calibration on
/// Ganymede: no law says why the episode stopped short of the whole
/// surface (its tectonics broke the shell along bands, Pappalardo et al.
/// 2004 in *Jupiter*, Cambridge UP, not checked).
pub const TIDAL_EPISODE_SHARE: f64 = 0.65;

/// Share of the system's age since the tidal episode of a placed moon
/// (asked by archetype, or a partner placed with one), whose formation
/// orbit, and so its capture, the draw does not know. Calibration on
/// Ganymede, no law: 0.44 of 4.57 Ga is 2.0 Ga, the age crater counts
/// give its grooved terrain (Zahnle et al. 2003,
/// doi:10.1016/S0019-1035(03)00048-4, their model A; the value as
/// [`ICE_LID_HALF_FLUX_W_M2`] cites it, not checked again). A drawn moon
/// is dated by its capture ([`super::resonance::Captures::age_share`]).
pub const TIDAL_EPISODE_AGE_SHARE: f64 = 0.44;

/// Spread of the stored map age of a tidal episode's unit around its
/// model age ([`TIDAL_EPISODE_AGE_SHARE`] already is the calibration, so
/// the scatter is centred on 1), a range drawn by `interim::params_for`
/// in the regime draw's place. A stated spread of about 10 %, in the
/// order of the 1 Ga uncertainty on Ganymede's grooved terrain age
/// (Zahnle et al. 2003), not a calibration.
pub(super) const TIDAL_EPISODE_AGE_SPREAD: ParamDefault = ParamDefault::Range([0.9, 1.1]);

/// Least share of the surface an episode's unit covers: a smaller patch
/// is not a unit. Calibration, no source.
pub const TIDAL_EPISODE_MIN_SHARE: f64 = 0.01;

/// The unit a past tidal episode left on a moon of `tide`'s host, of
/// radius `radius_earth`, in a system `age_gyr` old, whose surface is
/// otherwise `base_age_gyr` old; `None` for a moon that went through no
/// Laplace-like resonance ([`super::resonance::Resonance::laplace_like`])
/// and for one whose surface renews faster than the episode is old (Io's,
/// Europa's).
///
/// - **Which moons**: a moon of a resonant chain of three or more, not
///   its innermost: the chain captured it after its inner moons had
///   locked, through Laplace-like three-body resonances that pumped its
///   eccentricity to [`TIDAL_EPISODE_ECCENTRICITY`] (Showman & Malhotra
///   1997, doi:10.1006/icar.1996.5669, for Ganymede's grooved terrain).
///   That is the source's scope: a moon locked in a 2:1 pair alone gets
///   no episode (an outer moon's forced eccentricity in a pair is small,
///   Dione's about 0.002).
/// - **When**: for a drawn moon, at its capture, the share of the age
///   since then that [`super::resonance::capture`] found under its
///   constant migration rates; for a placed moon, whose capture the draw
///   does not know, [`TIDAL_EPISODE_AGE_SHARE`] of the age ago, a
///   calibration on Ganymede.
/// - **How much**: [`TIDAL_EPISODE_SHARE`] times the share an ice lid
///   heated by the episode's tide would renew (`1 −`
///   [`ice_lid_age_share`] of its flux: the tide ([`tidal_power_w`],
///   [`super::heat::ICY_K2_OVER_Q`]) at the pumped eccentricity on its
///   present orbit, over its surface), so that a small or distant moon,
///   barely heated, keeps its surface.
pub fn tidal_episode(
    tide: Option<&HostTide>,
    radius_earth: f64,
    age_gyr: f64,
    base_age_gyr: f64,
) -> Option<TerrainUnit> {
    let tide = tide?;
    let resonance = tide.resonance.filter(|r| r.laplace_like)?;
    let age = age_gyr
        * resonance
            .capture_age_share
            .unwrap_or(TIDAL_EPISODE_AGE_SHARE);
    if age >= base_age_gyr {
        return None;
    }
    let radius_m = radius_earth * EARTH_M;
    let power = tidal_power_w(
        tide.host_kg,
        radius_m,
        tide.a_m,
        TIDAL_EPISODE_ECCENTRICITY,
        ICY_K2_OVER_Q,
    );
    let flux = power / (4.0 * std::f64::consts::PI * radius_m * radius_m);
    let share = TIDAL_EPISODE_SHARE * (1.0 - ice_lid_age_share(flux));
    (share >= TIDAL_EPISODE_MIN_SHARE).then_some(TerrainUnit {
        age_gyr: age,
        share,
        renewal: Renewal::TidalEpisode,
    })
}

/// Before the climate: the units an ice crust gets from its heat and its
/// resonance history alone: an ice lid ([`ice_lid_age_share`]), part of
/// it renewed by the past tidal episode of `tide` ([`tidal_episode`]).
/// [`terrain`] gives an ice crust the same units.
pub fn icy_estimate(
    heat: &InternalHeat,
    radius_earth: f64,
    age_gyr: f64,
    tide: Option<&HostTide>,
) -> Terrain {
    let flux = heat.flux_w_m2(radius_earth);
    let lid = age_gyr * ice_lid_age_share(flux);
    let Some(episode) = tidal_episode(tide, radius_earth, age_gyr, lid) else {
        return one(lid, Renewal::IceLid);
    };
    let mut units = vec![
        episode,
        TerrainUnit {
            age_gyr: lid,
            share: 1.0 - episode.share,
            renewal: Renewal::IceLid,
        },
    ];
    units.sort_by(|a, b| b.share.total_cmp(&a.share));
    Terrain { units }
}

/// After the climate: every unit of the body, in a system `age_gyr` Ga
/// old, with `frost` (bar, gas by gas, [`super::climate::frost`]) on its
/// ground, under the tides of `tide` (a moon's host, see
/// [`super::Context::tide`]). A liquid cycle's unit
/// ([`liquid_cycle_age_gyr`]), else the mean of the regime's range, or
/// for a stagnant lid the system's age, shortened over an ice crust by
/// [`ice_lid_age_share`] and partly renewed by a past tidal episode
/// ([`icy_estimate`]). A body whose air
/// is in vapour equilibrium with a nitrogen or methane frost (carbon
/// monoxide is not a gas of the model) also gets a
/// [`Renewal::VolatileIce`] unit of the share of its disc that frost
/// covers ([`super::albedo::cover`]); the other unit keeps the rest. The
/// SO₂ frost of a volcanic body is not mobile ice: a heat pipe renews
/// faster.
pub fn terrain(
    model: &PlanetModel,
    age_gyr: f64,
    frost: &Gases,
    tide: Option<&HostTide>,
) -> Terrain {
    let base = base_terrain(model, age_gyr, tide);
    let Some(surface) = model.surface.as_ref() else {
        return base;
    };
    let volatile = Gases {
        n2: frost.n2,
        ch4: frost.ch4,
        ..Gases::default()
    };
    let share = cover(
        surface.climate.temperature_k,
        &surface.gases,
        &surface.water,
        &volatile,
    )
    .frost;
    if share <= 0.0 {
        return base;
    }
    let mut units = vec![TerrainUnit {
        age_gyr: VOLATILE_ICE_RETENTION_GYR,
        share,
        renewal: Renewal::VolatileIce,
    }];
    if share < 1.0 {
        units.extend(base.units.iter().map(|u| TerrainUnit {
            share: u.share * (1.0 - share),
            ..*u
        }));
    }
    // Largest first; the volatile ice leads a tie.
    units.sort_by(|a, b| b.share.total_cmp(&a.share));
    Terrain { units }
}

/// The units of the surface under the mobile ice, if any.
fn base_terrain(model: &PlanetModel, age_gyr: f64, tide: Option<&HostTide>) -> Terrain {
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
                icy_estimate(&model.heat, model.interior.radius_earth, age_gyr, tide)
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
    fn triton_is_young() {
        for seed in 1..=40 {
            let p = params_of("triton-like", seed).expect("drawn");
            let m = p.model.as_ref().expect("a model");
            assert_eq!(
                m.terrain.main().unwrap().renewal,
                Renewal::VolatileIce,
                "seed {seed}: {:?}",
                m.terrain
            );
            assert!(p.surface_age_ga < 0.1, "seed {seed}: {}", p.surface_age_ga);
        }
    }

    /// The tidal episode's age is scattered once (issue #566): the stored
    /// map age stays near the unit's 2.0 Ga.
    #[test]
    fn ganymedes_stored_age_is_about_two_ga() {
        let mut ages = vec![];
        for seed in 1..=40 {
            let p = params_of("ganymede-like", seed).expect("drawn");
            let m = p.model.as_ref().expect("a model");
            let unit = m.terrain.main().expect("a unit");
            assert_eq!(unit.renewal, Renewal::TidalEpisode, "seed {seed}");
            let ratio = f64::from(p.surface_age_ga) / unit.age_gyr;
            assert!((0.89..=1.11).contains(&ratio), "seed {seed}: {ratio}");
            ages.push(p.surface_age_ga);
        }
        ages.sort_by(f32::total_cmp);
        assert!((1.7..=2.3).contains(&ages[20]), "{ages:?}");
    }

    #[test]
    fn titan_is_not_volatile_ice() {
        for seed in 1..=40 {
            let p = params_of("titan-like", seed).expect("drawn");
            let m = p.model.as_ref().expect("a model");
            assert!(
                m.terrain
                    .units
                    .iter()
                    .all(|u| u.renewal != Renewal::VolatileIce),
                "seed {seed}: {:?}",
                m.terrain
            );
            assert_eq!(m.terrain.main().unwrap().renewal, Renewal::LiquidCycle);
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
    fn ganymede_has_grooved_terrain() {
        for seed in 1..=40 {
            let p = params_of("ganymede-like", seed).expect("drawn");
            let t = &p.model.as_ref().expect("a model").terrain;
            assert_eq!(t.units.len(), 2, "seed {seed}: {t:?}");
            let renewed = t
                .units
                .iter()
                .find(|u| u.renewal == Renewal::TidalEpisode)
                .unwrap_or_else(|| panic!("seed {seed}: {t:?}"));
            let old = t
                .units
                .iter()
                .find(|u| u.renewal != Renewal::TidalEpisode)
                .unwrap();
            assert!((0.4..=0.8).contains(&renewed.share), "seed {seed}: {t:?}");
            assert!(renewed.age_gyr < old.age_gyr, "seed {seed}: {t:?}");
            assert!(old.age_gyr >= 3.0, "seed {seed}: {t:?}");
            let sum: f64 = t.units.iter().map(|u| u.share).sum();
            assert!((sum - 1.0).abs() < 1e-12, "seed {seed}: {t:?}");
        }
    }

    #[test]
    fn callisto_has_one_old_unit() {
        for seed in 1..=40 {
            let p = params_of("callisto-like", seed).expect("drawn");
            let t = &p.model.as_ref().expect("a model").terrain;
            assert_eq!(t.units.len(), 1, "seed {seed}: {t:?}");
            assert!(t.units[0].age_gyr >= 3.0, "seed {seed}: {t:?}");
        }
    }

    #[test]
    fn europa_and_io_stay_young() {
        for (arch, renewal, under) in [
            ("europa-like", Renewal::IceLid, 0.3),
            ("io-like", Renewal::HeatPipe, 0.002),
        ] {
            for seed in 1..=40 {
                let p = params_of(arch, seed).expect("drawn");
                let t = &p.model.as_ref().expect("a model").terrain;
                assert_eq!(t.units.len(), 1, "{arch} seed {seed}: {t:?}");
                assert_eq!(t.units[0].renewal, renewal, "{arch} seed {seed}");
                assert!(t.units[0].age_gyr < under, "{arch} seed {seed}: {t:?}");
            }
        }
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
        let mut skipped_liquid = 0;
        let mut skipped_volatile = 0;
        let mut skipped_episode = 0;
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
            // So does mobile ice: the triton-like rows move under 0.1
            // Ga, see `triton_is_young`. And a past tidal episode: the
            // ganymede-like rows' main unit is now their grooved terrain,
            // see `ganymede_has_grooved_terrain`.
            let main = p.model.as_ref().and_then(|m| m.terrain.main().copied());
            match main.map(|u| u.renewal) {
                Some(Renewal::LiquidCycle) => {
                    assert!(arch.starts_with("titan-like"), "{arch} {seed}");
                    skipped_liquid += 1;
                    continue;
                }
                Some(Renewal::VolatileIce) => {
                    assert!(arch.starts_with("triton-like"), "{arch} {seed}");
                    skipped_volatile += 1;
                    continue;
                }
                Some(Renewal::TidalEpisode) => {
                    assert!(arch.starts_with("ganymede-like"), "{arch} {seed}");
                    skipped_episode += 1;
                    continue;
                }
                _ => {}
            }
            if now.abs_diff(bits) > slack {
                drifted.push(format!("{arch} {seed}: {bits} -> {now}"));
            }
        }
        assert!(drifted.is_empty(), "{drifted:#?}");
        assert!(skipped_liquid > 0, "no titan-like row was skipped");
        assert!(skipped_volatile > 0, "no triton-like row was skipped");
        assert!(skipped_episode > 0, "no ganymede-like row was skipped");
    }
}
