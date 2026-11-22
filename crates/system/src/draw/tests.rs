use std::cell::Cell;
use std::collections::BTreeMap;

use super::place::{MAX_PLANETS, Rules, System, place, probe_mass, recheck};
use super::*;
use crate::ArchetypeId;
use crate::families::table::SLOT_TRIES;
use crate::families::tests::placed as test_planet;
use crate::families::{Counts, FamilyContext, Placed};
use crate::inputs::{BodyRequest, INPUTS_SCHEMA, SystemRequest};
use crate::sources::{Light, light};
use crate::spin;
use crate::star::Star;
use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;
use science::orbit::{self, Pair, TWO_PLANET_FLOOR};
use science::units::{Dex, EarthMass, EarthRadius, KilogramPerCubicMetre, SolarMass};
use timeline::{Code, ContractError, Origin, PlanetClass, Provenance, Trace};

const FAMILIES: [Family; 5] = [
    Family::Compact,
    Family::SolarLike,
    Family::HotJupiter,
    Family::EccentricGiants,
    Family::Sparse,
];

/// Source: the mean density of the Earth, 5 514 kg/m³ (NASA Earth fact
/// sheet), to turn a test density into Earth radii.
const EARTH_DENSITY: f64 = 5514.0;

/// The test classifier: below 2 M⊕ rocky, below 10 a super-Earth, below
/// 30 a mini-Neptune, else a gas giant (a hot Jupiter inside 0.1 au).
/// Small planets: radius M^0.27, density 5 500 kg/m³; giants: radius
/// from a density of 1 300 kg/m³.
fn model(trial: &Trial, _: &Trace) -> Result<TrialOutcome, SystemError> {
    let mass = trial.mass.value().0;
    let a = trial.orbit.semi_major_axis.value().0;
    let class = match mass {
        m if m < 2.0 => PlanetClass::Rocky,
        m if m < 10.0 => PlanetClass::SuperEarth,
        m if m < 30.0 => PlanetClass::MiniNeptune,
        _ if a < 0.1 => PlanetClass::HotJupiter,
        _ => PlanetClass::GasGiant,
    };
    let (radius, density) = if mass < 30.0 {
        (mass.powf(0.27), 5500.0)
    } else {
        ((mass * EARTH_DENSITY / 1300.0).cbrt(), 1300.0)
    };
    Ok(TrialOutcome {
        class,
        radius: EarthRadius(radius),
        mean_density: KilogramPerCubicMetre(density),
    })
}

fn system_inputs(seed: u64) -> Inputs {
    Inputs {
        schema: INPUTS_SCHEMA,
        seed: Seed::new(seed),
        present: None,
        request: Request::System(SystemRequest {
            host_mass: None,
            metallicity: None,
            archetypes: BTreeMap::new(),
        }),
    }
}

#[test]
fn a_drawn_system_carries_its_star() {
    let inputs = Inputs {
        present: Some(Gyr(4.57)),
        request: Request::System(SystemRequest {
            host_mass: Some(SolarMass(1.0)),
            metallicity: Some(Dex(0.0)),
            archetypes: BTreeMap::new(),
        }),
        ..system_inputs(7)
    };
    let drawn = draw_with(&inputs, &mut Trace::new(), model).unwrap();
    let expected = star::star(
        Some(SolarMass(1.0)),
        Some(Dex(0.0)),
        Some(Gyr(4.57)),
        Seed::new(7),
        &mut Trace::new(),
    )
    .unwrap();
    assert_eq!(drawn.star.mass.value(), expected.state.mass.value());
    assert_eq!(drawn.star.radius.value(), expected.state.radius.value());
    assert_eq!(
        drawn.star.luminosity.value(),
        expected.state.luminosity.value()
    );
    assert_eq!(drawn.age.value(), expected.age.value());
    assert_eq!(drawn.seed, Seed::new(7));
}

#[test]
fn the_system_number_is_in_range() {
    for seed in 0..1000 {
        let drawn = draw_with(&system_inputs(seed), &mut Trace::new(), model).unwrap();
        let number = drawn.designation.system.get();
        assert!((10_000..=99_999).contains(&number), "{number}");
    }
}

