//! Tests of the condensation of N2 and CH4 at the surface, ported from
//! `legacy-final`'s `planet/atmosphere.rs` and `planet/climate.rs`, and
//! the observations they hold (Titan's air over its seas, Triton's frost).

use super::*;
use crate::radiation::test_points::{EARTH_AIR, TITAN_SURFACE_AIR, TITAN_SURFACE_K};
use crate::surface::{PHASE_BAND, methane_sea_vapour};
use crate::units::Bar;
use crate::volatiles::{
    ARGON_TRIPLE_PRESSURE, ARGON_TRIPLE_TEMPERATURE, argon_frost_pressure,
    methane_saturation_pressure, nitrogen_frost_pressure,
};

/// Definition: the tolerance of an identity.
const TIGHT: f64 = 1e-9;

/// Definition: the tolerance of a sum of weights that must give back the
/// inventory, 1e-12 bar.
const SUM_TOLERANCE: f64 = 1e-12;

/// Definition: an inventory of methane that floods a surface, 1 bar.
const FLOODING_CH4: Bar = Bar(1.0);

/// Definition: the legacy test's methane-ice inventory, N2 1e-3 bar and
/// CH4 1 bar (weights), at 50 K, far under any sea's freezing point.
const ICE_INVENTORY: Weights = Weights {
    n2: 1e-3,
    co2: 0.0,
    h2o: 0.0,
    ch4: 1.0,
    nh3: 0.0,
    so2: 0.0,
    o2: 0.0,
    ar: 0.0,
};
/// Definition: see [`ICE_INVENTORY`].
const ICE_K: Kelvin = Kelvin(50.0);
/// Definition: the legacy test's sea, 95 K, over Titan's sea's freezing
/// point.
const SEA_K: Kelvin = Kelvin(95.0);

/// Source: Triton's surface temperature, 38 +- 1 K (Tryka et al. 1993,
/// *Science* 261, 751, and 1994, *Icarus* 112, 513, as
/// `docs/science/planet-atmospheres.md` cites them, issue #673).
const TRITON_K: Kelvin = Kelvin(38.0);
/// Definition: a Triton-like inventory of nitrogen, 10 bar, far more than
/// its frost holds in the air.
const TRITON_N2: Bar = Bar(10.0);

/// Definition: the inventories of the sweep, bar.
const SWEEP_BAR: [f64; 4] = [0.0, 1e-3, 1.0, 10.0];
/// Definition: the temperatures of the sweep, 30 to 200 K by 5 K.
const SWEEP_K: (f64, f64, f64) = (30.0, 200.0, 5.0);

/// Source: Triton's surface pressure, 14 +- 1 ubar, 1.4e-5 bar (Voyager 2
/// radio occultation, Gurrola 1995, as quoted by Sicardy et al. 2024,
/// arXiv:2402.02476; as `volatiles_tests.rs` cites it).
const TRITON_PRESSURE: Bar = Bar(1.4e-5);
/// Definition: a methane trace in a Triton-like air, 1e-9 bar.
const CH4_TRACE: Bar = Bar(1e-9);

/// Titan's measured air ([`TITAN_SURFACE_AIR`], by volume) as weights.
fn titan_air() -> Weights {
    TITAN_SURFACE_AIR.weights()
}

/// Titan's measured air with `ch4` bar of methane by weight.
fn titan_inventory(ch4: f64) -> Weights {
    Weights { ch4, ..titan_air() }
}

fn sum(a: &Weights, b: &Weights) -> [f64; 8] {
    [
        a.n2 + b.n2,
        a.co2 + b.co2,
        a.h2o + b.h2o,
        a.ch4 + b.ch4,
        a.nh3 + b.nh3,
        a.so2 + b.so2,
        a.o2 + b.o2,
        a.ar + b.ar,
    ]
}

fn parts(p: &Weights) -> [f64; 8] {
    [p.n2, p.co2, p.h2o, p.ch4, p.nh3, p.so2, p.o2, p.ar]
}

/// Titan's seas (study 25, "Condensation at the surface"; the 2d-2
/// carry-over): with more methane than its air holds over a sea, the air
/// keeps the sea's vapour ([`methane_sea_vapour`], 0.0845 bar at
/// 93.65 K) and the rest is the sea; its measured methane (0.0829 ±
/// 0.0026 bar, Niemann et al. 2010) is under that cap and condenses
/// nothing; its nitrogen never condenses.
#[test]
fn titans_air_holds_its_sea_vapour() {
    let t = Kelvin(TITAN_SURFACE_K);
    let flooded = titan_inventory(FLOODING_CH4.0);
    let Condensed { air, condensate } = condense(&flooded, t);
    println!("Titan, 1 bar of CH4 at 93.65 K: air {air:?}, condensate {condensate:?}");
    let cap = methane_sea_vapour(t).0;
    let held = air.partial_pressures().ch4;
    assert!((held - cap).abs() <= TIGHT * cap, "{held} against {cap}");
    assert!(
        (condensate.ch4 - (FLOODING_CH4.0 - air.ch4)).abs() <= SUM_TOLERANCE,
        "{condensate:?}"
    );
    assert_eq!(condensate.n2, 0.0);
    assert_eq!(air.n2, titan_air().n2);

    let measured = condense(&titan_air(), t);
    assert_eq!(measured.air, titan_air());
    assert_eq!(measured.condensate, Weights::default());
}

