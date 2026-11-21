//! Tier 1: the composition draws, the interior and the heat of one planet.

use super::codes;
use super::draws::{self, Drawn, LABELS, Labels};
use super::heat::{self, ENVELOPED_MIN_ENVELOPE, ICY_MIN_WATER};
use super::interior::{Interior, Water, interior};
use crate::Trial;
use science::constants::EARTH_MEAN_RADIUS;
use science::heat::TidalKind;
use science::units::{AstronomicalUnit, Day, Degree, Dex, EarthMass, Fraction, Gyr, Kelvin};
use science::units::{EarthFlux, SolarLuminosity, SolarMass};
use timeline::{
    Code, Material, NodeValueOf, Orbit, Origin, Provenance, Seed, StellarSource, StellarSources,
    Trace, Traced,
};

/// Source: the 1901 definition of standard gravity by the 3rd CGPM,
/// 9.80665 m/s^2.
const STANDARD_GRAVITY: f64 = 9.80665;

/// Source: Marty 2012, bulk Earth water 2700 ppm of the mass (Earth
/// Planet. Sci. Lett. 313, 56, p. 18).
const EARTH_WATER_FRACTION: f64 = 2.7e-3;

/// Source: Wang et al. 2018 core mass fraction of Earth, as quoted by
/// Plotnykov & Valencia 2020, 0.325.
const EARTH_CORE_MASS_FRACTION: f64 = 0.325;

/// Definition: a water world's water fraction for the tests, 0.5, the
/// lower bound of Zeng et al. 2019's icy planets (a test choice).
const WATER_WORLD_FRACTION: f64 = 0.5;

/// Definition: 0.4 au around the trial's Sun-like source, an equilibrium
/// temperature of 255 / sqrt(0.4) = 403 K, above the 355 K where the water
/// laws end (a test choice).
const HOT_ORBIT_AU: f64 = 0.4;

/// Definition: the normal quantile of the 16th and 84th percentiles,
/// Phi(-1) = 1 - Phi(1) = 0.1587.
const ONE_SIGMA_TAIL: f64 = 0.1587;

/// Definition: sampling tolerance of a share, a median or a moment over
/// 10 000 draws, several standard errors (about 0.004 for a tail share).
const SAMPLING: f64 = 0.02;

fn leaf<T: NodeValueOf>(trace: &mut Trace, code: &'static str, value: T) -> Traced<T> {
    trace
        .leaf(Code::of(code), value, Provenance::Input)
        .unwrap()
}

/// Builds a trial of `mass` Earth masses at `au` around one 1 solar-mass
/// source, with the ice line at 2.7 au.
fn trial(trace: &mut Trace, seed: u64, mass: f64, au: f64, ecc: f64) -> Trial {
    let source = StellarSource {
        flux: leaf(trace, "test.flux", EarthFlux(1.0 / (au * au))),
        // Source: 255 K at one Earth flux, the Bond albedo 0.3 of the
        // timeline's `StellarSource`, scaled by flux^(1/4).
        equilibrium: leaf(trace, "test.equilibrium", Kelvin(255.0 / au.sqrt())),
        effective_temperature: leaf(trace, "test.teff", Kelvin(5772.0)),
        mass: leaf(trace, "test.star-mass", SolarMass(1.0)),
        luminosity: leaf(trace, "test.luminosity", SolarLuminosity(1.0)),
    };
    let orbit = Orbit {
        semi_major_axis: leaf(trace, "test.a", AstronomicalUnit(au)),
        eccentricity: leaf(trace, "test.e", Fraction::new(ecc).unwrap()),
        period: leaf(trace, "test.period", Day(365.25 * au.powf(1.5))),
        perihelion_longitude: leaf(trace, "test.varpi", Degree(0.0)),
    };
    Trial {
        seed: Seed::new(seed),
        mass: leaf(trace, "test.mass", EarthMass(mass)),
        orbit,
        sources: StellarSources::new(vec![source]).unwrap(),
        age: leaf(trace, "test.age", Gyr(4.5673)),
        metallicity: leaf(trace, "test.metallicity", Dex(0.0)),
        ice_line: leaf(trace, "test.ice-line", AstronomicalUnit(2.7)),
        archetype: None,
    }
}

