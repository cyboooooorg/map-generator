//! Life (part 2 of the physical planet model, issue #423): whether it
//! arose where liquid water lasted, and the oxygen it gives the air.
use serde::{Deserialize, Serialize};

use super::{Climate, Gases, PlanetInputs, SurfaceLiquid};

/// Age, Gyr, from which a world with liquid water can have life.
/// Calibration, no source: the spec's "order of a billion years".
pub const LIFE_MIN_GYR: f64 = 1.0;

/// Chance that life arises on a world that allows it: 1/3. A game
/// choice, set with the user on 2026-11-12. No measured rate exists (we
/// know one case); this is not a result of science.
pub const LIFE_CHANCE: f64 = 1.0 / 3.0;

/// Age, Gyr after formation, from which oxygenic life has oxygenated the
/// air: Earth's Great Oxidation Event, about 2.4 Ga, 2.2 Gyr after the
/// Earth formed 4.567 Ga (Lyons, Reinhard & Planavsky 2014,
/// doi:10.1038/nature13068: 2.3 to 2.4 Ga, checked in the authors' EGU
/// 2014 abstract, the Nature text not read).
pub const OXYGEN_AFTER_GYR: f64 = 2.2;

/// O₂ over N₂ in an oxygenated air. Calibration on Earth: 0.21 over 0.78.
pub const O2_PER_N2: f64 = 0.27;

/// What life did to a world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Life {
    /// Life arose.
    pub arose: bool,
    /// Its photosynthesis has put oxygen in the air.
    pub oxygenated: bool,
}

/// Whether life arose: the surface liquid is water now, the system is at
/// least [`LIFE_MIN_GYR`] old and `inputs.life_draw < LIFE_CHANCE`. It is
/// oxygenated once the system is [`OXYGEN_AFTER_GYR`] old.
pub fn life(inputs: &PlanetInputs, climate: &Climate, age_gyr: f64) -> Life {
    let arose = climate.liquid == SurfaceLiquid::Water
        && age_gyr >= LIFE_MIN_GYR
        && inputs.life_draw < LIFE_CHANCE;
    Life {
        arose,
        oxygenated: arose && age_gyr >= OXYGEN_AFTER_GYR,
    }
}

/// The air of an oxygenated world: O₂ at [`O2_PER_N2`] of the N₂, and the
/// methane gone (oxidised). Other gases are unchanged.
///
/// It runs after the climate: `Surface.gases` is the air after life, but
/// `Surface.climate` was solved with the air before it, methane
/// included. Removing a greenhouse gas would cool the surface a little;
/// the temperature is not solved again (a known simplification: on
/// Earth the model's abiotic air has no methane anyway).
pub fn oxygenate(gases: Gases, life: &Life) -> Gases {
    if !life.oxygenated {
        return gases;
    }
    Gases {
        o2: O2_PER_N2 * gases.n2,
        ch4: 0.0,
        ..gases
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::reference::{derive_at, model_inputs, reference};
    use crate::planet::{Context, climate};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Climate and air of a reference body with `draw`, at `age_gyr`.
    fn world(name: &str, draw: f64, age_gyr: f64) -> (Life, Gases) {
        let b = reference(name);
        let inputs = PlanetInputs {
            life_draw: draw,
            ..model_inputs(b)
        };
        let mut d = derive_at(&inputs, b.orbit_au);
        d.context.age_gyr = age_gyr;
        let (gases, c) = climate(
            &d.context,
            d.gases,
            &d.water,
            d.regime,
            d.heat_flux_w_m2,
            d.water_bar,
        );
        let l = life(&inputs, &c, age_gyr);
        (l, oxygenate(gases, &l))
    }

    #[test]
    fn earth_has_oxygenic_life() {
        let (l, g) = world("Earth", 0.0, 4.6);
        assert!(l.arose && l.oxygenated, "{l:?}");
        assert!((0.15..=0.3).contains(&g.o2), "{g:?}");
        assert_eq!(g.ch4, 0.0);
    }

    #[test]
    fn a_young_earth_is_lifeless() {
        let (l, g) = world("Earth", 0.0, 0.5);
        assert!(!l.arose && !l.oxygenated, "{l:?}");
        assert_eq!(g.o2, 0.0);
    }

    #[test]
    fn life_without_oxygen_before_the_oxidation() {
        let (l, g) = world("Earth", 0.0, 1.5);
        assert!(l.arose && !l.oxygenated, "{l:?}");
        assert_eq!(g.o2, 0.0);
    }

    #[test]
    fn a_habitable_world_can_be_lifeless() {
        let (l, _) = world("Earth", 0.99, 4.6);
        assert!(!l.arose, "{l:?}");
    }

    #[test]
    fn no_water_no_life() {
        for name in ["Venus", "Mars"] {
            for draw in [0.0, 0.5, 0.99] {
                let (l, _) = world(name, draw, 4.6);
                assert!(!l.arose, "{name} {draw}: {l:?}");
            }
        }
    }

    /// Part 1's draws for seed 7 round the Sun at 1 AU, captured from
    /// `main` before `life_draw` existed; the next draw of that stream
    /// is the life draw.
    #[test]
    fn life_draw_is_the_last_draw() {
        let c: Context = crate::planet::reference::sun_at(1.0);
        let mut rng = StdRng::seed_from_u64(7);
        let i = crate::planet::draw_inputs(&c, 1.0, &mut rng);
        // Fields that go through ln, cos or powf (the normal draws, the
        // log-uniform water, the log-normal scatter) may differ in their
        // last bits between platforms: 1e-12 relative.
        let close = |got: f64, want: f64| {
            assert!((got - want).abs() <= 1e-12 * want.abs(), "{got} {want}");
        };
        close(i.core_mass_fraction, 0.2785878209101608);
        close(i.water_fraction, 0.00022607183544463648);
        assert_eq!(i.envelope_fraction, 0.0);
        close(i.volatiles.co2, 0.0005830938999623686);
        close(i.volatiles.n2, 1.1407820714593542e-6);
        close(i.volatiles.ch4, 1.0841261800592375e-6);
        close(i.volatiles.nh3, 1.2976472772934134e-6);
        close(i.volatiles.s, 0.00013041927131499377);
        close(i.radiogenic, 0.6204511799069944);
        // The life draw is a plain uniform: exact.
        assert_eq!(i.life_draw, 0.7870037668293781);
        assert!((0.0..1.0).contains(&i.life_draw));
    }
}
