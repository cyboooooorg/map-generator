use super::*;
use crate::constants::SOLAR_EFFECTIVE_TEMPERATURE;
use crate::life_gases::modern_methane;
use crate::life_gases::tests::{STUDY_37_BAR, STUDY_37_WITHIN, earth_gravity};
use crate::units::{EarthFlux, Kelvin};

/// Source: the Sun as seen from Earth, its effective temperature
/// ([`SOLAR_EFFECTIVE_TEMPERATURE`]) and one Earth flux (the unit's
/// definition), the host of study 37's derivation.
const SUN: [(Kelvin, EarthFlux); 1] = [(SOLAR_EFFECTIVE_TEMPERATURE, EarthFlux(1.0))];

/// Definition: a K2V host at one Earth flux, 5000 K (Rugheimer &
/// Kaltenegger 2018, Table 1, a host of `life_gases`'s grid), whose
/// methane is not the Sun's.
const K2V: [(Kelvin, EarthFlux); 1] = [(Kelvin(5000.0), EarthFlux(1.0))];

/// Definition: a draw under any chance, the lowest a draw can be.
const LUCKY: f64 = 0.0;

/// Definition: a draw over [`LIFE_CHANCE`], near the top of `[0, 1)`.
const UNLUCKY: f64 = 0.99;

/// Definition: an age just under [`LIFE_MIN_GYR`] (1.17 Gyr), 1.16 Gyr.
const JUST_TOO_YOUNG: Gyr = Gyr(1.16);

/// Source: the ages of the page's table of a living Earth
/// (`docs/science/planet-atmospheres.md`, "Life"): 0.5, 1.5, 3.0 and
/// 4.6 Gyr.
const EARTH_AGES: [Gyr; 4] = [Gyr(0.5), Gyr(1.5), Gyr(3.0), Gyr(4.6)];

/// Source: the page's age of today's Earth, 4.6 Gyr ("Life", its table).
const TODAY: Gyr = Gyr(4.6);

/// Definition: the ratio by weight of O₂ to N₂ in the CIPM-2007 air,
/// from its mole fractions and the molar masses (the page's partial
/// pressures are weights: a gas's mass share times the pressure).
fn cipm_mass_ratio() -> f64 {
    (CIPM_O2_SHARE * O2_MOLAR_MASS) / (CIPM_N2_SHARE * N2_MOLAR_MASS)
}

/// Definition: the two sides are the same product, equal to rounding, 1e-12.
const RATIO_PLACES: f64 = 1e-12;

/// Definition: an air for the oxygen steps, N₂ 0.78, CH₄ 1e-3, no O₂,
/// CO₂ 4e-4 bar (an early Earth's shape, for the law only).
const EARLY_AIR: Weights = Weights {
    n2: 0.78,
    co2: 4e-4,
    h2o: 0.0,
    ch4: 1e-3,
    nh3: 0.0,
    so2: 0.0,
    o2: 0.0,
    ar: 0.0,
};

/// Source: legacy `earth_has_oxygenic_life`
/// (`legacy-final:crates/mapgen/src/planet/life.rs`): a living
/// Earth's O₂ between 0.15 and 0.30 bar.
const LIVING_EARTH_O2: std::ops::RangeInclusive<f64> = 0.15..=0.30;

#[test]
fn life_waits_for_the_oldest_accepted_evidence() {
    assert_eq!(stage(true, JUST_TOO_YOUNG, LUCKY), Stage::Lifeless);
    assert_eq!(stage(true, LIFE_MIN_GYR, LUCKY), Stage::Anoxic);
}

#[test]
fn the_ages_of_a_living_earth() {
    let stages = EARTH_AGES.map(|age| stage(true, age, LUCKY));
    assert_eq!(
        stages,
        [
            Stage::Lifeless,
            Stage::Anoxic,
            Stage::Oxidising,
            Stage::Modern
        ]
    );
    assert_eq!(stage(true, OXYGEN_AFTER_GYR, LUCKY), Stage::Oxidising);
    assert_eq!(stage(true, MODERN_OXYGEN_AFTER_GYR, LUCKY), Stage::Modern);
}

#[test]
fn a_habitable_world_can_be_lifeless() {
    assert_eq!(stage(true, TODAY, UNLUCKY), Stage::Lifeless);
    assert_eq!(stage(true, TODAY, LIFE_CHANCE), Stage::Lifeless);
}

#[test]
fn no_water_no_life() {
    for draw in [LUCKY, 0.5, UNLUCKY] {
        for age in [EARTH_AGES[0], TODAY] {
            assert_eq!(stage(false, age, draw), Stage::Lifeless, "{draw} {age:?}");
        }
    }
}

#[test]
fn o2_per_n2_is_the_cipm_ratio_by_weight() {
    assert!(
        (O2_PER_N2 - cipm_mass_ratio()).abs() < RATIO_PLACES,
        "{O2_PER_N2}"
    );
    // About 0.306, not the 0.2682 of the mole fractions.
    assert!((0.30..0.31).contains(&O2_PER_N2), "{O2_PER_N2}");
}

#[test]
fn oxygen_comes_in_two_steps() {
    let g = earth_gravity();
    assert_eq!(oxygenate(&EARLY_AIR, Stage::Lifeless, &SUN, g), EARLY_AIR);
    assert_eq!(oxygenate(&EARLY_AIR, Stage::Anoxic, &SUN, g), EARLY_AIR);
    let oxidising = oxygenate(&EARLY_AIR, Stage::Oxidising, &SUN, g);
    assert_eq!(
        oxidising,
        Weights {
            ch4: 0.0,
            ..EARLY_AIR
        }
    );
    let modern = oxygenate(&EARLY_AIR, Stage::Modern, &SUN, g);
    assert_eq!(
        modern,
        Weights {
            ch4: modern_methane(&SUN, g).0,
            o2: O2_PER_N2 * EARLY_AIR.n2,
            ..EARLY_AIR
        }
    );
    assert!(LIVING_EARTH_O2.contains(&modern.o2), "{}", modern.o2);
}

#[test]
fn a_modern_air_keeps_the_methane_of_its_stars() {
    let g = earth_gravity();
    let around_the_sun = oxygenate(&EARLY_AIR, Stage::Modern, &SUN, g).ch4;
    assert!(
        (around_the_sun / STUDY_37_BAR - 1.0).abs() < STUDY_37_WITHIN,
        "{around_the_sun}"
    );
    let around_a_k2v = oxygenate(&EARLY_AIR, Stage::Modern, &K2V, g).ch4;
    assert_eq!(around_a_k2v, modern_methane(&K2V, g).0);
    assert!(around_a_k2v > around_the_sun, "{around_a_k2v}");
    // An oxidising air keeps none, whatever its host (#717).
    for stars in [&SUN, &K2V] {
        assert_eq!(oxygenate(&EARLY_AIR, Stage::Oxidising, stars, g).ch4, 0.0);
    }
}