#[test]
fn a_body_request_is_refused() {
    let inputs = Inputs {
        request: Request::Body(BodyRequest {
            host_mass: None,
            metallicity: None,
            archetype: None,
        }),
        ..system_inputs(1)
    };
    assert_eq!(
        draw_with(&inputs, &mut Trace::new(), model).unwrap_err(),
        SystemError::BodyRequestNotDrawn
    );
}

fn id(text: &str) -> ArchetypeId {
    ArchetypeId(Code::new(text.to_owned()).unwrap())
}

fn asked(items: &[(&str, u32)]) -> BTreeMap<ArchetypeId, u32> {
    items
        .iter()
        .map(|&(name, count)| (id(name), count))
        .collect()
}

fn asking(seed: u64, items: &[(&str, u32)]) -> Inputs {
    Inputs {
        request: Request::System(SystemRequest {
            host_mass: None,
            metallicity: None,
            archetypes: asked(items),
        }),
        ..system_inputs(seed)
    }
}

/// A star of `host` M☉, its light, the context of a forced `family`,
/// drawn on the streams `draw_with` uses, and the trace holding the star.
fn parts(seed: u64, host: f64, family: Family) -> (Star, Light, FamilyContext, Trace) {
    let seed = Seed::new(seed);
    let mut trace = Trace::new();
    let star = star::star(Some(SolarMass(host)), None, None, seed, &mut trace).unwrap();
    let light = light(&star).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(seed.derive("family").get());
    let ctx = FamilyContext::draw(family, &star.state, &mut rng);
    (star, light, ctx, trace)
}

/// Places `asked` and the family's slots on a copy of the star's trace.
fn run(
    seed: u64,
    (star, light, ctx, trace): &(Star, Light, FamilyContext, Trace),
    asked: &BTreeMap<ArchetypeId, u32>,
    classify: &impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
) -> Result<Vec<Placed>, SystemError> {
    let system = System {
        star,
        light,
        ctx,
        seed: Seed::new(seed),
    };
    place(&system, asked, &mut trace.clone(), classify)
}

fn axis(planet: &Placed) -> f64 {
    planet.trial.orbit.semi_major_axis.value().0
}

fn pair(planet: &Placed) -> Pair {
    Pair {
        mass: *planet.trial.mass.value(),
        a: *planet.trial.orbit.semi_major_axis.value(),
        e: *planet.trial.orbit.eccentricity.value(),
    }
}

/// No periapsis inside the star or its Roche floor (twice it for a hot
/// Jupiter), every e below 1, every pair at 2√3 or more, sorted by axis.
fn assert_valid(placed: &[Placed], ctx: &FamilyContext, what: &str) {
    for planet in placed {
        let e = planet.trial.orbit.eccentricity.value().get();
        assert!(e < 1.0, "{what}: e {e}");
        let periapsis = axis(planet) * (1.0 - e);
        let roche =
            orbit::semi_major_axis(orbit::roche_period(planet.outcome.mean_density), ctx.host).0;
        let factor = if planet.outcome.class == PlanetClass::HotJupiter {
            2.0
        } else {
            1.0
        };
        assert!(periapsis >= ctx.stellar_radius.0, "{what}: inside the star");
        assert!(
            periapsis >= factor * roche,
            "{what}: inside the Roche floor"
        );
    }
    for (i, inner) in placed.iter().enumerate() {
        for outer in &placed[i + 1..] {
            let delta = orbit::mutual_hill_spacing(pair(inner), pair(outer), ctx.host);
            assert!(delta >= TWO_PLANET_FLOOR, "{what}: Δ {delta}");
        }
    }
    assert!(
        placed.windows(2).all(|w| axis(&w[0]) <= axis(&w[1])),
        "{what}: not sorted"
    );
}

