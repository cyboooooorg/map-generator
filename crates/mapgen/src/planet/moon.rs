//! A large moon through the planet model (part 4 of #423): a round moon
//! gets its inputs from how it formed ([`MoonFormation`]) and goes through
//! [`planet_model`] under its star's light at its planet's orbit, with its
//! planet's tides as a heat source ([`derive_moon_model`]).
//!
//! What differs from a planet: its mass and its ice share are given, not
//! drawn; it holds no hydrogen and helium; a moon born in a warm disk
//! keeps no hypervolatile ices ([`keeps_hypervolatiles`]); and a giant's
//! moon meets the unshielded escape inside its giant's magnetosphere (see
//! [`planet_model`]). The cosmic shoreline, the climate and the light are
//! the planet's: the XUV fluence and the equilibrium temperature at the
//! planet's orbit.
//!
//! Known departures: the shells of Ganymede and Titan, heated by their
//! rock and their tides alone, can come out thicker than interior models
//! give, so a Ganymede may keep no ocean on a low radiogenic draw, and
//! Enceladus's mean shell is thicker than observed (see
//! [`super::ice_shell::ice_shell`]).
use rand::SeedableRng;
use rand::rngs::StdRng;

use super::inputs::volatile_rule;
use super::{
    Context, HostTide, InputOverrides, OCEAN_KG, PlanetInputs, PlanetModel, draw_inputs_with,
    planet_model,
};
use crate::exomoon::EARTH_KG;
use crate::system::body_seed;

/// How a moon formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonBirth {
    /// In its planet's circumplanetary disk, a regular moon (Titan, the
    /// Galilean moons).
    Disk,
    /// Captured from beyond the star's ice line (Triton, a Kuiper-belt
    /// body).
    Captured,
    /// From the debris of a giant impact on its planet (the Moon).
    Impact,
}

/// What a moon formed with: the causes [`derive_moon_model`] derives it
/// from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoonFormation {
    /// Mass, Earth masses.
    pub mass_earth: f64,
    /// Share of the mass in water and ices.
    pub ice_fraction: f64,
    pub birth: MoonBirth,
    /// Its planet and its orbit around it, whose tides heat it.
    pub tide: HostTide,
}

/// Equilibrium temperature of the planet, K, under which the moons born
/// in its disk keep their hypervolatile ices: NH₃ (as its hydrate), CH₄
/// (as a clathrate) and N₂. Titan's nitrogen came from ammonia ice
/// formed in the protosolar disk (Mandt, Mousis, Lunine & Gautier 2014,
/// doi:10.1088/2041-8205/788/2/L24), which Saturn's cold subnebula let
/// its moons keep (Mousis, Gautier & Bockelée-Morvan 2002,
/// doi:10.1006/icar.2001.6782); a warmer subnebula, as Jupiter's,
/// vaporised them before the moons formed. Neither paper was read beyond
/// its title and abstract. The planet's equilibrium temperature stands
/// for its disk's, as for the disk's ice line. Calibration, no source,
/// for 100 K: between Jupiter (about 112 K at 5.2 AU, whose Ganymede and
/// Callisto are airless) and Saturn (about 83 K, whose Titan keeps
/// 1.5 bar of N₂); set where it is so that both come out as themselves.
pub const HYPERVOLATILES_MAX_K: f64 = 100.0;

/// Whether a moon keeps the hypervolatile ices of its ice share: a moon
/// born in a disk under [`HYPERVOLATILES_MAX_K`] and a captured one (a
/// body from beyond the ice line) do; a moon born in a warmer disk does
/// not. A moon born from an impact draws its volatiles as a planet does.
pub fn keeps_hypervolatiles(birth: MoonBirth, planet_equilibrium_k: f64) -> bool {
    match birth {
        MoonBirth::Disk => planet_equilibrium_k < HYPERVOLATILES_MAX_K,
        MoonBirth::Captured | MoonBirth::Impact => true,
    }
}