/// Methane over its own ice keeps all of its saturation pressure
/// (activity 1, a pure solid); over a sea, `SEA_METHANE_SHARE` of it
/// (legacy `climate` l.3035, issue #530).
#[test]
fn methane_ice_holds_its_full_saturation() {
    let Condensed { air, condensate } = condense(&ICE_INVENTORY, ICE_K);
    let freezes = sea_freezes(Bar(air.partial_pressures().n2), &condensate);
    assert!(ICE_K.0 < freezes.0 - PHASE_BAND.0, "{freezes:?}");
    let full = methane_saturation_pressure(ICE_K).0 / crate::constants::PASCALS_PER_BAR;
    let held = air.partial_pressures().ch4;
    assert!((held / full - 1.0).abs() < TIGHT, "{air:?} {full:e}");

    let Condensed { air, .. } = condense(&ICE_INVENTORY, SEA_K);
    let sea = methane_sea_vapour(SEA_K).0;
    let held = air.partial_pressures().ch4;
    assert!((held / sea - 1.0).abs() < TIGHT, "{air:?} {sea:e}");
}

/// Nitrogen condenses first, over its own frost (activity 1): a
/// Triton-like inventory keeps its frost pressure in the air.
#[test]
fn nitrogen_condenses_first() {
    let inventory = Weights {
        n2: TRITON_N2.0,
        ..Weights::default()
    };
    let Condensed { air, condensate } = condense(&inventory, TRITON_K);
    let frost = nitrogen_frost_pressure(TRITON_K)
        .expect("under the triple point")
        .0
        / crate::constants::PASCALS_PER_BAR;
    println!("Triton, 10 bar of N2 at 38 K: air {:.3e} bar", air.n2);
    assert!((air.n2 / frost - 1.0).abs() < TIGHT, "{air:?} {frost:e}");
    assert!(
        (condensate.n2 - (TRITON_N2.0 - air.n2)).abs() <= SUM_TOLERANCE,
        "{condensate:?}"
    );
}

/// Nothing is made or lost: the air and the condensate give back the
/// inventory, and no part is negative or NaN.
#[test]
fn condensation_keeps_the_inventory() {
    let (floor, ceiling, step) = SWEEP_K;
    for n2 in SWEEP_BAR {
        for ch4 in SWEEP_BAR {
            let inventory = Weights {
                n2,
                ch4,
                ar: ch4,
                ..EARTH_AIR.weights()
            };
            let mut k = 0.0;
            loop {
                let t = floor + k * step;
                if t > ceiling {
                    break;
                }
                let Condensed { air, condensate } = condense(&inventory, Kelvin(t));
                let total = sum(&air, &condensate);
                for (got, want) in total.iter().zip(parts(&inventory)) {
                    assert!(
                        (got - want).abs() <= SUM_TOLERANCE,
                        "{n2} N2 {ch4} CH4 and Ar at {t} K: {air:?} {condensate:?}"
                    );
                }
                for p in parts(&air).into_iter().chain(parts(&condensate)) {
                    assert!(!p.is_nan() && p >= 0.0, "{t} K: {air:?} {condensate:?}");
                }
                k += 1.0;
            }
        }
    }
}

/// `condensed_air` changes only the gases that condense: the CO2, H2O,
/// NH3, SO2 and O2 of `air` pass through; its N2, CH4 and Ar are what
/// the inventory leaves over those of `air`.
#[test]
fn condensed_air_changes_only_what_condenses() {
    let air = Weights {
        n2: 0.5,
        co2: 0.1,
        h2o: 0.02,
        ch4: 0.3,
        nh3: 0.004,
        so2: 0.005,
        o2: 0.006,
        ar: 0.007,
    };
    let inventory = Weights {
        ar: FLOODING_CH4.0,
        ..titan_inventory(FLOODING_CH4.0)
    };
    let t = Kelvin(TITAN_SURFACE_K);
    let condensed = condensed_air(&air, &inventory, t);
    let kept = condense(&condensables(&air, &inventory), t).air;
    assert_eq!(condensed.n2, kept.n2);
    assert_eq!(condensed.ch4, kept.ch4);
    assert_eq!(condensed.ar, kept.ar);
    assert_eq!(
        [
            condensed.co2,
            condensed.h2o,
            condensed.nh3,
            condensed.so2,
            condensed.o2
        ],
        [air.co2, air.h2o, air.nh3, air.so2, air.o2]
    );
}