/// Draws every family on every host for `seeds` seeds and checks each
/// system; returns the axis ratios of adjacent cold giants (gas or ice
/// giants, in order of distance) and the largest axis scaled to the Sun's
/// ice line (`a × 2.7 au / ice line`), of every planet and of the giants.
fn every_family_on_every_host(seeds: u64) -> (Vec<f64>, f64, f64) {
    let (mut ratios, mut widest, mut widest_giant) = (Vec::new(), 0.0_f64, 0.0_f64);
    for host in [0.08, 0.3, 0.8, 1.0, 2.0, 8.0] {
        for family in FAMILIES {
            for seed in 0..seeds {
                let parts = parts(seed, host, family);
                let what = format!("{host} M☉, {family}, seed {seed}");
                let placed = run(seed, &parts, &BTreeMap::new(), &model)
                    .unwrap_or_else(|error| panic!("{what}: {error}"));
                assert_valid(&placed, &parts.2, &what);
                let scale = 2.7 / parts.2.ice_line.0;
                widest = placed
                    .iter()
                    .map(|p| axis(p) * scale)
                    .fold(widest, f64::max);
                // No drawn giant past Fulton's support, 30 au scaled.
                assert!(
                    placed
                        .iter()
                        .filter(|p| p.outcome.class == PlanetClass::GasGiant)
                        .all(|p| axis(p) * scale <= 30.0 * (1.0 + 1e-12)),
                    "{what}: past the outer support"
                );
                let giants: Vec<f64> = placed
                    .iter()
                    .filter(|p| {
                        matches!(
                            p.outcome.class,
                            PlanetClass::GasGiant | PlanetClass::IceGiant
                        )
                    })
                    .map(axis)
                    .collect();
                ratios.extend(giants.windows(2).map(|w| w[1] / w[0]));
                widest_giant = giants
                    .iter()
                    .map(|a| a * scale)
                    .fold(widest_giant, f64::max);
            }
        }
    }
    (ratios, widest, widest_giant)
}

#[test]
fn every_family_draws_on_every_host() {
    every_family_on_every_host(20);
}

/// Prints the median axis ratio of adjacent cold giants (Rosenthal et al.
/// 2023 peak near 3) and the largest scaled axis, for the record.
#[test]
#[ignore = "heavy: full grid"]
fn every_family_draws_on_every_host_full_grid() {
    let (mut ratios, widest, widest_giant) = every_family_on_every_host(200);
    ratios.sort_by(f64::total_cmp);
    let median = ratios.get(ratios.len() / 2).copied().unwrap_or(f64::NAN);
    eprintln!(
        "adjacent giant pairs {}, median a2/a1 {median:.2}, largest a × 2.7 au / ice line {widest:.1} au (giants {widest_giant:.1} au)",
        ratios.len()
    );
}

#[test]
fn five_hot_jupiters_find_no_room() {
    let inputs = Inputs {
        request: Request::System(SystemRequest {
            host_mass: Some(SolarMass(1.0)),
            metallicity: None,
            archetypes: asked(&[("hot-jupiter", 5)]),
        }),
        ..system_inputs(3)
    };
    assert_eq!(
        draw_with(&inputs, &mut Trace::new(), model).unwrap_err(),
        SystemError::NoRoom {
            archetype: id("hot-jupiter")
        }
    );
}

#[test]
fn a_huge_count_is_refused_at_once() {
    let inputs = asking(1, &[("super-earth", 4_000_000_000)]);
    match draw_with(&inputs, &mut Trace::new(), model).unwrap_err() {
        SystemError::TooManyPlanets { count } => assert!(count >= 4_000_000_000, "{count}"),
        other => panic!("{other}"),
    }
}

#[test]
fn the_planet_cap_counts_asked_and_family_slots() {
    assert_eq!(MAX_PLANETS, 650);
    let mut parts = parts(1, 1.0, Family::Sparse);
    parts.2.counts = Counts {
        small: 0,
        giants: 0,
    };
    let refused = run(1, &parts, &asked(&[("hot-neptune", 650)]), &model);
    assert_eq!(
        refused.unwrap_err(),
        SystemError::TooManyPlanets { count: 650 }
    );
    parts.2.counts = Counts {
        small: 1,
        giants: 1,
    };
    let refused = run(1, &parts, &asked(&[("hot-neptune", 648)]), &model);
    assert_eq!(
        refused.unwrap_err(),
        SystemError::TooManyPlanets { count: 650 }
    );
    // 649 is below the cap: the slots run, and the hot Neptunes run out of
    // room long before the last one.
    parts.2.counts = Counts {
        small: 0,
        giants: 0,
    };
    let tried = run(1, &parts, &asked(&[("hot-neptune", 649)]), &model);
    assert_eq!(
        tried.unwrap_err(),
        SystemError::NoRoom {
            archetype: id("hot-neptune")
        }
    );
}

