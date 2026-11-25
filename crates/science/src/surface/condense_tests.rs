//! Tests of the condensation of N2 and CH4 at the surface, ported from
//! `legacy-final`'s `planet/atmosphere.rs` and `planet/climate.rs`, and
//! the observations they hold (Titan's air over its seas, Triton's frost).

use super::*;
use crate::radiation::test_points::{EARTH_AIR, TITAN_SURFACE_AIR, TITAN_SURFACE_K};
use crate::surface::{PHASE_BAND, methane_sea_vapour};
use crate::units::Bar;
use crate::volatiles::{methane_saturation_pressure, nitrogen_frost_pressure};

/// Definition: the tolerance of an identity.
const TIGHT: f64 = 1e-9;

/// Definition: the tolerance of a sum of partial pressures that must give
/// back the inventory, 1e-12 bar.
const SUM_TOLERANCE: f64 = 1e-12;

/// Definition: an inventory of methane that floods a surface, 1 bar.
const FLOODING_CH4: Bar = Bar(1.0);

/// Definition: the legacy test's methane-ice inventory, N2 1e-3 bar and
/// CH4 1 bar, at 50 K, far under any sea's freezing point.
const ICE_INVENTORY: PartialPressures = PartialPressures {
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

fn titan_inventory(ch4: f64) -> PartialPressures {
    PartialPressures {
        ch4,
        ..TITAN_SURFACE_AIR
    }
}

fn sum(a: &PartialPressures, b: &PartialPressures) -> [f64; 7] {
    [
        a.n2 + b.n2,
        a.co2 + b.co2,
        a.h2o + b.h2o,
        a.ch4 + b.ch4,
        a.nh3 + b.nh3,
        a.so2 + b.so2,
        a.o2 + b.o2,
    ]
}

fn parts(p: &PartialPressures) -> [f64; 7] {
    [p.n2, p.co2, p.h2o, p.ch4, p.nh3, p.so2, p.o2]
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
    assert!(
        (air.ch4 - cap).abs() <= TIGHT * cap,
        "{} against {cap}",
        air.ch4
    );
    assert!(
        (condensate.ch4 - (FLOODING_CH4.0 - air.ch4)).abs() <= SUM_TOLERANCE,
        "{condensate:?}"
    );
    assert_eq!(condensate.n2, 0.0);
    assert_eq!(air.n2, TITAN_SURFACE_AIR.n2);

    let measured = condense(&TITAN_SURFACE_AIR, t);
    assert_eq!(measured.air, TITAN_SURFACE_AIR);
    assert_eq!(measured.condensate, PartialPressures::default());
}

/// Methane over its own ice keeps all of its saturation pressure
/// (activity 1, a pure solid); over a sea, `SEA_METHANE_SHARE` of it
/// (legacy `climate` l.3035, issue #530).
#[test]
fn methane_ice_holds_its_full_saturation() {
    let Condensed { air, condensate } = condense(&ICE_INVENTORY, ICE_K);
    let freezes = sea_freezes(Bar(air.n2), &condensate);
    assert!(ICE_K.0 < freezes.0 - PHASE_BAND.0, "{freezes:?}");
    let full = methane_saturation_pressure(ICE_K).0 / crate::constants::PASCALS_PER_BAR;
    assert!((air.ch4 / full - 1.0).abs() < TIGHT, "{air:?} {full:e}");

    let Condensed { air, .. } = condense(&ICE_INVENTORY, SEA_K);
    let sea = methane_sea_vapour(SEA_K).0;
    assert!((air.ch4 / sea - 1.0).abs() < TIGHT, "{air:?} {sea:e}");
}

/// Nitrogen condenses first, over its own frost (activity 1): a
/// Triton-like inventory keeps its frost pressure in the air.
#[test]
fn nitrogen_condenses_first() {
    let inventory = PartialPressures {
        n2: TRITON_N2.0,
        ..PartialPressures::default()
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
            let inventory = PartialPressures {
                n2,
                ch4,
                ..EARTH_AIR
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
                        "{n2} N2 {ch4} CH4 at {t} K: {air:?} {condensate:?}"
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
/// NH3, SO2 and O2 of `air` pass through.
#[test]
fn condensed_air_changes_only_what_condenses() {
    let air = PartialPressures {
        n2: 0.5,
        co2: 0.1,
        h2o: 0.02,
        ch4: 0.3,
        nh3: 0.004,
        so2: 0.005,
        o2: 0.006,
        ar: 0.0,
    };
    let inventory = titan_inventory(FLOODING_CH4.0);
    let t = Kelvin(TITAN_SURFACE_K);
    let condensed = condensed_air(&air, &inventory, t);
    let kept = condense(&inventory, t).air;
    assert_eq!(condensed.n2, kept.n2);
    assert_eq!(condensed.ch4, kept.ch4);
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
    assert!(titan_like_air(&TITAN_SURFACE_AIR));
    let triton = PartialPressures {
        n2: TRITON_PRESSURE.0,
        ch4: CH4_TRACE.0,
        ..PartialPressures::default()
    };
    assert!(!titan_like_air(&triton));
    assert!(!titan_like_air(&EARTH_AIR));
}
