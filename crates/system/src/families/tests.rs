use super::table::*;
use super::*;
use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use science::constants::AGE_OF_UNIVERSE;
use science::stellar::life;
use science::units::{Dex, Gyr, SolarMass};
use std::collections::BTreeMap;
use timeline::{Seed, Trace};

const ALL: [Family; 5] = [
    Family::Compact,
    Family::SolarLike,
    Family::HotJupiter,
    Family::EccentricGiants,
    Family::Sparse,
];

#[test]
fn families_are_even() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    let picks = 100_000;
    let mut counts = BTreeMap::new();
    for _ in 0..picks {
        *counts
            .entry(pick_family(&mut rng, |_| true))
            .or_insert(0u32) += 1;
    }
    for family in ALL {
        let share = f64::from(counts[&family]) / f64::from(picks);
        assert!((share - 0.2).abs() < 0.01, "{family}: {share}");
    }
}

#[test]
fn an_impossible_family_is_drawn_again() {
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    for _ in 0..1_000 {
        let family = pick_family(&mut rng, |f| {
            f != Family::HotJupiter && f != Family::Compact
        });
        assert!(!matches!(family, Family::HotJupiter | Family::Compact));
    }
}

#[test]
fn sparse_is_always_possible() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    for _ in 0..100 {
        assert_eq!(
            pick_family(&mut rng, |f| f == Family::Sparse),
            Family::Sparse
        );
    }
}

#[test]
fn every_family_is_possible_on_every_host() {
    for mass in [0.08, 0.3, 1.0, 2.0, 8.0] {
        let life = life(SolarMass(mass), Dex(0.0)).unwrap();
        let end = Gyr(life.end.age().0.min(AGE_OF_UNIVERSE.0));
        for age in [life.zams, end] {
            let mut trace = Trace::new();
            let star = crate::star::star(
                Some(SolarMass(mass)),
                Some(Dex(0.0)),
                Some(age),
                Seed::new(1),
                &mut trace,
            )
            .unwrap();
            for family in ALL {
                assert!(
                    possible(family, &star.state),
                    "{family} on {mass} at {age:?}"
                );
            }
        }
    }
}

#[test]
fn the_table_ranges_are_ordered() {
    for (low, high) in [
        COMPACT_PLANETS,
        SOLAR_LIKE_ROCKY,
        SOLAR_LIKE_GIANTS,
        ECCENTRIC_GIANTS,
        ECCENTRIC_INNER_SMALL,
        SPARSE_PLANETS,
    ] {
        assert!(low <= high);
    }
    for chance in [
        HOT_JUPITER_COMPANION_CHANCE,
        SOLAR_LIKE_STIRRED_CHANCE,
        SPARSE_GIANT_CHANCE,
    ] {
        assert!((0.0..=1.0).contains(&chance));
    }
    const { assert!(SLOT_TRIES > 0) };
}