#[test]
fn an_asked_planet_is_never_moved() {
    for seed in 0..5 {
        for name in ["earth-like", "super-earth"] {
            let mut seen = Vec::new();
            for family in FAMILIES {
                let parts = parts(seed, 1.0, family);
                let placed = run(seed, &parts, &asked(&[(name, 1)]), &model).unwrap();
                let planet = placed.iter().find(|p| p.asked == Some(id(name))).unwrap();
                seen.push((
                    planet.trial.mass.value().0,
                    axis(planet),
                    planet.trial.orbit.eccentricity.value().get(),
                ));
                if name == "earth-like" {
                    let luminosity = parts.0.state.luminosity.value().0;
                    assert_eq!(seen.last(), Some(&(1.0, luminosity.sqrt(), 0.017)));
                }
            }
            assert!(seen.windows(2).all(|w| w[0] == w[1]), "{name}: {seen:?}");
        }
    }
}

#[test]
fn an_earth_like_around_2_solar_masses_draws() {
    // The Sun's 4.57 Gyr is past a 2 M☉ star's main sequence: the date
    // is drawn for the asked host instead.
    let inputs = Inputs {
        request: Request::System(SystemRequest {
            host_mass: Some(SolarMass(2.0)),
            metallicity: None,
            archetypes: asked(&[("earth-like", 1)]),
        }),
        ..system_inputs(8)
    };
    let drawn = draw_with(&inputs, &mut Trace::new(), model).unwrap();
    assert!(
        drawn
            .placements
            .iter()
            .any(|p| p.trial.archetype == Some(id("earth-like")))
    );
}

#[test]
fn asked_values_are_inputs_and_drawn_ones_draws() {
    let mut trace = Trace::new();
    let (_, placed) = draw_system(&asking(2, &[("earth-like", 1)]), &mut trace, model).unwrap();
    assert!(placed.iter().any(|p| p.asked.is_some()));
    for planet in &placed {
        let expected = if planet.asked.is_some() {
            Provenance::Input
        } else {
            Provenance::Drawn
        };
        let origin = &trace.node(planet.trial.mass.node()).unwrap().origin;
        assert_eq!(origin, &Origin::Leaf(expected));
    }
}

#[test]
fn an_asked_eccentricity_is_an_input_only_when_the_archetype_gives_one() {
    let mut trace = Trace::new();
    let inputs = asking(2, &[("earth-like", 1), ("hot-neptune", 1)]);
    let (_, placed) = draw_system(&inputs, &mut trace, model).unwrap();
    let origin = |name: &str| {
        let planet = placed.iter().find(|p| p.asked == Some(id(name))).unwrap();
        trace
            .node(planet.trial.orbit.eccentricity.node())
            .unwrap()
            .origin
            .clone()
    };
    assert_eq!(origin("earth-like"), Origin::Leaf(Provenance::Input));
    assert_eq!(origin("hot-neptune"), Origin::Leaf(Provenance::Drawn));
}

#[test]
fn an_insolation_band_traces_its_flux_and_the_class_its_circular_trial() {
    let mut trace = Trace::new();
    let (_, placed) = draw_system(&asking(2, &[("earth-like", 1)]), &mut trace, model).unwrap();
    let earth = placed.iter().find(|p| p.asked.is_some()).unwrap();
    let from = |node| match &trace.node(node).unwrap().origin {
        Origin::Derived { from } => from.clone(),
        other => panic!("{other:?}"),
    };
    let code = |node| trace.node(node).unwrap().code.clone();
    // The axis comes from the asked flux (an input leaf) and the luminosity.
    let axis_causes = from(earth.trial.orbit.semi_major_axis.node());
    assert_eq!(code(axis_causes[0]), Code::of("system.draw.insolation"));
    assert_eq!(
        trace.node(axis_causes[0]).unwrap().origin,
        Origin::Leaf(Provenance::Input)
    );
    // The class cites the circular trial's flux, not the eccentric orbit's.
    let class = (0..trace.len())
        .map(|i| serde_json::from_value(serde_json::json!(i)).unwrap())
        .find(|&node| {
            code(node) == Code::of("system.draw.class")
                && from(node).contains(&earth.trial.mass.node())
        })
        .expect("the class node of the earth-like");
    let causes = from(class);
    let final_flux = earth.trial.sources.primary().flux.node();
    assert!(!causes.contains(&final_flux));
    assert!(
        causes
            .iter()
            .any(|&node| code(node) == Code::of("system.draw.flux"))
    );
}

