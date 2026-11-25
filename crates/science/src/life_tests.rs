use super::*;

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

/// Source: the page's ratio of the CIPM shares, 0.2682
/// (`docs/science/planet-atmospheres.md`, "The oxygenated air").
const PAGE_O2_PER_N2: f64 = 0.2682;

/// Definition: the page quotes the ratio to four places, 1e-4.
const PAGE_PLACES: f64 = 1e-4;

/// Definition: an air for the oxygen steps, N₂ 0.78, CH₄ 1e-3, no O₂,
/// CO₂ 4e-4 bar (an early Earth's shape, for the law only).
const EARLY_AIR: PartialPressures = PartialPressures {
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
fn o2_per_n2_is_the_cipm_ratio() {
    assert!(
        (O2_PER_N2 - PAGE_O2_PER_N2).abs() < PAGE_PLACES,
        "{O2_PER_N2}"
    );
}

#[test]
fn oxygen_comes_in_two_steps() {
    assert_eq!(oxygenate(&EARLY_AIR, Stage::Lifeless), EARLY_AIR);
    assert_eq!(oxygenate(&EARLY_AIR, Stage::Anoxic), EARLY_AIR);
    let oxidising = oxygenate(&EARLY_AIR, Stage::Oxidising);
    assert_eq!(
        oxidising,
        PartialPressures {
            ch4: 0.0,
            ..EARLY_AIR
        }
    );
    let modern = oxygenate(&EARLY_AIR, Stage::Modern);
    assert_eq!(
        modern,
        PartialPressures {
            ch4: 0.0,
            o2: O2_PER_N2 * EARLY_AIR.n2,
            ..EARLY_AIR
        }
    );
    assert!(LIVING_EARTH_O2.contains(&modern.o2), "{}", modern.o2);
}
