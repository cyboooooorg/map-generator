use super::laws::*;
use super::table::*;
use super::*;
use crate::{ArchetypeId, Trial, TrialOutcome};
use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use science::constants::AGE_OF_UNIVERSE;
use science::stellar::life;
use science::units::{
    Degree, Dex, EarthFlux, EarthMass, EarthRadius, Fraction, Gyr, JoulePerSquareMetre, Kelvin,
    KilogramPerCubicMetre, SolarLuminosity, SolarMass, WattPerSquareMetre,
};
use std::collections::BTreeMap;
use timeline::{
    BandLight, Code, NodeValueOf, Orbit, PlanetClass, Provenance, Seed, StellarSource,
    StellarSources, Trace, Traced,
};

pub(crate) const ALL: [Family; 5] = [
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

/// A context for `family` around a 1 M☉, 1 R☉ host with `giants` giants
/// planned, the compact σ_e at 0.03 and the system mass at 3 M⊕.
pub(crate) fn context(family: Family, giants: u32, stirred: bool) -> FamilyContext {
    FamilyContext {
        family,
        host: SolarMass(1.0),
        ice_line: orbit::ice_line(SolarMass(1.0)),
        stellar_radius: AstronomicalUnit(SOLAR_RADIUS.0 / ASTRONOMICAL_UNIT.0),
        counts: Counts { small: 3, giants },
        system_mass: EarthMass(3.0),
        sigma_e: 0.03,
        stirred,
        companion: family == Family::HotJupiter && giants == 2,
    }
}

fn leaf<T: NodeValueOf>(trace: &mut Trace, code: &'static str, value: T) -> Traced<T> {
    trace
        .leaf(Code::of(code), value, Provenance::Input)
        .unwrap()
}

/// A placed planet of `mass` M⊕ at `au` around a 1 M☉ host; only the
/// mass, the orbit and `asked` matter to the proposers.
pub(crate) fn placed(trace: &mut Trace, mass: f64, au: f64, e: f64, asked: bool) -> Placed {
    let band = |trace: &mut Trace| BandLight {
        flux: leaf(trace, "test.band-flux", WattPerSquareMetre(1.0)),
        fluence: leaf(trace, "test.band-fluence", JoulePerSquareMetre(1.0)),
    };
    let source = StellarSource {
        flux: leaf(trace, "test.flux", EarthFlux(1.0 / (au * au))),
        equilibrium: leaf(trace, "test.equilibrium", Kelvin(255.0)),
        effective_temperature: leaf(trace, "test.teff", Kelvin(5772.0)),
        mass: leaf(trace, "test.star-mass", SolarMass(1.0)),
        luminosity: leaf(trace, "test.luminosity", SolarLuminosity(1.0)),
        xuv: band(trace),
        lyman_alpha: band(trace),
    };
    let trial = Trial {
        seed: Seed::new(1),
        mass: leaf(trace, "test.mass", EarthMass(mass)),
        orbit: Orbit {
            semi_major_axis: leaf(trace, "test.a", AstronomicalUnit(au)),
            eccentricity: leaf(trace, "test.e", Fraction::new(e).unwrap()),
            period: leaf(
                trace,
                "test.period",
                orbit::period(AstronomicalUnit(au), SolarMass(1.0)),
            ),
            perihelion_longitude: leaf(trace, "test.varpi", Degree(0.0)),
        },
        sources: StellarSources::new(vec![source]).unwrap(),
        age: leaf(trace, "test.age", Gyr(4.6)),
        metallicity: leaf(trace, "test.metallicity", Dex(0.0)),
        ice_line: leaf(trace, "test.ice-line", AstronomicalUnit(2.7)),
        archetype: None,
    };
    Placed {
        trial,
        outcome: TrialOutcome {
            class: if mass < 30.0 {
                PlanetClass::SuperEarth
            } else {
                PlanetClass::GasGiant
            },
            radius: EarthRadius(1.0),
            mean_density: KilogramPerCubicMetre(5500.0),
        },
        label: "test".to_owned(),
        asked: asked.then(|| ArchetypeId(Code::of("earth-like"))),
    }
}

#[test]
fn counts_cover_their_table_ranges() {
    let mut rng = ChaCha8Rng::seed_from_u64(50);
    for family in ALL {
        let mut seen = BTreeMap::new();
        for _ in 0..5_000 {
            let counts = super::counts(family, &mut rng);
            *seen.entry((counts.small, counts.giants)).or_insert(0u32) += 1;
        }
        let smalls: Vec<u32> = seen.keys().map(|k| k.0).collect();
        let giants: Vec<u32> = seen.keys().map(|k| k.1).collect();
        let span = |values: &[u32]| (*values.iter().min().unwrap(), *values.iter().max().unwrap());
        let every =
            |values: &[u32], (low, high): (u32, u32)| (low..=high).all(|n| values.contains(&n));
        match family {
            Family::Compact => {
                assert_eq!(span(&smalls), COMPACT_PLANETS);
                assert!(every(&smalls, COMPACT_PLANETS));
                assert_eq!(span(&giants), (0, 0));
            }
            Family::SolarLike => {
                assert_eq!(span(&smalls), SOLAR_LIKE_ROCKY);
                assert!(every(&smalls, SOLAR_LIKE_ROCKY));
                assert_eq!(span(&giants), SOLAR_LIKE_GIANTS);
                assert!(every(&giants, SOLAR_LIKE_GIANTS));
            }
            Family::HotJupiter => {
                assert_eq!(span(&smalls), (0, 0));
                assert_eq!(span(&giants), (1, 2));
                let with = seen.get(&(0, 2)).copied().unwrap_or(0);
                let share = f64::from(with) / 5_000.0;
                // Standard error of a 0.70 share over 5 000 draws: 0.0065.
                assert!((share - HOT_JUPITER_COMPANION_CHANCE).abs() < 0.026);
            }
            Family::EccentricGiants => {
                assert_eq!(span(&smalls), ECCENTRIC_INNER_SMALL);
                assert!(every(&smalls, ECCENTRIC_INNER_SMALL));
                assert_eq!(span(&giants), ECCENTRIC_GIANTS);
                assert!(every(&giants, ECCENTRIC_GIANTS));
            }
            Family::Sparse => {
                let totals: Vec<u32> = seen.keys().map(|k| k.0 + k.1).collect();
                assert_eq!(span(&totals), SPARSE_PLANETS);
                assert!(every(&totals, SPARSE_PLANETS));
                assert!(seen.contains_key(&(1, 1)) && seen.contains_key(&(0, 2)));
            }
        }
    }
}

#[test]
fn the_context_draws_its_system_values_in_their_ranges() {
    let mut trace = Trace::new();
    let star = crate::star::star(
        Some(SolarMass(1.0)),
        Some(Dex(0.0)),
        Some(Gyr(4.6)),
        Seed::new(1),
        &mut trace,
    )
    .unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(51);
    let mut stirred = 0;
    for _ in 0..2_000 {
        let ctx = FamilyContext::draw(Family::SolarLike, &star.state, &mut rng);
        assert!((COMPACT_SIGMA_E.0..COMPACT_SIGMA_E.1).contains(&ctx.sigma_e));
        assert!(ctx.system_mass.0 >= SMALL_MASS_MIN.0 && ctx.system_mass.0 < SMALL_MASS_MAX.0);
        assert!((ctx.ice_line.0 - 2.7).abs() < 0.01);
        assert!(!ctx.companion);
        stirred += u32::from(ctx.stirred);
    }
    // Standard error of a 0.1 share over 2 000 draws: 0.0067.
    assert!((f64::from(stirred) / 2_000.0 - SOLAR_LIKE_STIRRED_CHANCE).abs() < 0.027);
    let ctx = FamilyContext::draw(Family::Compact, &star.state, &mut rng);
    assert!(!ctx.stirred);
}

#[test]
fn hot_jupiter_periods_stay_in_their_band() {
    let mut rng = ChaCha8Rng::seed_from_u64(52);
    for host in [0.08, 0.3, 1.0, 2.0, 8.0] {
        let mut ctx = context(Family::HotJupiter, 2, false);
        ctx.host = SolarMass(host);
        ctx.ice_line = orbit::ice_line(SolarMass(host));
        let surface = orbit::period(ctx.stellar_radius, ctx.host);
        for _ in 0..2_000 {
            let hot = propose(&ctx, 0, &mut rng, &[]);
            let period = orbit::period(hot.a, ctx.host);
            assert_eq!(hot.kind, SlotKind::Giant);
            assert!(
                period.0 >= surface.0 * (1.0 - 1e-9) && period.0 < 10.0 * (1.0 + 1e-9),
                "{host}: {period:?}"
            );
            let jupiters = hot.mass.0 / EARTH_MASSES_PER_JUPITER_MASS;
            assert!((HOT_JUPITER_MASS.0..HOT_JUPITER_MASS.1).contains(&jupiters));
            let companion = propose(&ctx, 1, &mut rng, &[]);
            let scale = ctx.ice_line.0 / 2.7;
            assert!(
                companion.a.0 >= COMPANION_AXIS.0.0 * scale
                    && companion.a.0 < COMPANION_AXIS.1.0 * scale
            );
        }
    }
}

#[test]
fn compact_planets_follow_the_outermost_drawn_one() {
    let mut rng = ChaCha8Rng::seed_from_u64(53);
    let ctx = context(Family::Compact, 0, false);
    let mut trace = Trace::new();
    let first = propose(&ctx, 0, &mut rng, &[]);
    assert_eq!(first.mass, ctx.system_mass);
    assert!(orbit::period(first.a, ctx.host).0 < COMPACT_MAX_PERIOD.0);
    // An asked planet further out is not the chain's anchor.
    let placed = [
        placed(&mut trace, 3.0, 0.1, 0.0, false),
        placed(&mut trace, 3.0, 0.15, 0.0, false),
        placed(&mut trace, 300.0, 1.0, 0.0, true),
    ];
    for _ in 0..1_000 {
        let next = propose(&ctx, 2, &mut rng, &placed);
        let ratio = (next.a.0 / 0.15).powf(1.5);
        assert!(ratio >= WEISS_WALL * (1.0 - 1e-9), "{ratio}");
    }
}

#[test]
fn solar_like_slots_are_rocky_then_giants() {
    let mut rng = ChaCha8Rng::seed_from_u64(54);
    let ctx = context(Family::SolarLike, 2, false);
    let mut trace = Trace::new();
    let rocky = [
        placed(&mut trace, 1.0, 0.4, 0.0, false),
        placed(&mut trace, 1.0, 0.8, 0.0, false),
    ];
    for _ in 0..1_000 {
        let first = propose(&ctx, 0, &mut rng, &[]);
        assert_eq!(first.kind, SlotKind::Small);
        assert!(first.a.0 < ctx.ice_line.0);
        assert!((ROCKY_MASS.0.0..ROCKY_MASS.1.0).contains(&first.mass.0));
        let next = propose(&ctx, 2, &mut rng, &rocky);
        assert_eq!(next.kind, SlotKind::Small);
        assert!(next.a.0 > 0.8 && next.a.0 < ctx.ice_line.0, "{:?}", next.a);
        let giant = propose(&ctx, 3, &mut rng, &rocky);
        assert_eq!(giant.kind, SlotKind::Giant);
        assert!(giant.a.0 >= ctx.ice_line.0);
        assert!((SOLAR_LIKE_GIANT_MASS.0.0..SOLAR_LIKE_GIANT_MASS.1.0).contains(&giant.mass.0));
    }
    let giant = [placed(&mut trace, 300.0, 5.0, 0.05, false)];
    for _ in 0..1_000 {
        let next = propose(&ctx, 4, &mut rng, &giant);
        let ratio = next.a.0 / 5.0;
        assert!(ratio >= 0.5f64.exp() && ratio < 10.0, "{ratio}");
    }
}

#[test]
fn eccentric_inner_planets_stay_inside_the_giant_periapsis() {
    let mut rng = ChaCha8Rng::seed_from_u64(55);
    let ctx = context(Family::EccentricGiants, 1, false);
    let mut trace = Trace::new();
    let first = propose(&ctx, 0, &mut rng, &[]);
    assert_eq!(first.kind, SlotKind::Giant);
    assert!(first.a.0 >= ctx.ice_line.0);
    let giant = [placed(&mut trace, 300.0, 1.0, 0.6, false)];
    for _ in 0..2_000 {
        let inner = propose(&ctx, 1, &mut rng, &giant);
        assert_eq!(inner.kind, SlotKind::Small);
        assert!(inner.a.0 < 0.4 * (1.0 + 1e-9), "{:?}", inner.a);
        assert!(inner.mass.0 >= SMALL_MASS_MIN.0 && inner.mass.0 < SMALL_MASS_MAX.0);
    }
}

#[test]
fn sparse_planets_stay_between_the_star_and_the_survey_edge() {
    let mut rng = ChaCha8Rng::seed_from_u64(56);
    let mut ctx = context(Family::Sparse, 1, false);
    ctx.counts = Counts {
        small: 1,
        giants: 1,
    };
    for _ in 0..2_000 {
        let small = propose(&ctx, 0, &mut rng, &[]);
        let giant = propose(&ctx, 1, &mut rng, &[]);
        assert_eq!((small.kind, giant.kind), (SlotKind::Small, SlotKind::Giant));
        for proposal in [small, giant] {
            assert!(proposal.a.0 >= ctx.stellar_radius.0 && proposal.a.0 < 30.0);
        }
        let jupiters = giant.mass.0 / EARTH_MASSES_PER_JUPITER_MASS;
        assert!((ECCENTRIC_GIANT_MASS.0..ECCENTRIC_GIANT_MASS.1).contains(&jupiters));
    }
}
