//! Tests of the surface liquids and their frost, ported from
//! `legacy-final`'s `planet/climate.rs` with the condensate given as an
//! input, and the observations they hold (Richardson et al.'s liquids,
//! Titan's seas and air, Earth's water, Io's frost).

use super::*;
use crate::constants::PASCALS_PER_BAR;
use crate::habitable::OCEAN_MASS;
use crate::radiation::test_points::{
    EARTH_AIR, EARTH_SURFACE_K, TITAN_SURFACE_AIR, TITAN_SURFACE_K, VENUS_PRESSURE_BAR,
    VENUS_SURFACE_K,
};
use crate::volatiles::{
    SULPHUR_DIOXIDE_TRIPLE_TEMPERATURE, nitrogen_frost_pressure, nitrogen_vapour_pressure,
    sulphur_dioxide_frost_pressure,
};
use crate::water::{TRIPLE_POINT_PRESSURE, boiling_temperature, ice_ih_melting_temperature};

/// Definition: the tolerance of an identity.
const TIGHT: f64 = 1e-9;

/// Definition: Titan's nitrogen in the legacy tests, 1.45 bar (its 1.467
/// bar less its methane, rounded).
const TITAN_N2: Bar = Bar(1.45);

/// Definition: a sea's surface of the legacy tests, 94 K (Titan's 93.65 K
/// rounded up).
const TITAN_SEA_K: Kelvin = Kelvin(94.0);

/// Definition: a Mars-like surface of the test cases (the legacy's):
/// temperature (K), CO₂ (bar) and water, in Earth oceans.
const MARS_LIKE: [f64; 3] = [210.0, 6e-3, 0.005];

/// Definition: a surface over nitrogen's triple point and under its
/// normal boiling point, 70 K (the legacy test's).
const NITROGEN_SEA_K: Kelvin = Kelvin(70.0);

/// Definition: a surface under nitrogen's triple point, 60 K (the legacy
/// test's).
const NITROGEN_FROST_K: Kelvin = Kelvin(60.0);

/// Definition: an Io-like surface under SO2's triple point, 110 K.
const IO_SURFACE_K: Kelvin = Kelvin(110.0);

/// Definition: one standard atmosphere, 1.01325 bar (101 325 Pa, exact),
/// the surface pressure of the test Earth.
const STANDARD_ATMOSPHERE_BAR: f64 = 1.013_25;

fn oceans(n: f64) -> Kilogram {
    Kilogram(n * OCEAN_MASS.0)
}

fn dry() -> Kilogram {
    Kilogram(0.0)
}

fn none() -> PartialPressures {
    PartialPressures::default()
}

/// Titan's seas on its ground, 1 bar of methane condensate.
fn seas() -> PartialPressures {
    PartialPressures {
        ch4: 1.0,
        ..PartialPressures::default()
    }
}

#[test]
fn a_trace_of_methane_makes_no_lake() {
    let trace = PartialPressures {
        n2: 1.5,
        ch4: 0.003,
        ..PartialPressures::default()
    };
    assert_ne!(
        liquid(TITAN_SEA_K, &trace, dry(), false, &none()),
        Some(Liquid::Hydrocarbons)
    );
    // Titan's air holds the vapour of its seas.
    let titan = PartialPressures {
        ch4: methane_sea_vapour(Kelvin(TITAN_SURFACE_K)).0,
        ..TITAN_SURFACE_AIR
    };
    assert_eq!(
        liquid(Kelvin(TITAN_SURFACE_K), &titan, dry(), false, &none()),
        Some(Liquid::Hydrocarbons)
    );
    // Its methane share, against Titan's 5.65 % (Niemann et al. 2010).
    let vapour = methane_sea_vapour(TITAN_SEA_K).0;
    let share = vapour / (TITAN_N2.0 + vapour);
    println!("Titan's CH4 share over a sea at 94 K: {share:.4}");
    assert!((0.03..0.06).contains(&share), "{share}");
}