/// The model of the round moon `id` of the system of `system_seed`, which
/// formed as `moon` says around the planet whose formation context is
/// `planet_context`, with some inputs set in `overrides` (see
/// [`draw_inputs_with`]).
///
/// Its context is its planet's (its star's light, XUV and equilibrium
/// temperature at the planet's orbit) with the planet's tides on the
/// moon's orbit ([`Context::with_tide`]). Its inputs are drawn as a
/// planet's from the stream seeded by the system's seed and `<id>/model`
/// (as [`super::derive_model`]), with its mass and ice share from `moon`
/// (the ice as `water_oceans`) and no envelope; the other overrides
/// apply. The ice-borne N₂, NH₃ and CH₄ of the volatile rule are dropped
/// when it does not [`keep_hypervolatiles`](keeps_hypervolatiles); its
/// rock's N₂, and its CO₂, stay. The amounts it keeps are the planet
/// rule's (1 % of the ice in NH₃ and in CH₄, 0.1 % in N₂), which give
/// Titan its 1.5 bar or so of N₂ and a few percent of CH₄. Its geysers,
/// if its shell cracks through, are drawn on their own stream
/// (`super::vent`).
pub fn derive_moon_model(
    system_seed: u32,
    id: &str,
    planet_context: &Context,
    moon: &MoonFormation,
    overrides: &InputOverrides,
) -> PlanetModel {
    let context = planet_context.with_tide(moon.tide);
    let seed = body_seed(system_seed, &format!("{id}/model"));
    let mut rng = StdRng::seed_from_u64(u64::from(seed));
    let overrides = InputOverrides {
        water_oceans: Some(moon.ice_fraction * moon.mass_earth * EARTH_KG / OCEAN_KG),
        envelope_fraction: Some(0.0),
        ..overrides.clone()
    };
    let inputs = draw_inputs_with(&context, moon.mass_earth, &overrides, &mut rng);
    let inputs = if keeps_hypervolatiles(moon.birth, planet_context.equilibrium_k) {
        inputs
    } else {
        without_hypervolatiles(inputs)
    };
    super::vent(planet_model(&context, inputs), system_seed, id)
}