#[test]
fn the_solar_giants_can_be_asked_together() {
    let giants = ["jupiter-like", "saturn-like", "uranus-like", "neptune-like"];
    for seed in 0..10 {
        let inputs = asking(seed, &giants.map(|name| (name, 1)));
        let (_, placed) = draw_system(&inputs, &mut Trace::new(), model).unwrap();
        for name in giants {
            assert!(placed.iter().any(|p| p.asked == Some(id(name))), "{name}");
        }
    }
}

#[test]
fn a_sparse_system_may_be_empty() {
    let mut parts = parts(4, 1.0, Family::Sparse);
    parts.2.counts = Counts {
        small: 0,
        giants: 0,
    };
    assert!(run(4, &parts, &BTreeMap::new(), &model).unwrap().is_empty());
    let empty = (0..500).find_map(|seed| {
        let (drawn, placed) = draw_system(&system_inputs(seed), &mut Trace::new(), model).unwrap();
        (drawn.family == Family::Sparse && placed.is_empty()).then_some(drawn)
    });
    let drawn = empty.expect("an empty sparse system among 500 seeds");
    assert!(drawn.placements.is_empty());
}

#[test]
fn a_hot_jupiter_clears_its_neighbourhood() {
    let mut hot = 0;
    for seed in 0..20 {
        for request in [asked(&[]), asked(&[("mini-neptune", 1)])] {
            let parts = parts(seed, 1.0, Family::HotJupiter);
            let placed = run(seed, &parts, &request, &model).unwrap();
            let zone = parts.2.ice_line.0 / 2.7;
            let hot_jupiters = placed
                .iter()
                .filter(|p| p.outcome.class == PlanetClass::HotJupiter)
                .count();
            hot += hot_jupiters;
            if hot_jupiters > 0 {
                let inside = placed.iter().filter(|p| axis(p) < zone).count();
                assert_eq!(inside, hot_jupiters, "seed {seed}");
            }
            if !request.is_empty() {
                // The asked mini-Neptune sits inside the zone: no hot Jupiter.
                assert_eq!(hot_jupiters, 0, "seed {seed}");
            }
        }
    }
    assert!(hot > 0, "no hot Jupiter drawn");
}

/// The system mass and the `ln M` of the planets of a compact system.
fn ln_masses(
    seed: u64,
    classify: impl Fn(&Trial, &Trace) -> Result<TrialOutcome, SystemError>,
) -> (f64, Vec<f64>) {
    let parts = parts(seed, 1.0, Family::Compact);
    let placed = run(seed, &parts, &BTreeMap::new(), &classify).unwrap();
    let masses = placed.iter().map(|p| p.trial.mass.value().0.ln()).collect();
    (parts.2.system_mass.0, masses)
}

fn variance(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
}

#[test]
fn compact_planets_have_similar_masses() {
    let systems: Vec<Vec<f64>> = (0..60)
        .map(|seed| ln_masses(seed, model).1)
        .filter(|masses| masses.len() >= 2)
        .collect();
    let within = systems.iter().map(|m| variance(m)).sum::<f64>() / systems.len() as f64;
    let across = variance(&systems.concat());
    assert!(within < 0.5 * across, "within {within}, across {across}");
    assert!(within > 0.0, "no scatter at all");
}

#[test]
fn a_flat_model_keeps_compact_planets_at_the_system_mass() {
    let flat = |trial: &Trial, trace: &Trace| {
        model(trial, trace).map(|outcome| TrialOutcome {
            radius: EarthRadius(1.0),
            ..outcome
        })
    };
    for seed in 0..10 {
        let (mass, masses) = ln_masses(seed, flat);
        assert!(masses.iter().all(|&m| m == mass.ln()), "seed {seed}");
    }
}

#[test]
fn the_slope_probe_stays_inside_one_envelope_regime() {
    let probe = |m: f64| probe_mass(EarthMass(m)).map(|m| m.0);
    assert_eq!(probe(3.0), Some(3.0 * 1.1));
    assert_eq!(probe(5.0), Some(5.0 * 1.1));
    assert_eq!(probe(4.8), Some(4.8 / 1.1));
    assert_eq!(probe(19.0), Some(19.0 / 1.1));
    assert_eq!(probe(0.095), Some(0.095 / 1.1));
}

