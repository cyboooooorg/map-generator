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
//! Known departures (issue #463): Enceladus has no ocean (the icy `k₂/Q`
//! gives it a quarter of the tidal heat it radiates), and the oceans of
//! Ganymede, Titan and Callisto depend on the radiogenic draw (see
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
/// Titan its 1.5 bar or so of N₂ and a few percent of CH₄.
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
    planet_model(&context, inputs)
}

/// `inputs` without the N₂, NH₃ and CH₄ its ices carry: the N₂ keeps the
/// rock's share of the rule (whatever factor scaled it), NH₃ and CH₄, all
/// ice-borne, go.
fn without_hypervolatiles(inputs: PlanetInputs) -> PlanetInputs {
    let refractory = (1.0 - inputs.water_fraction - inputs.envelope_fraction).max(0.0);
    let rule = volatile_rule(refractory, inputs.water_fraction);
    let rock = volatile_rule(refractory, 0.0);
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
    use crate::planet::PlanetClass;
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

    #[test]
    fn titan_comes_out_a_hydrocarbon_world() {
        let m = model(9.54, &titan());
        let s = m.surface.expect("surface");
        assert_eq!(m.class, PlanetClass::HydrocarbonWorld, "{m:?}");
        let p = s.gases.total();
        assert!((1.0..2.5).contains(&p), "{s:?}");
        let ch4 = s.gases.ch4 / p;
        assert!((0.01..0.10).contains(&ch4), "{s:?}");
        assert!(ocean_km(&m) < 400.0, "{:?}", m.ice_shell);
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

    #[test]
    fn triton_keeps_a_thin_nitrogen_air() {
        let m = model(
            30.07,
            &moon(0.00359, 0.3, MoonBirth::Captured, NEPTUNE_KG, 3.548e8, 0.0),
        );
        let n2 = m.surface.expect("surface").gases.n2;
        assert!((1e-6..1e-2).contains(&n2), "{m:?}");
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
        // Its ocean is a known departure (issue #463): the icy k₂/Q gives
        // it a quarter of the tidal heat it radiates, and the conductive
        // shell takes the whole water layer.
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
}