/// `inputs` without the N₂, NH₃ and CH₄ its ices carry: the N₂ keeps the
/// rock's share of the rule (whatever factor scaled it), NH₃ and CH₄, all
/// ice-borne, go.
fn without_hypervolatiles(inputs: PlanetInputs) -> PlanetInputs {
    let refractory = (1.0 - inputs.water_fraction - inputs.envelope_fraction).max(0.0);
    let rule = volatile_rule(refractory, inputs.water_fraction, inputs.core_mass_fraction);
    let rock = volatile_rule(refractory, 0.0, inputs.core_mass_fraction);
    let n2 = if rule.n2 > 0.0 {
        inputs.volatiles.n2 * rock.n2 / rule.n2
    } else {
        0.0
    };
    PlanetInputs {
        volatiles: super::Volatiles {
            n2,
            nh3: 0.0,
            ch4: 0.0,
            ..inputs.volatiles
        },
        ..inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geology::Regime;
    use crate::planet::{PlanetClass, SurfaceLiquid};
    use crate::star::{StarInfo, parse_type};

    const SATURN_KG: f64 = 5.683e26;
    const JUPITER_KG: f64 = 1.898e27;
    const NEPTUNE_KG: f64 = 1.024e26;
    const EARTH_HOST_KG: f64 = 5.972e24;

    /// The context of a planet at `orbit_au` round a G2V star at 4.57 Gyr.
    fn planet_at(orbit_au: f32) -> Context {
        let star = StarInfo::new(parse_type("G2V").unwrap(), orbit_au, 0.0, 4.57);
        Context::of(&star, 0.0, 2.7)
    }

    fn moon(
        mass_earth: f64,
        ice_fraction: f64,
        birth: MoonBirth,
        host_kg: f64,
        a_m: f64,
        eccentricity: f64,
    ) -> MoonFormation {
        MoonFormation {
            mass_earth,
            ice_fraction,
            birth,
            tide: HostTide {
                host_kg,
                a_m,
                eccentricity,
                resonance: None,
            },
        }
    }

    fn model(orbit_au: f32, m: &MoonFormation) -> PlanetModel {
        derive_moon_model(
            1,
            "p1/m1",
            &planet_at(orbit_au),
            m,
            &InputOverrides::default(),
        )
    }

    fn pressure(m: &PlanetModel) -> f64 {
        m.surface.expect("a moon has a surface").gases.total()
    }

    /// The ocean under the shell, km, at most the depth high-pressure ice
    /// allows.
    fn ocean_km(m: &PlanetModel) -> f64 {
        m.ice_shell.expect("an icy moon has a shell").ocean_km
    }

    fn titan() -> MoonFormation {
        moon(0.0225, 0.45, MoonBirth::Disk, SATURN_KG, 1.2219e9, 0.0288)
    }

    /// Titan at Saturn's orbit with the `titan-like` archetype's factors:
    /// its 93.7 K within 90 to 97, its 1.47 bar (Fulchignoni et al. 2005,
    /// doi:10.1038/nature04314) within 1.3 to 1.6, and 3 to 6 % of
    /// methane near its surface (5.7 %, Niemann et al. 2005,
    /// doi:10.1038/nature04122), on every seed.
    #[test]
    fn titan_comes_out_a_hydrocarbon_world() {
        use rand::SeedableRng;
        let archetype = crate::archetype::Archetype::load("titan-like").unwrap();
        for seed in 0..20 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed));
            let overrides = archetype.overrides(&mut rng);
            let m = derive_moon_model(seed, "p1/m1", &planet_at(9.54), &titan(), &overrides);
            let s = m.surface.expect("surface");
            assert_eq!(m.class, PlanetClass::HydrocarbonWorld, "seed {seed}: {m:?}");
            let t = s.climate.temperature_k;
            assert!((90.0..=97.0).contains(&t), "seed {seed}: {s:?}");
            assert_eq!(s.climate.liquid, SurfaceLiquid::Hydrocarbons, "{s:?}");
            let p = s.gases.total();
            assert!((1.3..=1.6).contains(&p), "seed {seed}: {s:?}");
            let ch4 = s.gases.ch4 / p;
            assert!((0.03..=0.06).contains(&ch4), "seed {seed}: {s:?}");
            assert!(ocean_km(&m) < 400.0, "{:?}", m.ice_shell);
        }
    }

    /// The model of the moon of archetype `name` in the system of `seed`
    /// that asks for it, as `archetype_census` draws it.
    fn archetype_moon(name: &str, seed: u32) -> PlanetModel {
        use crate::system::{SystemRequest, WholeSystemRequest, draw};
        use std::collections::BTreeMap;
        let request = SystemRequest::System(WholeSystemRequest {
            planets: Some(BTreeMap::from([(name.to_string(), 1)])),
            seed: Some(seed),
            ..WholeSystemRequest::default()
        });
        let system = draw(
            &request,
            &mut rand::rngs::StdRng::seed_from_u64(u64::from(seed)),
        )
        .expect("the system draws");
        let body = system
            .bodies
            .iter()
            .find(|b| b.params.archetype.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name} seed {seed}: no body"));
        body.model.clone().expect("an asked moon has a model")
    }

    /// Europa, Ganymede and Callisto lose the CO₂ and N₂ the shoreline
    /// left them to Jupiter's plasma and keep an exosphere: Callisto's
    /// CO₂ at 7.5e-12 bar (Carlson 1999, doi:10.1126/science.283.5403.820),
    /// Europa's and Ganymede's O₂ near 1e-12 (spec goal 2: 1e-13 to
    /// 1e-10 bar).
    #[test]
    fn galilean_icy_moons_have_exospheres() {
        for name in ["europa-like", "ganymede-like", "callisto-like"] {
            for seed in 1..=20 {
                let m = archetype_moon(name, seed);
                let p = pressure(&m);
                assert!(
                    (1e-13..=1e-10).contains(&p),
                    "{name} seed {seed}: {p:e} {m:?}"
                );
            }
        }
    }

    /// Io sits in Jupiter's plasma torus: its N₂ and CO₂ are an exosphere
    /// at most, whatever else it breathes.
    #[test]
    fn io_loses_its_nitrogen_and_carbon_dioxide() {
        for seed in 1..=20 {
            let m = archetype_moon("io-like", seed);
            let g = m.surface.expect("surface").gases;
            assert!(
                g.n2 + g.co2 <= super::super::EXOSPHERE_BAR,
                "seed {seed}: {g:?}"
            );
        }
    }

    /// Io breathes the SO₂ of its volcanoes: its dayside column of 1e16 to
    /// 2e17 cm⁻² (about 1 to 10 nbar) sits on SO₂ frost at 106 to 117 K
    /// (Giles et al. 2024, arXiv:2405.19253, from 22 years of IRTF/TEXES
    /// spectra), far above the N₂ and CO₂ the plasma left it. On every
    /// seed its air is 1e-10 to 1e-8 bar of SO₂, more than its N₂ and CO₂,
    /// the frost on its ground is SO₂, and its Bond albedo is within 0.4
    /// to 0.7 (0.63 observed, Simonelli & Veverka 1988, from memory).
    #[test]
    fn io_breathes_so2() {
        use crate::system::{SystemRequest, WholeSystemRequest, body_context, draw};
        use std::collections::BTreeMap;
        for seed in 1..=40 {
            let request = SystemRequest::System(WholeSystemRequest {
                planets: Some(BTreeMap::from([("io-like".to_string(), 1)])),
                seed: Some(seed),
                ..WholeSystemRequest::default()
            });
            let system = draw(
                &request,
                &mut rand::rngs::StdRng::seed_from_u64(u64::from(seed)),
            )
            .expect("the system draws");
            let body = system
                .bodies
                .iter()
                .find(|b| b.params.archetype.as_deref() == Some("io-like"))
                .expect("an io-like body");
            let m = body.model.as_ref().expect("a model");
            let s = m.surface.expect("surface");
            let g = s.gases;
            assert!((1e-10..=1e-8).contains(&g.so2), "seed {seed}: {g:?}");
            assert!(g.so2 > g.n2 + g.co2, "seed {seed}: {g:?}");
            let context = body_context(&system, body).expect("its context");
            let frost = crate::planet::explain::replay(&context, m, &s).frost;
            assert!(frost.so2 > 0.0, "seed {seed}: {frost:?}");
            assert!(
                frost.so2 > frost.total() - frost.so2,
                "seed {seed}: {frost:?}"
            );
            let a = s.climate.bond_albedo;
            assert!((0.4..=0.7).contains(&a), "seed {seed}: albedo {a}");
        }
    }

    /// A heat pipe warmer than Io keeps its volcanic SO₂ within the
    /// chain. Io round a Jupiter at 2.5 AU: its dayside frost, under SO₂'s
    /// triple point, breathes far more than a trace, which goes into the
    /// air before the climate, so that air and frost together hold the
    /// whole supply and the stored air, washed out as any SO₂ is, holds at
    /// most that. Io round a Jupiter at 1 AU and an Io round an Earth-mass
    /// host at 1 AU (no giant's plasma): their dayside is above the triple
    /// point, so they keep no volcanic frost and no volcanic air, only the
    /// SO₂ of their outgassing, finite and small.
    #[test]
    fn a_warm_heat_pipe_keeps_its_volcanic_so2() {
        use crate::planet::atmosphere::{
            SO2_FROST, TRACE_SO2_BAR, bar_per_kg, volcanic_so2_frost_bar, volcanic_so2_supply,
        };
        let io = moon(0.015, 0.0, MoonBirth::Disk, JUPITER_KG, 4.217e8, 0.0041);
        let near_earth = moon(0.015, 0.0, MoonBirth::Disk, EARTH_HOST_KG, 5.0e7, 0.02);
        for (orbit, f, frost) in [
            (2.5, &io, true),
            (1.0, &io, false),
            (1.0, &near_earth, false),
        ] {
            let m = model(orbit, f);
            let s = m.surface.expect("surface");
            assert_eq!(s.regime, Regime::HeatPipe, "{orbit} AU: {s:?}");
            let context = planet_at(orbit).with_tide(f.tide);
            let steps = crate::planet::explain::replay(&context, &m, &s);
            let loss = crate::planet::loss_context(&context, &m.inputs, &m.interior);
            let supply = volcanic_so2_supply(&m.heat, &m.inputs);
            let inventory =
                volcanic_so2_frost_bar(supply, &loss, bar_per_kg(m.inputs.mass_earth, &m.interior));
            assert!(inventory > 0.0, "{orbit} AU: {supply}");
            let vapour = steps.before.gases.so2 - steps.trapped.so2;
            let so2 = s.gases.so2;
            assert!(so2.is_finite() && so2 >= 0.0, "{orbit} AU: {s:?}");
            assert!(so2 <= steps.before.gases.so2, "{orbit} AU: {s:?}");
            if frost {
                let held = vapour + steps.before.so2_frost_bar;
                assert!(
                    (TRACE_SO2_BAR..=SO2_FROST.triple_bar).contains(&vapour),
                    "{orbit} AU: {vapour:e}"
                );
                assert!(
                    (held / inventory - 1.0).abs() < 1e-9,
                    "{orbit} AU: {held:e} of {inventory:e}"
                );
            } else {
                assert_eq!(vapour, 0.0, "{orbit} AU");
                assert_eq!(steps.before.so2_frost_bar, 0.0, "{orbit} AU");
                assert!(so2 < SO2_FROST.triple_bar, "{orbit} AU: {s:?}");
            }
        }
    }

    /// Io's volcanic frost holds its air at its dayside vapour wherever
    /// it lies, on both sides of [`TRACE_SO2_BAR`] (the vapour then joins
    /// the air before the climate, not after it), and the air thickens
    /// steadily as Io's giant nears the star, from 5.2 to 2.5 AU (1e-9 to
    /// 1.7e-4 bar). The vapour of the final climate differs from the one
    /// the climate was solved with by 0.4 % at most (at 2.5 AU): one pass
    /// suffices.
    ///
    /// [`TRACE_SO2_BAR`]: crate::planet::atmosphere::TRACE_SO2_BAR
    #[test]
    fn a_frost_holds_its_air_at_its_vapour() {
        use crate::planet::atmosphere::{
            TRACE_SO2_BAR, bar_per_kg, volcanic_so2_frost_bar, volcanic_so2_supply,
            volcanic_so2_vapour,
        };
        let io = moon(0.015, 0.0, MoonBirth::Disk, JUPITER_KG, 4.217e8, 0.0041);
        let (mut last, mut sides) = (0.0, [false; 2]);
        for orbit in [5.2, 4.6, 4.2, 4.0, 3.9, 3.8, 3.6, 3.2, 2.8, 2.5] {
            let m = model(orbit, &io);
            let s = m.surface.expect("surface");
            let context = planet_at(orbit).with_tide(io.tide);
            let loss = crate::planet::loss_context(&context, &m.inputs, &m.interior);
            let inventory = volcanic_so2_frost_bar(
                volcanic_so2_supply(&m.heat, &m.inputs),
                &loss,
                bar_per_kg(m.inputs.mass_earth, &m.interior),
            );
            let c = s.climate;
            let vapour =
                volcanic_so2_vapour(inventory, c.equilibrium_k, c.temperature_k).expect("a frost");
            let so2 = s.gases.so2;
            assert!(
                (so2 / vapour - 1.0).abs() < 0.01,
                "{orbit} AU: {so2:e} against {vapour:e}"
            );
            assert!(so2 > last, "{orbit} AU: {so2:e} after {last:e}");
            last = so2;
            sides[usize::from(vapour > TRACE_SO2_BAR)] = true;
        }
        assert_eq!(sides, [true, true]);
    }

    /// Titan's 1.3 to 1.6 bar and Triton's N₂ in equilibrium with its
    /// frost (1e-5 to 3e-5 bar) outlast what their giants' plasma and the
    /// Sun's ionisation take of them.
    #[test]
    fn titan_and_triton_keep_their_air() {
        for seed in 1..=20 {
            let titan = archetype_moon("titan-like", seed);
            let p = pressure(&titan);
            assert!((1.3..=1.6).contains(&p), "Titan seed {seed}: {p} {titan:?}");
            let triton = archetype_moon("triton-like", seed);
            let n2 = triton.surface.expect("surface").gases.n2;
            assert!(
                (1e-5..=3e-5).contains(&n2),
                "Triton seed {seed}: {n2:e} {triton:?}"
            );
        }
    }

    #[test]
    fn ganymede_is_airless() {
        let m = model(
            5.2,
            &moon(0.0248, 0.45, MoonBirth::Disk, JUPITER_KG, 1.0704e9, 0.0013),
        );
        assert!(pressure(&m) < 1e-3, "{m:?}");
        assert!(ocean_km(&m) < 400.0, "{:?}", m.ice_shell);
    }

    #[test]
    fn callisto_is_airless() {
        let m = model(
            5.2,
            &moon(0.018, 0.5, MoonBirth::Disk, JUPITER_KG, 1.883e9, 0.0074),
        );
        assert!(pressure(&m) < 1e-3, "{m:?}");
        assert!(ocean_km(&m) < 400.0, "{:?}", m.ice_shell);
    }

    #[test]
    fn io_is_a_heat_pipe() {
        let m = model(
            5.2,
            &moon(0.015, 0.0, MoonBirth::Disk, JUPITER_KG, 4.217e8, 0.0041),
        );
        assert_eq!(m.surface.unwrap().regime, Regime::HeatPipe, "{m:?}");
        assert!(pressure(&m) < 1e-3, "{m:?}");
    }

    #[test]
    fn an_icy_heat_pipe_moon_stays_an_ice_world() {
        let io = model(
            5.2,
            &moon(0.015, 0.0, MoonBirth::Disk, JUPITER_KG, 4.217e8, 0.0041),
        );
        let surface = io.surface.as_ref().expect("Io has a surface");
        assert_eq!(surface.regime, Regime::HeatPipe);
        let rocky = crate::planet::classify(&io.inputs, &io.interior, 1.77, Some(surface));
        assert_eq!(rocky, PlanetClass::VolcanicWorld);
        // The same body with 30 % water and a ocean kept is icy: an ice
        // world, whatever its regime.
        let mut icy = io.inputs.clone();
        icy.water_fraction = 0.3;
        let mut wet = *surface;
        wet.water.oceans = 1.0;
        let class = crate::planet::classify(&icy, &io.interior, 1.77, Some(&wet));
        assert_eq!(class, PlanetClass::IceWorld);
    }

    #[test]
    fn io_is_a_volcanic_world() {
        let m = model(
            5.2,
            &moon(0.015, 0.0, MoonBirth::Disk, JUPITER_KG, 4.217e8, 0.0041),
        );
        assert_eq!(m.class, PlanetClass::VolcanicWorld, "{m:?}");
    }

    #[test]
    fn europa_has_an_ocean() {
        let m = model(
            5.2,
            &moon(0.008, 0.08, MoonBirth::Disk, JUPITER_KG, 6.709e8, 0.009),
        );
        assert!(ocean_km(&m) > 0.0, "{m:?}");
    }

    /// Triton's nitrogen frost brightens it and its air is the vapour of
    /// that frost: 38 K and 1.4e-5 bar of N₂ at Voyager 2's flyby (Tyler
    /// et al. 1989, doi:10.1126/science.246.4936.1466), a Bond albedo of
    /// about 0.75 (spec goal 6, issue #464).
    #[test]
    fn triton_keeps_a_thin_nitrogen_air() {
        let m = model(
            30.07,
            &moon(0.00359, 0.3, MoonBirth::Captured, NEPTUNE_KG, 3.548e8, 0.0),
        );
        let s = m.surface.expect("surface");
        let (t, n2, a) = (s.climate.temperature_k, s.gases.n2, s.climate.bond_albedo);
        println!("Triton: {t:.2} K, N2 {n2:.3e} bar, Bond albedo {a:.3}");
        assert!((35.0..=41.0).contains(&t), "{s:?}");
        assert!((1e-5..=3e-5).contains(&n2), "{s:?}");
        assert!(a >= 0.6, "{s:?}");
    }

    /// Triton at 38 K (37-39) under 1.4e-5 bar of N₂ (1e-5 to 2e-5) on
    /// every seed of its archetype (spec goal 6, issue #529): its air is
    /// the vapour of its β-N₂ frost (Fray & Schmitt 2009, through
    /// [`super::super::atmosphere`]), at the temperature that frost's
    /// emissivity gives it.
    #[test]
    fn triton_is_38_kelvin() {
        for seed in 1..=40 {
            let m = archetype_moon("triton-like", seed);
            let s = m.surface.expect("surface");
            let (t, n2) = (s.climate.temperature_k, s.gases.n2);
            assert!((37.0..=39.0).contains(&t), "seed {seed}: {t} K {s:?}");
            assert!((1e-5..=2e-5).contains(&n2), "seed {seed}: {n2:e} bar {s:?}");
            assert!(s.climate.bond_albedo >= 0.6, "seed {seed}: {s:?}");
        }
    }

    /// A moon's albedo comes from its own cover, not its planet's
    /// (Review Focus 4): Titan under its haze (0.27, Li et al. 2011), an
    /// airless icy Europa (0.68, Buratti & Veverka 1983) and Io under its
    /// frosts (0.63, Simonelli & Veverka 1988; all from memory, not
    /// checked), round one star. Io's CO₂ frost went with its air to
    /// Jupiter's plasma ([`super::super::non_thermal_loss`]); the SO₂
    /// frost of its volcanoes brightens it
    /// ([`super::super::atmosphere::volcanic_so2_frost_bar`]).
    #[test]
    fn a_moon_uses_its_own_cover() {
        let titan = model(9.54, &titan());
        let europa = model(
            5.2,
            &moon(0.008, 0.08, MoonBirth::Disk, JUPITER_KG, 6.709e8, 0.009),
        );
        let io = model(
            5.2,
            &moon(0.015, 0.0, MoonBirth::Disk, JUPITER_KG, 4.217e8, 0.0041),
        );
        let albedo = |m: &PlanetModel| m.surface.expect("surface").climate.bond_albedo;
        let (t, e, i) = (albedo(&titan), albedo(&europa), albedo(&io));
        println!("Bond albedo: Titan {t:.3}, Europa {e:.3}, Io {i:.3}");
        assert!((0.2..=0.35).contains(&t), "Titan {t}: {titan:?}");
        assert!((0.5..=0.75).contains(&e), "Europa {e}: {europa:?}");
        assert!((0.4..=0.7).contains(&i), "Io {i}: {io:?}");
        assert!(t != e && e != i && t != i, "{t} {e} {i}");
    }

    #[test]
    fn the_moon_is_airless_and_stagnant() {
        let m = model(
            1.0,
            &moon(
                0.0123,
                0.0,
                MoonBirth::Impact,
                EARTH_HOST_KG,
                3.844e8,
                0.0549,
            ),
        );
        assert!(pressure(&m) < 1e-6, "{m:?}");
        assert_eq!(m.surface.unwrap().regime, Regime::Stagnant, "{m:?}");
    }

    #[test]
    fn enceladus_is_its_size() {
        // Its ocean needs the heat of its resonance with Dione, which a
        // lone moon has not: see `system::tests::enceladus_shell_and_ocean`.
        let m = model(
            9.54,
            &moon(1.8e-5, 0.5, MoonBirth::Disk, SATURN_KG, 2.3802e8, 0.0047),
        );
        let km = m.interior.radius_earth * crate::exomoon::EARTH_M / 1e3;
        assert!((km / 252.0 - 1.0).abs() < 0.1, "{km} km");
    }

    #[test]
    fn a_moon_model_is_deterministic() {
        let t = titan();
        let o = InputOverrides::default();
        let a = derive_moon_model(7, "p2/m1", &planet_at(9.54), &t, &o);
        let b = derive_moon_model(7, "p2/m1", &planet_at(9.54), &t, &o);
        assert_eq!(a, b);
    }

    #[test]
    fn a_warm_disk_moon_loses_its_hypervolatiles_and_a_captured_one_keeps_them() {
        let o = InputOverrides::default();
        let ganymede = moon(0.0248, 0.45, MoonBirth::Disk, JUPITER_KG, 1.0704e9, 0.0013);
        let warm = derive_moon_model(1, "p1/m1", &planet_at(5.2), &ganymede, &o);
        let v = warm.inputs.volatiles;
        assert_eq!((v.nh3, v.ch4), (0.0, 0.0));
        assert!(v.n2 > 0.0 && v.n2 < 1e-5, "{v:?}");
        let captured = MoonFormation {
            birth: MoonBirth::Captured,
            ..ganymede
        };
        let kept = derive_moon_model(1, "p1/m1", &planet_at(5.2), &captured, &o);
        assert!(
            kept.inputs.volatiles.nh3 > 1e-3,
            "{:?}",
            kept.inputs.volatiles
        );
        // Everything else is drawn alike.
        assert_eq!(
            kept.inputs.core_mass_fraction,
            warm.inputs.core_mass_fraction
        );
        assert!(keeps_hypervolatiles(MoonBirth::Disk, 83.0));
        assert!(!keeps_hypervolatiles(MoonBirth::Disk, 112.0));
    }

    /// A moon stripped of its hypervolatiles holds no ammonia, so its ice
    /// melts at 273 K; one without water has no shell; neither gives a
    /// NaN (Review Focus 3 of the lot 3 plan).
    #[test]
    fn a_body_without_ammonia_melts_at_273() {
        use super::super::ice_shell::liquidus_k;
        let o = InputOverrides::default();
        let ganymede = moon(0.0248, 0.45, MoonBirth::Disk, JUPITER_KG, 1.0704e9, 0.0013);
        let stripped = derive_moon_model(1, "p1/m1", &planet_at(5.2), &ganymede, &o);
        assert_eq!(stripped.inputs.volatiles.nh3, 0.0);
        let nh3_of_ice = stripped.inputs.volatiles.nh3 / stripped.inputs.water_fraction;
        assert_eq!(liquidus_k(nh3_of_ice), 273.0);
        let shell = stripped.ice_shell.expect("an icy moon has a shell");
        assert!(
            shell.shell_km.is_finite() && shell.ocean_km.is_finite(),
            "{shell:?}"
        );
        assert!(stripped.moment_of_inertia.is_finite(), "{stripped:?}");
        let io = model(
            5.2,
            &moon(0.015, 0.0, MoonBirth::Disk, JUPITER_KG, 4.217e8, 0.0041),
        );
        assert_eq!(io.ice_shell, None);
        assert!(io.moment_of_inertia.is_finite(), "{io:?}");
        assert!(!liquidus_k(f64::NAN).is_nan());
    }
}