#[test]
fn a_classifier_error_on_an_asked_planet_propagates() {
    let failing = |trial: &Trial, trace: &Trace| match trial.archetype {
        Some(_) => Err(SystemError::Contract(ContractError::NoSource)),
        None => model(trial, trace),
    };
    let inputs = asking(5, &[("earth-like", 1)]);
    assert_eq!(
        draw_with(&inputs, &mut Trace::new(), failing).unwrap_err(),
        SystemError::Contract(ContractError::NoSource)
    );
}

#[test]
fn a_drawn_slot_spends_its_tries_then_stays_empty() {
    let calls = Cell::new(0u32);
    let failing = |_: &Trial, _: &Trace| {
        calls.set(calls.get() + 1);
        Err(SystemError::Contract(ContractError::NoSource))
    };
    let mut parts = parts(6, 1.0, Family::HotJupiter);
    parts.2.counts = Counts {
        small: 0,
        giants: 2,
    };
    let placed = run(6, &parts, &BTreeMap::new(), &failing).unwrap();
    assert!(placed.is_empty());
    assert_eq!(calls.get(), 2 * SLOT_TRIES);
}

#[test]
fn the_recheck_drops_drawn_planets_only() {
    let mut trace = Trace::new();
    let rules = Rules {
        host: SolarMass(1.0),
        age: Gyr(4.6),
        sigma_e: 0.05,
        total: 3,
    };
    // Δ = 6 mutual Hill radii between 1 M⊕ planets near 1 au: above 2√3,
    // below K50 (about 15 at T1 = 1 yr and 4.6 Gyr).
    let hill = (2.0 * 3.003e-6 / 3.0_f64).cbrt();
    let next = |a: f64| a * (1.0 + 3.0 * hill) / (1.0 - 3.0 * hill);
    let asked_one = test_planet(&mut trace, 1.0, 1.0, 0.0, true);
    let drawn_out = test_planet(&mut trace, 1.0, next(1.0), 0.0, false);
    let far = test_planet(&mut trace, 300.0, 5.0, 0.0, false);
    let mut placed = vec![far.clone(), drawn_out, asked_one.clone()];
    recheck(&mut placed, &rules);
    assert_eq!(placed.len(), 2);
    assert!(placed[0].asked.is_some());
    assert_eq!(axis(&placed[1]), 5.0);
    // A drawn planet inside an asked one goes too.
    let drawn_in = test_planet(&mut trace, 1.0, 1.0 / next(1.0), 0.0, false);
    let mut placed = vec![drawn_in, asked_one.clone(), far];
    recheck(&mut placed, &rules);
    assert_eq!(placed.len(), 2);
    assert!(placed[0].asked.is_some());
    // Two asked neighbours face 2√3 only.
    let asked_out = test_planet(&mut trace, 1.0, next(1.0), 0.0, true);
    let mut placed = vec![asked_one, asked_out];
    recheck(&mut placed, &rules);
    assert_eq!(placed.len(), 2);
}

#[test]
fn the_recheck_drops_the_drawn_planet_placed_later() {
    let mut trace = Trace::new();
    let rules = Rules {
        host: SolarMass(1.0),
        age: Gyr(4.6),
        sigma_e: 0.05,
        total: 3,
    };
    // Δ = 6 mutual Hill radii between two drawn 1 M⊕ planets: below K50.
    let hill = (2.0 * 3.003e-6 / 3.0_f64).cbrt();
    let outer = test_planet(&mut trace, 1.0, 1.0, 0.0, false);
    let inner = test_planet(
        &mut trace,
        1.0,
        (1.0 - 3.0 * hill) / (1.0 + 3.0 * hill),
        0.0,
        false,
    );
    // The outer one was placed first (as an eccentric family's first
    // planet): the inner one, placed later, goes.
    let mut placed = vec![outer, inner];
    recheck(&mut placed, &rules);
    assert_eq!(placed.len(), 1);
    assert_eq!(axis(&placed[0]), 1.0);
}

