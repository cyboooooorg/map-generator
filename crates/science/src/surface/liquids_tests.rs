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
    SULPHUR_DIOXIDE_TRIPLE_TEMPERATURE, nitrogen_frost_pressure, nitrogen_saturation_pressure,
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
}

/// Titan's air holds the vapour of its seas: they are liquid at the
/// surface temperature Huygens measured, 93.65 K (Fulchignoni et al.
/// 2005), above the freezing point of the sea under its 1.45 bar of N2.
#[test]
fn titans_seas_are_liquid_at_its_surface_temperature() {
    let titan = PartialPressures {
        ch4: methane_sea_vapour(Kelvin(TITAN_SURFACE_K)).0,
        ..TITAN_SURFACE_AIR
    };
    assert!(methane_freezes(TITAN_N2).0 < TITAN_SURFACE_K);
    assert_eq!(
        liquid(Kelvin(TITAN_SURFACE_K), &titan, dry(), false, &none()),
        Some(Liquid::Hydrocarbons)
    );
}

/// Titan's methane share over a sea, against 5.65 % (Niemann et al.
/// 2010).
#[test]
fn the_vapour_of_a_sea_gives_titans_methane_share() {
    let vapour = methane_sea_vapour(TITAN_SEA_K).0;
    let share = vapour / (TITAN_N2.0 + vapour);
    println!("Titan's CH4 share over a sea at 94 K: {share:.4}");
    assert!((0.03..0.06).contains(&share), "{share}");
}

/// The more nitrogen the air dissolves in the sea, the lower its freezing
/// point (issue #524).
#[test]
fn nitrogen_lowers_the_freezing_of_methane() {
    let freezes = |n2: f64| methane_freezes(Bar(n2)).0;
    println!(
        "methane_freezes: 0 bar {:.3} K, 1.45 bar {:.3} K, 100 bar {:.3} K",
        freezes(0.0),
        freezes(TITAN_N2.0),
        freezes(100.0)
    );
    let mut last = freezes(0.0);
    for n2 in [0.5, TITAN_N2.0, 3.0] {
        assert!(freezes(n2) < last, "{n2} bar: {} >= {last}", freezes(n2));
        last = freezes(n2);
    }
}

/// Once the sea holds `MAX_NITROGEN_IN_LIQUID` of nitrogen no more
/// dissolves: its freezing point stops falling.
#[test]
fn dissolved_nitrogen_has_a_floor() {
    let floor = methane_freezes(Bar(100.0)).0;
    let cap = hydrocarbon_freezes(
        MAX_NITROGEN_IN_LIQUID,
        1.0 - SEA_METHANE_SHARE - MAX_NITROGEN_IN_LIQUID,
    )
    .0;
    assert!((floor - cap).abs() < TIGHT, "{floor} {cap}");
    assert!((methane_freezes(Bar(1000.0)).0 - floor).abs() < TIGHT);
}

/// The ethane of the sea dilutes its methane: it freezes under the pure
/// methane triple point, with or without nitrogen.
#[test]
fn a_methane_sea_freezes_below_pure_methane() {
    for n2 in [0.0, TITAN_N2.0, 100.0] {
        let freezes = methane_freezes(Bar(n2)).0;
        assert!(freezes < METHANE_TRIPLE_TEMPERATURE, "{n2} bar: {freezes}");
    }
}

/// A sea is liquid over its freezing point and not under it, with or
/// without the nitrogen of the air.
#[test]
fn a_methane_sea_is_liquid_over_its_freezing_point() {
    let sea_at = |n2: f64, t: f64| {
        let air = PartialPressures {
            n2,
            ch4: methane_sea_vapour(Kelvin(t)).0,
            ..PartialPressures::default()
        };
        liquid(Kelvin(t), &air, dry(), false, &seas())
    };
    for n2 in [0.0, 1.0] {
        let freezes = methane_freezes(Bar(n2)).0;
        assert_eq!(
            sea_at(n2, freezes + 0.5),
            Some(Liquid::Hydrocarbons),
            "{n2} bar"
        );
        assert_eq!(sea_at(n2, freezes - 0.5), None, "{n2} bar");
    }
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
        n2: bar(nitrogen_saturation_pressure(warm)),
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
        n2: bar(nitrogen_saturation_pressure(t)),
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
/// above the frost's triple point it all goes back to the air. The 1 bar
/// of CO2 frost of `frost_at_triple` goes back too, for another reason:
/// at 197.64 K CO2's sublimation pressure exceeds that 1 bar, so the
/// air takes all of it.
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

/// Definition: 3000 bar (300 MPa), where ice III is the stable ice.
const ICE_III_AIR_BAR: f64 = 3000.0;
/// Definition: 254 K, under the ice III melting temperature at 300 MPa
/// (254.96 K, IAPWS R14-08(2011), eq. 2).
const BELOW_ICE_III_MELTING_K: Kelvin = Kelvin(254.0);
/// Definition: 256 K, over the same melting temperature.
const ABOVE_ICE_III_MELTING_K: Kelvin = Kelvin(256.0);

/// Issue #671: under 300 MPa the stable ice is III, which melts at
/// 254.96 K, so water at 254 K is frozen (the floor of ice Ih, 251.165 K,
/// called it liquid).
#[test]
fn water_under_ice_iii_pressure_freezes() {
    let air = PartialPressures {
        n2: ICE_III_AIR_BAR,
        ..PartialPressures::default()
    };
    let water = oceans(1.0);
    assert_ne!(
        liquid(BELOW_ICE_III_MELTING_K, &air, water, false, &none()),
        Some(Liquid::Water)
    );
    assert_eq!(
        liquid(ABOVE_ICE_III_MELTING_K, &air, water, false, &none()),
        Some(Liquid::Water)
    );
}