#[test]
fn nitrogen_keeps_methane_liquid_below_its_triple_point() {
    // No nitrogen: a sea of half methane, half ethane, liquid down to
    // about 77 K.
    let bare = methane_freezes(Bar(0.0)).0;
    // Titan's 1.45 bar of N2: about a fifth of the liquid, and less
    // ethane, down to about 69 K.
    let titan = methane_freezes(TITAN_N2).0;
    // More nitrogen, colder liquid, down to the cap, where methane
    // freezes first.
    let floor = methane_freezes(Bar(100.0)).0;
    println!("methane_freezes: 0 bar {bare:.3} K, 1.45 bar {titan:.3} K, 100 bar {floor:.3} K");
    assert!((75.0..78.0).contains(&bare), "{bare}");
    assert!((67.0..71.0).contains(&titan), "{titan}");
    assert!(methane_freezes(Bar(0.5)).0 > titan);
    assert!(methane_freezes(Bar(10.0)).0 < titan);
    let cap = hydrocarbon_freezes(
        MAX_NITROGEN_IN_LIQUID,
        1.0 - SEA_METHANE_SHARE - MAX_NITROGEN_IN_LIQUID,
    )
    .0;
    assert!((floor - cap).abs() < TIGHT, "{floor} {cap}");
    assert!((66.0..69.0).contains(&floor), "{floor}");
    // A Titan at 89 K under 1 bar of N2 keeps its lakes.
    let air = PartialPressures {
        n2: 1.0,
        ch4: methane_sea_vapour(Kelvin(89.0)).0,
        ..PartialPressures::default()
    };
    assert_eq!(
        liquid(Kelvin(89.0), &air, dry(), false, &seas()),
        Some(Liquid::Hydrocarbons)
    );
    // Without nitrogen its ethane keeps it liquid at 89 K too, but not at
    // 75 K.
    let thin = PartialPressures { n2: 0.0, ..air };
    assert_eq!(
        liquid(Kelvin(89.0), &thin, dry(), false, &seas()),
        Some(Liquid::Hydrocarbons)
    );
    let cold = PartialPressures {
        ch4: methane_sea_vapour(Kelvin(75.0)).0,
        ..thin
    };
    assert_eq!(liquid(Kelvin(75.0), &cold, dry(), false, &seas()), None);
}

/// Nitrogen alone, condensed above its triple point: a sea of liquid
/// nitrogen; under it, a frost (issue #524).
#[test]
fn a_pure_nitrogen_sea_is_nitrogen() {
    let condensate = PartialPressures {
        n2: 0.5,
        ..PartialPressures::default()
    };
    let warm = NITROGEN_SEA_K;
    let air = PartialPressures {
        n2: bar(nitrogen_vapour_pressure(warm)),
        ..PartialPressures::default()
    };
    assert_eq!(
        liquid(warm, &air, dry(), false, &condensate),
        Some(Liquid::Nitrogen)
    );
    let cold = NITROGEN_FROST_K;
    let air = PartialPressures {
        n2: bar(nitrogen_frost_pressure(cold).expect("under the triple point")),
        ..PartialPressures::default()
    };
    assert_eq!(liquid(cold, &air, dry(), false, &condensate), None);
    let frost = frost(&condensate, Bar(0.0), Bar(0.0), cold, &air);
    assert!(frost.n2 > 0.0, "{frost:?}");
}

/// Nitrogen that condenses into a methane sea makes it nitrogen-rich,
/// liquid down to nitrogen's triple point: under the liquidus of
/// [`methane_freezes`] the sea is still liquid and its methane no frost
/// (issue #524).
#[test]
fn condensed_nitrogen_keeps_the_sea_liquid() {
    let t = NITROGEN_SEA_K;
    let condensate = PartialPressures {
        n2: 5.0,
        ch4: 8.0,
        ..PartialPressures::default()
    };
    let air = PartialPressures {
        n2: bar(nitrogen_vapour_pressure(t)),
        ch4: methane_sea_vapour(t).0,
        ..PartialPressures::default()
    };
    assert!(t.0 < methane_freezes(Bar(air.n2)).0, "{air:?}");
    assert_eq!(
        liquid(t, &air, dry(), false, &condensate),
        Some(Liquid::Hydrocarbons)
    );
    let frost = frost(&condensate, Bar(0.0), Bar(0.0), t, &air);
    assert_eq!(frost.ch4 + frost.n2, 0.0, "{frost:?}");
}

/// Ethane lowers the freezing point of a methane-nitrogen liquid:
/// Richardson, Hartwig & Leachman 2019's two liquids (F1: 61.0 % CH4,
/// 25.6 % C2H6, 13.4 % N2, frozen at 71.5 +- 0.5 K; F2: 46.9 %, 45.3 %,
/// 7.9 %, at 74.0 +- 0.5 K) within 2 K (issue #530).
#[test]
fn ethane_lowers_the_liquidus() {
    let f1 = hydrocarbon_freezes(0.134, 0.256).0;
    let f2 = hydrocarbon_freezes(0.079, 0.453).0;
    println!("Richardson F1: {f1:.3} K (71.5 K), F2: {f2:.3} K (74.0 K)");
    assert!((f1 - 71.5).abs() < 2.0, "{f1}");
    assert!((f2 - 74.0).abs() < 2.0, "{f2}");
    // Without ethane or nitrogen, pure methane's triple point.
    let pure = hydrocarbon_freezes(0.0, 0.0).0;
    assert!((pure - METHANE_TRIPLE_TEMPERATURE).abs() < TIGHT, "{pure}");
    // A sea under Titan's air, with its ethane, stays liquid well under
    // the binary rule's 77.6 K.
    let titan = methane_freezes(TITAN_N2).0;
    assert!(titan < 72.0, "{titan}");
}