#[test]
fn a_pair_with_a_giant_keeps_the_two_planet_floor_only() {
    let mut trace = Trace::new();
    let rules = Rules {
        host: SolarMass(1.0),
        age: Gyr(4.6),
        sigma_e: 0.05,
        total: 3,
    };
    // Δ = 6 mutual Hill radii: above 2√3, below K50 (about 15 here).
    let next = |a: f64, mass: f64| {
        let hill = (2.0 * mass * 3.003e-6 / 3.0_f64).cbrt();
        a * (1.0 + 3.0 * hill) / (1.0 - 3.0 * hill)
    };
    let inner = test_planet(&mut trace, 300.0, 5.0, 0.0, false);
    let outer = test_planet(&mut trace, 300.0, next(5.0, 300.0), 0.0, false);
    let mut placed = vec![inner, outer];
    recheck(&mut placed, &rules);
    assert_eq!(placed.len(), 2, "two drawn giants");
    let small = test_planet(&mut trace, 1.0, 1.0, 0.0, false);
    let small_out = test_planet(&mut trace, 1.0, next(1.0, 1.0), 0.0, false);
    let mut placed = vec![small, small_out];
    recheck(&mut placed, &rules);
    assert_eq!(placed.len(), 1, "two drawn small planets");
}

#[test]
fn designations_follow_distance() {
    let mut checked = 0;
    for seed in 0..40 {
        let inputs = asking(seed, &[("earth-like", 1), ("hot-neptune", 1)]);
        let drawn = draw_with(&inputs, &mut Trace::new(), model).unwrap();
        let system = drawn.designation.system;
        let axes: Vec<f64> = drawn
            .placements
            .iter()
            .map(|p| p.trial.orbit.semi_major_axis.value().0)
            .collect();
        assert!(axes.windows(2).all(|pair| pair[0] <= pair[1]), "{axes:?}");
        for (rank, placement) in (0_u32..).zip(&drawn.placements) {
            let designation = Designation::planet(system, rank).unwrap();
            assert_eq!(placement.designation, designation);
            assert_eq!(
                placement.seed,
                Seed::new(seed).derive(&designation.to_string())
            );
            checked += 1;
        }
    }
    assert!(checked >= 80, "{checked}");
}

#[test]
fn a_planet_spin_comes_from_its_body_seed() {
    let mut trace = Trace::new();
    let drawn = draw_with(&asking(5, &[("earth-like", 1)]), &mut trace, model).unwrap();
    let planet = drawn
        .placements
        .iter()
        .find(|p| p.trial.archetype == Some(id("earth-like")))
        .unwrap();
    let outcome = model(&planet.trial, &Trace::new()).unwrap();
    let again = spin::spin(
        outcome.class,
        &planet.trial.mass,
        outcome.radius,
        crate::archetype::placement(&id("earth-like")),
        planet.seed,
        &mut trace,
    )
    .unwrap();
    assert_eq!(
        again.rotation_period.value(),
        planet.spin.rotation_period.value()
    );
    assert_eq!(again.obliquity.value(), planet.spin.obliquity.value());
}

#[test]
fn adding_a_hostless_archetype_keeps_the_star_family_and_slots() {
    let slot_seeds = |placed: &[Placed]| -> Vec<Seed> {
        placed
            .iter()
            .filter(|p| p.asked.is_none())
            .map(|p| p.trial.seed)
            .collect()
    };
    let (mut compared, mut shared) = (0, 0);
    for seed in 0..60 {
        let (plain, without) = draw_system(&system_inputs(seed), &mut Trace::new(), model).unwrap();
        let (added, with) = draw_system(
            &asking(seed, &[("hot-neptune", 1)]),
            &mut Trace::new(),
            model,
        )
        .unwrap();
        assert_eq!(plain.star, added.star);
        assert_eq!(plain.family, added.family);
        assert_eq!(plain.designation, added.designation);
        // A slot of the plain draw may be lost to the asked planet, and
        // one lost there may be kept here (its retries meet other
        // neighbours), but every kept slot is on its own stream.
        let before = slot_seeds(&without);
        let streams: Vec<Seed> = (0..MAX_PLANETS)
            .map(|n| Seed::new(seed).derive(&format!("trial/slot-{n}")))
            .collect();
        for slot in slot_seeds(&with) {
            assert!(streams.contains(&slot), "seed {seed}: {slot:?}");
            shared += usize::from(before.contains(&slot));
            compared += 1;
        }
        for planet in with.iter().filter(|p| p.asked.is_some()) {
            assert_eq!(
                planet.trial.seed,
                Seed::new(seed).derive("trial/hot-neptune")
            );
        }
    }
    assert!(compared > 20, "{compared}");
    assert!(shared * 10 >= compared * 8, "{shared} of {compared}");
}