#[test]
fn the_same_seed_draws_the_same_composition() {
    let mut a = Trace::new();
    let mut b = Trace::new();
    let ta = trial(&mut a, 7, 1.0, 1.0, 0.0);
    let tb = trial(&mut b, 7, 1.0, 1.0, 0.0);
    let first = draws::draw(ta.seed, &ta, &mut a).unwrap();
    let second = draws::draw(tb.seed, &tb, &mut b).unwrap();
    assert_eq!(first, second);
    let bits = |d: &Drawn| {
        [
            d.core_mass_fraction.value().get().to_bits(),
            d.water_fraction.value().get().to_bits(),
            d.radiogenic_abundance.value().0.to_bits(),
            u64::from(*d.migrated.value()),
        ]
    };
    assert_eq!(bits(&first), bits(&second));
}

#[test]
fn the_draw_follows_the_seed_it_is_given_not_the_trials() {
    // `model` passes the body seed, `class_of` the trial seed: the draw
    // reads only the seed it is given.
    let mut a = Trace::new();
    let mut b = Trace::new();
    let ta = trial(&mut a, 7, 1.0, 1.0, 0.0);
    let tb = trial(&mut b, 8, 1.0, 1.0, 0.0);
    let body = Seed::new(42);
    let first = draws::draw(body, &ta, &mut a).unwrap();
    let second = draws::draw(body, &tb, &mut b).unwrap();
    assert_eq!(first, second);
    let other = draws::draw(Seed::new(43), &ta, &mut a).unwrap();
    assert_ne!(
        first.core_mass_fraction.value(),
        other.core_mass_fraction.value()
    );
}

#[test]
fn different_labels_draw_independently() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 7, 1.0, 1.0, 0.0);
    let base = draws::draw(t.seed, &t, &mut trace).unwrap();
    let other = Labels {
        water: "planet/water-other",
        ..LABELS
    };
    let changed = draws::draw_with(t.seed, &t, &mut trace, &other).unwrap();
    assert_eq!(
        base.core_mass_fraction.value(),
        changed.core_mass_fraction.value()
    );
    assert_eq!(
        base.radiogenic_abundance.value(),
        changed.radiogenic_abundance.value()
    );
    assert_ne!(base.water_fraction.value(), changed.water_fraction.value());
}

#[test]
fn the_trace_says_why_a_planet_is_a_water_world() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 3, 1.0, 1.0, 0.0);
    let d = draws::draw(t.seed, &t, &mut trace).unwrap();
    for (node, code) in [
        (d.core_mass_fraction.node(), codes::CORE_MASS_FRACTION),
        (d.migrated.node(), codes::MIGRATED),
        (d.radiogenic_abundance.node(), codes::RADIOGENIC_ABUNDANCE),
    ] {
        let recorded = trace.node(node).unwrap();
        assert_eq!(recorded.code.as_str(), code);
        assert_eq!(recorded.origin, Origin::Leaf(Provenance::Drawn));
    }
    let beyond = trace.node(d.formed_beyond_ice_line.node()).unwrap();
    assert_eq!(beyond.code.as_str(), codes::FORMED_BEYOND_ICE_LINE);
    assert_eq!(
        beyond.origin,
        Origin::Derived {
            from: vec![t.orbit.semi_major_axis.node(), t.ice_line.node()]
        }
    );
    let water = trace.node(d.water_fraction.node()).unwrap();
    assert_eq!(water.code.as_str(), codes::WATER_FRACTION);
    assert_eq!(
        water.origin,
        Origin::Derived {
            from: vec![d.formed_beyond_ice_line.node(), d.migrated.node()]
        }
    );
}

/// Draws per distribution test.
const N: u64 = 10_000;

#[test]
fn the_core_mass_fraction_follows_plotnykov_and_valencia() {
    let mut cmf: Vec<f64> = (0..N)
        .map(|i| draws::core_mass_fraction(Seed::new(i)))
        .collect();
    cmf.sort_by(f64::total_cmp);
    let median = cmf[(N / 2) as usize];
    assert!((median - 0.32).abs() < SAMPLING / 2.0, "median {median}");
    // Plotnykov & Valencia 2020: 0.32 +0.14/-0.12, the 16th percentile at
    // 0.20 and the 84th at 0.46.
    let share =
        |keep: &dyn Fn(f64) -> bool| cmf.iter().filter(|&&c| keep(c)).count() as f64 / N as f64;
    let below = share(&|c| c < 0.20);
    assert!(
        (below - ONE_SIGMA_TAIL).abs() < SAMPLING,
        "below 0.20: {below}"
    );
    let above = share(&|c| c > 0.46);
    assert!(
        (above - ONE_SIGMA_TAIL).abs() < SAMPLING,
        "above 0.46: {above}"
    );
    assert!(cmf.iter().all(|c| (0.0..=1.0).contains(c)));
}