#[test]
fn water_on_top_is_never_lava() {
    let air = PartialPressures {
        n2: 10.0,
        h2o: 200.0,
        ..PartialPressures::default()
    };
    for runaway in [false, true] {
        for t in [700.0, 2000.0, 5000.0] {
            assert_eq!(
                liquid(Kelvin(t), &air, oceans(1.0), runaway, &none()),
                None,
                "{t} {runaway}"
            );
        }
    }
    assert_eq!(
        liquid(Kelvin(2000.0), &air, dry(), true, &none()),
        Some(Liquid::Lava)
    );
}

/// The liquid half of legacy's `earth_mars_titan_venus` check, at the
/// observed surface temperatures.
#[test]
fn earth_mars_titan_have_their_liquids() {
    let earth = PartialPressures {
        n2: STANDARD_ATMOSPHERE_BAR - EARTH_AIR.o2 - EARTH_AIR.co2,
        ..EARTH_AIR
    };
    assert_eq!(
        liquid(Kelvin(EARTH_SURFACE_K), &earth, oceans(1.0), false, &none()),
        Some(Liquid::Water)
    );
    let [mars_k, mars_co2, mars_oceans] = MARS_LIKE;
    let mars = PartialPressures {
        co2: mars_co2,
        ..PartialPressures::default()
    };
    assert_eq!(
        liquid(Kelvin(mars_k), &mars, oceans(mars_oceans), false, &none()),
        None
    );
    // Titan's measured air holds 0.49 of pure methane's saturation, just
    // under the sea's 0.5: its seas on the ground make the liquid.
    assert_eq!(
        liquid(
            Kelvin(TITAN_SURFACE_K),
            &TITAN_SURFACE_AIR,
            dry(),
            false,
            &seas()
        ),
        Some(Liquid::Hydrocarbons)
    );
    let venus = PartialPressures {
        co2: VENUS_PRESSURE_BAR,
        ..PartialPressures::default()
    };
    assert_eq!(
        liquid(Kelvin(VENUS_SURFACE_K), &venus, dry(), true, &none()),
        None
    );
}

/// Each window holds its lower edge and leaves out its upper one.
#[test]
fn liquid_boundaries_are_inclusive_below() {
    assert_eq!(
        liquid(LAVA_TEMPERATURE, &none(), dry(), false, &none()),
        Some(Liquid::Lava)
    );
    let air = PartialPressures {
        n2: 1.0,
        ..PartialPressures::default()
    };
    let p = Pascal(PASCALS_PER_BAR);
    let melts = ice_ih_melting_temperature(p);
    let boils = boiling_temperature(p).expect("over the triple point");
    let water = oceans(1.0);
    assert_eq!(
        liquid(melts, &air, water, false, &none()),
        Some(Liquid::Water)
    );
    assert_eq!(
        liquid(Kelvin(melts.0 - 0.01), &air, water, false, &none()),
        None
    );
    assert_eq!(liquid(boils, &air, water, false, &none()), None);
    // Under water's triple-point pressure, no liquid water at all.
    let thin = PartialPressures {
        n2: 0.9 * bar(TRIPLE_POINT_PRESSURE),
        ..PartialPressures::default()
    };
    for t in (200..=400).map(f64::from) {
        assert_eq!(liquid(Kelvin(t), &thin, water, false, &none()), None, "{t}");
    }
}

#[test]
fn frozen_frost_melts_back() {
    let melts = NITROGEN_TRIPLE_TEMPERATURE;
    assert_eq!(
        frozen_share(Kelvin(melts.0 - PHASE_BAND.0), melts).get(),
        1.0
    );
    assert_eq!(frozen_share(melts, melts).get(), 0.0);
    let half = Fraction::new(0.5).expect("a share");
    let left = melting(Bar(5.0), half).0;
    assert!((left - 0.5 * FROST_LAYER.0).abs() < TIGHT, "{left}");
    // All frozen: all of it stays.
    let all = Fraction::new(1.0).expect("a share");
    assert_eq!(melting(Bar(5.0), all).0, 5.0);
}

/// Io's SO2 frost under an air at its sublimation pressure stays whole;
/// above the frost's triple point it all goes back to the air.
#[test]
fn io_keeps_its_so2_frost() {
    let t = IO_SURFACE_K;
    let air = PartialPressures {
        so2: bar(sulphur_dioxide_frost_pressure(t).expect("under the triple point")),
        ..PartialPressures::default()
    };
    let frost = frost(&none(), Bar(0.0), Bar(1.0), t, &air);
    assert_eq!(frost.so2, 1.0, "{frost:?}");
    let warm = frost_at_triple(&air);
    assert_eq!(warm.so2, 0.0, "{warm:?}");
    assert_eq!(warm.co2, 0.0, "{warm:?}");
}

fn frost_at_triple(air: &PartialPressures) -> PartialPressures {
    frost(
        &none(),
        Bar(1.0),
        Bar(1.0),
        SULPHUR_DIOXIDE_TRIPLE_TEMPERATURE,
        air,
    )
}