/// A Titan's air is dense and holds methane; a Triton's thin nitrogen
/// air with a trace of methane is not one, nor is Earth's.
#[test]
fn titan_like_air_is_dense_and_methane_bearing() {
    assert!(titan_like_air(&titan_air()));
    let triton = Weights {
        n2: TRITON_PRESSURE.0,
        ch4: CH4_TRACE.0,
        ..Weights::default()
    };
    assert!(!titan_like_air(&triton));
    assert!(!titan_like_air(&EARTH_AIR.weights()));
}

/// Source: the argon a Triton made into its air, 1.3 to 1.8 Pa, and a
/// Pluto, 0.8 to 1.1 Pa, at their N2 ices' 37.2 and 37.1 K, and the
/// frost that holds them, 0.022 and 0.020 Pa by Xiao et al. 2025's eq. 55
/// (study 29b, "Checks on the cold bodies", the table's columns).
const COLD_BODIES: [(f64, (f64, f64), f64); 2] =
    [(37.2, (1.3, 1.8), 0.022), (37.1, (0.8, 1.1), 0.020)];

/// Definition: the tolerance of a value printed to two digits, 2.5 %.
const TWO_DIGITS: f64 = 0.025;

/// Source: the largest share of a Triton's or a Pluto's air its argon
/// can be at activity 1, 2.5 % (study 29b: 1.6 to 2.5 % between the two
/// fits); the check of the settle tests takes 5 %.
const COLD_ARGON_SHARE: f64 = 0.025;

/// Argon condenses as the third gas, over its own frost (activity 1, the
/// methane-over-its-ice rule): on Triton and Pluto the frost holds nearly
/// all the argon their rock made, and the air keeps eq. 55's pressure, a
/// small share of the N2's (study 29b).
#[test]
fn argon_condenses_over_its_own_frost_on_the_cold_bodies() {
    for (t, (low, high), frost) in COLD_BODIES {
        let t = Kelvin(t);
        let curve = argon_frost_pressure(t).expect("under the triple point").0;
        assert!((curve / frost - 1.0).abs() < TWO_DIGITS, "{t:?}: {curve}");
        for made in [low, high] {
            let inventory = Weights {
                n2: TRITON_N2.0,
                ar: made / crate::constants::PASCALS_PER_BAR,
                ..Weights::default()
            };
            let Condensed { air, condensate } = condense(&inventory, t);
            let cap = curve / crate::constants::PASCALS_PER_BAR;
            let p = air.partial_pressures();
            assert!((p.ar / cap - 1.0).abs() < TIGHT, "{t:?}: {air:?}");
            assert!(
                (condensate.ar - (inventory.ar - air.ar)).abs() <= SUM_TOLERANCE,
                "{condensate:?}"
            );
            // The N2 holds its own frost's pressure beside the argon: two
            // gases that condense together both sit at their caps, where
            // capping the N2 with all the argon still in the air would
            // leave it under its own.
            let n2_frost =
                nitrogen_frost_pressure(t).unwrap().0 / crate::constants::PASCALS_PER_BAR;
            assert!((p.n2 / n2_frost - 1.0).abs() < TIGHT, "{t:?}: {p:?}");
            let share = p.ar / p.total();
            println!(
                "{t:?}, {made} Pa of Ar: {:.3e} Pa in the air, {share:.4} of it",
                curve
            );
            assert!(share < COLD_ARGON_SHARE, "{t:?}: {share}");
        }
    }
}

/// Above its triple point argon's liquid takes more than the triple
/// pressure (68 891 Pa) to condense: less argon stays all in the air, up
/// to the critical point and beyond.
#[test]
fn argon_under_its_triple_pressure_stays_in_a_warm_air() {
    let ar = 0.99 * ARGON_TRIPLE_PRESSURE.0 / crate::constants::PASCALS_PER_BAR;
    let inventory = Weights {
        ar,
        ..Weights::default()
    };
    for t in [ARGON_TRIPLE_TEMPERATURE.0, 87.28, 120.0, 150.687, 300.0] {
        let Condensed { air, condensate } = condense(&inventory, Kelvin(t));
        assert_eq!(air.ar, ar, "{t} K");
        assert_eq!(condensate.ar, 0.0, "{t} K");
    }
}