#[test]
fn the_water_fraction_follows_zeng_and_marty() {
    let wet: Vec<f64> = (0..N)
        .map(|i| draws::water_fraction(Seed::new(i), true).fraction)
        .collect();
    assert!(wet.iter().all(|w| (0.5..=0.67).contains(w)));

    // Inside the ice line: a dry log-normal around Marty 2012's 2700 ppm
    // (its median), or an icy fraction for the migrated planets.
    let draws: Vec<_> = (0..N)
        .map(|i| draws::water_fraction(Seed::new(i), false))
        .collect();
    let mut dry: Vec<f64> = draws
        .iter()
        .filter(|d| !d.migrated)
        .map(|d| d.fraction)
        .collect();
    assert!(
        draws
            .iter()
            .filter(|d| d.migrated)
            .all(|d| d.fraction >= 0.5)
    );
    dry.sort_by(f64::total_cmp);
    let median = dry[dry.len() / 2];
    assert!(
        (median / EARTH_WATER_FRACTION - 1.0).abs() < 0.1,
        "median {median}"
    );
}

#[test]
fn the_radiogenic_abundance_follows_nimmo() {
    let dex: Vec<f64> = (0..N)
        .map(|i| draws::radiogenic_abundance(Seed::new(i)))
        .collect();
    let mean = dex.iter().sum::<f64>() / N as f64;
    let sd = (dex.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / N as f64).sqrt();
    // Nimmo et al. 2020: 0.23 dex, centred on Earth.
    assert!(
        mean.abs() < SAMPLING / 2.0 && (sd - 0.23).abs() < SAMPLING / 2.0,
        "mean {mean} sd {sd}"
    );
}

#[test]
fn the_ice_line_decides_a_water_world() {
    for i in 0..100 {
        let mut trace = Trace::new();
        let far = trial(&mut trace, i, 1.0, 5.0, 0.0);
        let d = draws::draw(far.seed, &far, &mut trace).unwrap();
        assert!(*d.formed_beyond_ice_line.value() && d.is_water_world());
        assert!((0.5..=0.67).contains(&d.water_fraction.value().get()));

        let near = trial(&mut trace, i, 1.0, 1.0, 0.0);
        let d = draws::draw(near.seed, &near, &mut trace).unwrap();
        assert!(!*d.formed_beyond_ice_line.value());
        assert_eq!(d.is_water_world(), *d.migrated.value());
        if !d.is_water_world() {
            assert!(d.water_fraction.value().get() < 0.5);
        }
    }
}

/// The shells of `interior` made of water or ice.
fn water_shells(interior: &Interior) -> usize {
    interior
        .shells
        .iter()
        .filter(|shell| matches!(shell.material.value(), Material::Water | Material::Ice))
        .count()
}

#[test]
fn an_earth_trial_has_earths_bulk_and_no_water_layer() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, 1.0, 0.0167);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
        false,
    );
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    let r = interior.structure.radius.0;
    assert!((r / EARTH_MEAN_RADIUS.0 - 1.0).abs() < 0.01, "radius {r}");
    let g = interior.bulk.surface_gravity.value().0;
    assert!((g / STANDARD_GRAVITY - 1.0).abs() < 0.01, "gravity {g}");
    // Earth's bulk water is bound in its mantle: no layer of its own.
    assert_eq!(interior.water, Water::InMantle);
    assert_eq!(water_shells(&interior), 0);
    assert!(!interior.shells.is_empty());

    // The radius and every shell derive from the structure node, which
    // derives from the inputs; a shell also from its layer's mass.
    let from = |node| match &trace.node(node).unwrap().origin {
        Origin::Derived { from } => from.clone(),
        origin => panic!("{origin:?} is not derived"),
    };
    let structure = interior.central_pressure.node();
    assert_eq!(
        from(structure),
        vec![
            t.mass.node(),
            drawn.core_mass_fraction.node(),
            drawn.water_fraction.node()
        ]
    );
    assert_eq!(from(interior.bulk.radius.node()), vec![structure]);
    assert_eq!(
        trace
            .node(interior.bulk.radius.node())
            .unwrap()
            .code
            .as_str(),
        codes::RADIUS
    );
    let layer_masses = [interior.core_mass.node(), interior.mantle_mass.node()];
    for shell in &interior.shells {
        for node in [
            shell.outer_radius.node(),
            shell.mass.node(),
            shell.material.node(),
            shell.state.node(),
        ] {
            let sources = from(node);
            assert_eq!(sources[0], structure);
            assert!(layer_masses.contains(&sources[1]), "{sources:?}");
        }
    }
    // The core's state is its equation of state's phase, read by pressure.
    let core_state = trace.node(interior.shells[0].state.node()).unwrap();
    assert_eq!(core_state.code.as_str(), codes::LAYER_STATE_BY_PRESSURE);
}

