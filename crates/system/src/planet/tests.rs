//! Tier 1: the composition draws, the interior and the heat of one planet.

use super::codes;
use super::draws::{self, Drawn, LABELS, Labels};
use super::heat::{self, ENVELOPED_MIN_ENVELOPE, ICY_MIN_WATER};
use super::interior::{InteriorError, interior};
use crate::Trial;
use science::constants::EARTH_MEAN_RADIUS;
use science::heat::{TidalKind, k2_over_q};
use science::units::{AstronomicalUnit, Day, Degree, Dex, EarthMass, Fraction, Gyr, Kelvin};
use science::units::{EarthFlux, SolarLuminosity, SolarMass};
use timeline::{
    Code, NodeValueOf, Orbit, Origin, Provenance, Seed, StellarSource, StellarSources, Trace,
    Traced,
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
fn the_same_trial_draws_the_same_composition() {
    let mut a = Trace::new();
    let mut b = Trace::new();
    let first = draws::draw(&trial(&mut a, 7, 1.0, 1.0, 0.0), &mut a).unwrap();
    let second = draws::draw(&trial(&mut b, 7, 1.0, 1.0, 0.0), &mut b).unwrap();
    assert_eq!(first, second);
    let bits = |d: &Drawn| {
        [
            d.core_mass_fraction.value().get().to_bits(),
            d.water_fraction.value().get().to_bits(),
            d.radiogenic_abundance.value().0.to_bits(),
        ]
    };
    assert_eq!(bits(&first), bits(&second));
}

#[test]
fn different_labels_draw_independently() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 7, 1.0, 1.0, 0.0);
    let base = draws::draw(&t, &mut trace).unwrap();
    let other = Labels {
        water: "planet/water-other",
        ..LABELS
    };
    let changed = draws::draw_with(&t, &mut trace, &other).unwrap();
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
fn draws_are_leaves_of_the_trace() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 3, 1.0, 1.0, 0.0);
    let d = draws::draw(&t, &mut trace).unwrap();
    for (node, code) in [
        (d.core_mass_fraction.node(), codes::CORE_MASS_FRACTION),
        (d.water_fraction.node(), codes::WATER_FRACTION),
        (d.radiogenic_abundance.node(), codes::RADIOGENIC_ABUNDANCE),
    ] {
        let recorded = trace.node(node).unwrap();
        assert_eq!(recorded.code.as_str(), code);
        assert_eq!(recorded.origin, Origin::Leaf(Provenance::Drawn));
    }
}

#[test]
fn draws_follow_their_sources() {
    const N: u64 = 10_000;
    let mut cmf: Vec<f64> = (0..N)
        .map(|i| draws::core_mass_fraction(Seed::new(i)))
        .collect();
    cmf.sort_by(f64::total_cmp);
    let median = cmf[(N / 2) as usize];
    assert!((median - 0.32).abs() < 0.01, "median {median}");
    // Definition: the 16th percentile of a normal, Phi(-1) = 0.1587, is
    // the 0.32 - 0.12 = 0.20 of Plotnykov & Valencia 2020.
    let below = cmf.iter().filter(|&&c| c < 0.20).count() as f64 / N as f64;
    assert!((below - 0.1587).abs() < 0.02, "share below 0.20: {below}");
    assert!(cmf.iter().all(|c| (0.0..=1.0).contains(c)));

    let wet: Vec<f64> = (0..N)
        .map(|i| draws::water_fraction(Seed::new(i), true))
        .collect();
    assert!(wet.iter().all(|w| (0.5..=0.67).contains(w)));

    let mut dry: Vec<f64> = (0..N)
        .map(|i| draws::water_fraction(Seed::new(i), false))
        .collect();
    dry.sort_by(f64::total_cmp);
    // Source: Marty 2012, 2700 ppm bulk Earth.
    assert!((dry[(N / 2) as usize] / EARTH_WATER_FRACTION - 1.0).abs() < 0.1);

    let dex: Vec<f64> = (0..N)
        .map(|i| draws::radiogenic_abundance(Seed::new(i)))
        .collect();
    let mean = dex.iter().sum::<f64>() / N as f64;
    let sd = (dex.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / N as f64).sqrt();
    assert!(
        mean.abs() < 0.01 && (sd - 0.23).abs() < 0.01,
        "mean {mean} sd {sd}"
    );
}

#[test]
fn a_trial_beyond_the_ice_line_draws_an_icy_planet() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 11, 1.0, 5.0, 0.0);
    let d = draws::draw(&t, &mut trace).unwrap();
    assert!((0.5..=0.67).contains(&d.water_fraction.value().get()));
}