#[test]
fn a_hot_dry_planet_builds() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, HOT_ORBIT_AU, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
        false,
    );
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    assert_eq!(interior.water, Water::InMantle);
    assert_eq!(water_shells(&interior), 0);
}

#[test]
fn a_hot_water_world_keeps_its_water_as_an_inventory() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, HOT_ORBIT_AU, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        WATER_WORLD_FRACTION,
        0.0,
        true,
    );
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    assert_eq!(water_shells(&interior), 0);
    let Water::Inventory { mass, .. } = interior.water else {
        panic!("{:?}", interior.water);
    };
    assert_eq!(
        trace.node(mass.node()).unwrap().code.as_str(),
        codes::WATER_INVENTORY
    );
    // Definition: the inventory is the bulk water, w M, and the structure
    // holds the rest.
    let total = science::units::Kilogram::from(*t.mass.value()).0;
    assert!((mass.value().0 / (WATER_WORLD_FRACTION * total) - 1.0).abs() < 1e-12);
    let condensed: f64 = interior.shells.iter().map(|s| s.mass.value().0).sum();
    assert!((condensed / ((1.0 - WATER_WORLD_FRACTION) * total) - 1.0).abs() < 1e-9);
}

#[test]
fn a_temperate_water_world_has_a_water_layer() {
    // At 1 au the trial sits at 255 K: its water follows the melting curve.
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, 1.0, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        WATER_WORLD_FRACTION,
        0.0,
        true,
    );
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    let Water::Layer { mass, temperature } = interior.water else {
        panic!("{:?}", interior.water);
    };
    assert!(water_shells(&interior) > 0);
    let structure = trace.node(interior.central_pressure.node()).unwrap();
    let Origin::Derived { from } = &structure.origin else {
        panic!("the structure node is derived");
    };
    assert!(from.contains(&temperature.node()));
    let last = interior.shells.last().unwrap();
    let Origin::Derived { from } = &trace.node(last.mass.node()).unwrap().origin else {
        panic!("a shell is derived");
    };
    assert_eq!(from[1], mass.node());
}

#[test]
fn heat_nodes_point_at_their_inputs() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, 0.05, 0.1);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.1,
        false,
    );
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    let heat = heat::heat(&t, &interior, &drawn.radiogenic_abundance, &mut trace).unwrap();
    let sources = |node| match &trace.node(node).unwrap().origin {
        Origin::Derived { from } => from.clone(),
        _ => panic!("a heat node is derived"),
    };
    let radiogenic = sources(heat.radiogenic.node());
    assert!(radiogenic.contains(&drawn.radiogenic_abundance.node()));
    assert!(radiogenic.contains(&t.age.node()));
    assert!(radiogenic.contains(&interior.mantle_mass.node()));
    let secular = sources(heat.secular.node());
    assert!(secular.contains(&t.age.node()));
    assert!(secular.contains(&interior.mantle_mass.node()));
    let tidal = sources(heat.tidal.node());
    assert!(tidal.contains(&t.sources.primary().mass.node()));
    assert!(tidal.contains(&t.orbit.eccentricity.node()));
    assert!(tidal.contains(&t.orbit.semi_major_axis.node()));
    assert!(tidal.contains(&interior.bulk.radius.node()));
    assert!(heat.tidal.value().0 > 0.0);
    assert_eq!(
        trace.node(heat.tidal.node()).unwrap().code.as_str(),
        codes::HEAT_TIDAL
    );
}

#[test]
fn a_circular_orbit_has_no_tide() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, 1.0, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
        false,
    );
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    let heat = heat::heat(&t, &interior, &drawn.radiogenic_abundance, &mut trace).unwrap();
    assert_eq!(heat.tidal.value().0, 0.0);
}

#[test]
fn the_tidal_kind_follows_the_composition() {
    assert_eq!(heat::tidal_kind(0.0, 2.7e-3), TidalKind::Rocky);
    assert_eq!(heat::tidal_kind(0.0, ICY_MIN_WATER), TidalKind::Icy);
    assert_eq!(
        heat::tidal_kind(ENVELOPED_MIN_ENVELOPE, 0.0),
        TidalKind::Enveloped
    );
}