#[test]
fn an_earth_trial_has_earths_bulk() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, 1.0, 0.0167);
    let drawn = Drawn::fixed(
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
    );
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    let r = interior.structure.radius.0;
    assert!((r / EARTH_MEAN_RADIUS.0 - 1.0).abs() < 0.01, "radius {r}");
    let g = interior.bulk.surface_gravity.value().0;
    assert!((g / STANDARD_GRAVITY - 1.0).abs() < 0.01, "gravity {g}");
    // Every recorded node resolves in the trace.
    let mut nodes = vec![
        interior.bulk.mass.node(),
        interior.bulk.radius.node(),
        interior.bulk.surface_gravity.node(),
        interior.bulk.mean_density.node(),
        interior.composition.core_mass_fraction.node(),
        interior.composition.water_fraction.node(),
    ];
    for shell in &interior.shells {
        nodes.extend([
            shell.outer_radius.node(),
            shell.mass.node(),
            shell.material.node(),
            shell.state.node(),
        ]);
    }
    assert!(!interior.shells.is_empty());
    for node in nodes {
        assert!(trace.node(node).is_some());
    }
    assert_eq!(
        trace
            .node(interior.bulk.radius.node())
            .unwrap()
            .code
            .as_str(),
        codes::RADIUS
    );
}

#[test]
fn a_hot_water_world_is_refused() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, 0.1, 0.0);
    let drawn = Drawn::fixed(&mut trace, 0.3, 0.1, 0.0);
    assert!(matches!(
        interior(&t, &drawn, &mut trace),
        Err(InteriorError::WaterTooHot { .. })
    ));
}

#[test]
fn heat_nodes_point_at_their_inputs() {
    // Dry, so that a planet at 0.05 au is not a hot water world.
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, 0.05, 0.1);
    let drawn = Drawn::fixed(&mut trace, EARTH_CORE_MASS_FRACTION, 0.0, 0.1);
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    let heat = heat::heat(&t, &interior, &drawn.radiogenic_abundance, &mut trace).unwrap();
    let sources = |node| match &trace.node(node).unwrap().origin {
        Origin::Derived { from } => from.clone(),
        _ => panic!("a heat node is derived"),
    };
    let radiogenic = sources(heat.radiogenic.node());
    assert!(radiogenic.contains(&drawn.radiogenic_abundance.node()));
    assert!(radiogenic.contains(&t.age.node()));
    assert!(radiogenic.contains(&drawn.core_mass_fraction.node()));
    let secular = sources(heat.secular.node());
    assert!(secular.contains(&t.age.node()));
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
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
    );
    let interior = interior(&t, &drawn, &mut trace).unwrap();
    let heat = heat::heat(&t, &interior, &drawn.radiogenic_abundance, &mut trace).unwrap();
    assert_eq!(heat.tidal.value().0, 0.0);
    // Earth's radiogenic power is about 20 TW (Dye 2012, Table 7).
    let radiogenic = heat.radiogenic.value().0;
    assert!(
        (5e12..=40e12).contains(&radiogenic),
        "radiogenic {radiogenic}"
    );
}

#[test]
fn the_tidal_kind_follows_the_composition() {
    assert_eq!(heat::tidal_kind(0.0, 2.7e-3), TidalKind::Rocky);
    assert_eq!(heat::tidal_kind(0.0, ICY_MIN_WATER), TidalKind::Icy);
    assert_eq!(
        heat::tidal_kind(ENVELOPED_MIN_ENVELOPE, 0.0),
        TidalKind::Enveloped
    );
    assert!(k2_over_q(TidalKind::Icy) < k2_over_q(TidalKind::Rocky));
}

#[test]
fn every_code_is_valid() {
    for code in [
        codes::CORE_MASS_FRACTION,
        codes::WATER_FRACTION,
        codes::RADIOGENIC_ABUNDANCE,
        codes::WATER_TEMPERATURE,
        codes::RADIUS,
        codes::SURFACE_GRAVITY,
        codes::MEAN_DENSITY,
        codes::LAYER_OUTER_RADIUS,
        codes::LAYER_MASS,
        codes::LAYER_MATERIAL,
        codes::LAYER_STATE,
        codes::HEAT_RADIOGENIC,
        codes::HEAT_TIDAL,
        codes::HEAT_SECULAR,
    ] {
        let _ = Code::of(code);
    }
}
