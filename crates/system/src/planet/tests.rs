//! Tier 1: the composition draws, the interior and the heat of one planet.

use super::codes;
use super::draws::{self, Drawn, LABELS, Labels};
use super::enveloped::Body;
use super::heat::{self, ENVELOPED_MIN_ENVELOPE, ICY_MIN_WATER};
use super::interior::{Interior, Water, interior};
use crate::Trial;
use science::constants::EARTH_MEAN_RADIUS;
use science::heat::TidalKind;
use science::units::{AstronomicalUnit, Day, Degree, Dex, EarthMass, Fraction, Gyr, Kelvin};
use science::units::{
    EarthFlux, JoulePerSquareMetre, SolarLuminosity, SolarMass, WattPerSquareMetre,
};
use timeline::{
    BandLight, Code, Material, MatterState, NodeValueOf, Orbit, Origin, Provenance, Seed,
    StellarSource, StellarSources, Trace, Traced,
};

/// Source: the 1901 definition of standard gravity by the 3rd CGPM,
/// 9.80665 m/s^2.
const STANDARD_GRAVITY: f64 = 9.80665;

/// Source: Marty 2012, bulk Earth water 2700 ppm of the mass (Earth
/// Planet. Sci. Lett. 313-314, 56-66; arXiv:1405.6336 author copy,
/// p. 18).
pub(super) const EARTH_WATER_FRACTION: f64 = 2.7e-3;

/// Source: Wang et al. 2018 core mass fraction of Earth, as quoted by
/// Plotnykov & Valencia 2020, 0.325.
pub(super) const EARTH_CORE_MASS_FRACTION: f64 = 0.325;

/// Definition: a water world's water fraction for the tests, 0.5, the
/// lower bound of Zeng et al. 2019's icy planets (a test choice).
pub(super) const WATER_WORLD_FRACTION: f64 = 0.5;

/// Definition: 0.4 au around the trial's Sun-like source, an equilibrium
/// temperature of 255 / sqrt(0.4) = 403 K, above the 355 K where the water
/// laws end (a test choice).
pub(super) const HOT_ORBIT_AU: f64 = 0.4;

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
pub(super) fn trial(trace: &mut Trace, seed: u64, mass: f64, au: f64, ecc: f64) -> Trial {
    let source = StellarSource {
        flux: leaf(trace, "test.flux", EarthFlux(1.0 / (au * au))),
        // Source: 255 K at one Earth flux, the Bond albedo 0.3 of the
        // timeline's `StellarSource`, scaled by flux^(1/4).
        equilibrium: leaf(trace, "test.equilibrium", Kelvin(255.0 / au.sqrt())),
        effective_temperature: leaf(trace, "test.teff", Kelvin(5772.0)),
        mass: leaf(trace, "test.star-mass", SolarMass(1.0)),
        luminosity: leaf(trace, "test.luminosity", SolarLuminosity(1.0)),
        // Source: the Sun's XUV today, 4.6 mW/m², and over Earth's age,
        // 5.64e15 J/m² (`science::xuv`), scaled as the flux.
        xuv: BandLight {
            flux: leaf(
                trace,
                "test.xuv-flux",
                WattPerSquareMetre(4.6e-3 / (au * au)),
            ),
            fluence: leaf(
                trace,
                "test.xuv-fluence",
                JoulePerSquareMetre(5.64e15 / (au * au)),
            ),
        },
        // Source: the Sun's Lyman α today, 6.4 mW/m² (Ribas et al. 2005);
        // its eq. 2 integrated over Earth's age, held before 0.1 Gyr.
        lyman_alpha: BandLight {
            flux: leaf(
                trace,
                "test.lya-flux",
                WattPerSquareMetre(6.4e-3 / (au * au)),
            ),
            fluence: leaf(
                trace,
                "test.lya-fluence",
                JoulePerSquareMetre(2.49e15 / (au * au)),
            ),
        },
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
            d.migrated.map_or(2, |m| u64::from(*m.value())),
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
        (d.water_fraction.node(), codes::WATER_FRACTION),
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
    // Inside the ice line the migration draw is its own leaf.
    let migrated = d.migrated.expect("inside the ice line");
    let recorded = trace.node(migrated.node()).unwrap();
    assert_eq!(recorded.code.as_str(), codes::MIGRATED);
    assert_eq!(recorded.origin, Origin::Leaf(Provenance::Drawn));
    // Gap 1: beyond the ice line too, the water fraction is a drawn leaf,
    // the flags recorded apart, never its inputs.
    let far = trial(&mut trace, 3, 1.0, 5.0, 0.0);
    let d = draws::draw(far.seed, &far, &mut trace).unwrap();
    let water = trace.node(d.water_fraction.node()).unwrap();
    assert_eq!(water.code.as_str(), codes::WATER_FRACTION);
    assert_eq!(water.origin, Origin::Leaf(Provenance::Drawn));
}

#[test]
fn no_migration_is_recorded_beyond_the_ice_line() {
    // Gap 4: beyond the ice line there is nothing to migrate from; the
    // uniform is still consumed, so the water draw does not depend on it.
    for i in 0..50 {
        let mut trace = Trace::new();
        let far = trial(&mut trace, i, 1.0, 5.0, 0.0);
        let d = draws::draw(far.seed, &far, &mut trace).unwrap();
        assert!(d.migrated.is_none());
        let near = trial(&mut trace, i, 1.0, 1.0, 0.0);
        let d = draws::draw(near.seed, &near, &mut trace).unwrap();
        assert!(d.migrated.is_some());
    }
    let mut trace = Trace::new();
    let far = trial(&mut trace, 9, 1.0, 5.0, 0.0);
    let near = trial(&mut trace, 9, 1.0, 1.0, 0.0);
    let icy = draws::water_draws(far.seed.derive(LABELS.water)).icy;
    let d = draws::draw(far.seed, &far, &mut trace).unwrap();
    assert_eq!(d.water_fraction.value().get(), icy);
    let d = draws::draw(near.seed, &near, &mut trace).unwrap();
    assert!(d.migrated.is_some());
}

/// Sub-seeds of the bound tests.
const BOUND_SEEDS: u64 = 2_000;

/// Whether `values` reach the lowest and the highest tenth of
/// `[low, high]` (on the log scale when `log`).
fn reaches_both_ends(values: &[f64], low: f64, high: f64, log: bool) -> bool {
    let at = |v: f64| if log { v.ln() } else { v };
    let (lo, hi) = (at(low), at(high));
    let tenth = (hi - lo) / 10.0;
    values.iter().any(|&v| at(v) <= lo + tenth) && values.iter().any(|&v| at(v) >= hi - tenth)
}

/// Every draw of `BOUND_SEEDS` sub-seeds: cmf, dry, icy, radiogenic.
fn bound_draws() -> [Vec<f64>; 4] {
    let seeds = (0..BOUND_SEEDS).map(Seed::new);
    let mut out: [Vec<f64>; 4] = Default::default();
    for seed in seeds {
        out[0].push(draws::core_mass_fraction(seed));
        let water = draws::water_draws(seed);
        out[1].push(water.dry);
        out[2].push(water.icy);
        out[3].push(draws::radiogenic_abundance(seed));
    }
    out
}

#[test]
fn draws_stay_within_their_published_bounds() {
    // Bounds of `draw-bounds.md` sections 1 to 3.
    let [cmf, dry, icy, radiogenic] = bound_draws();
    assert!(cmf.iter().all(|c| (0.0..=0.8).contains(c)));
    assert!(dry.iter().all(|w| (1e-5..=1e-2).contains(w)));
    assert!(icy.iter().all(|w| (0.1..=0.67).contains(w)));
    assert!(radiogenic.iter().all(|d| (-0.5..=0.5).contains(d)));
}

#[test]
fn draws_reach_both_ends() {
    let [cmf, dry, icy, radiogenic] = bound_draws();
    assert!(reaches_both_ends(&cmf, 0.0, 0.8, false));
    assert!(reaches_both_ends(&dry, 1e-5, 1e-2, true));
    assert!(reaches_both_ends(&icy, 0.1, 0.67, false));
    assert!(reaches_both_ends(&radiogenic, -0.5, 0.5, false));
}

#[test]
fn the_water_draws_are_uniform_not_concentrated() {
    // The dry water is uniform in its logarithm: half the draws lie below
    // the geometric mean of the bounds, sqrt(1e-5 * 1e-2).
    let [_, dry, icy, _] = bound_draws();
    let share = |values: &[f64], cut: f64| {
        values.iter().filter(|&&v| v < cut).count() as f64 / values.len() as f64
    };
    let below = share(&dry, (1e-5_f64 * 1e-2).sqrt());
    assert!((below - 0.5).abs() < SAMPLING * 2.0, "dry below {below}");
    let below = share(&icy, (0.1 + 0.67) / 2.0);
    assert!((below - 0.5).abs() < SAMPLING * 2.0, "icy below {below}");
}

#[test]
fn the_ice_line_decides_a_water_world() {
    for i in 0..100 {
        let mut trace = Trace::new();
        let far = trial(&mut trace, i, 1.0, 5.0, 0.0);
        let d = draws::draw(far.seed, &far, &mut trace).unwrap();
        assert!(*d.formed_beyond_ice_line.value() && d.is_water_world());
        assert!((0.1..=0.67).contains(&d.water_fraction.value().get()));

        let near = trial(&mut trace, i, 1.0, 1.0, 0.0);
        let d = draws::draw(near.seed, &near, &mut trace).unwrap();
        assert!(!*d.formed_beyond_ice_line.value());
        assert_eq!(d.is_water_world(), d.migrated.is_some_and(|m| *m.value()));
        if !d.is_water_world() {
            assert!(d.water_fraction.value().get() <= 1e-2);
        }
    }
}

/// The interior of a planet without an envelope.
fn rocky_of(body: &Body) -> &Interior {
    match body {
        Body::Rocky(interior) => interior,
        _ => panic!("not a rocky body"),
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
        Some(false),
        0.0,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let interior = rocky_of(&body);
    let r = interior.structure.radius.0;
    assert!((r / EARTH_MEAN_RADIUS.0 - 1.0).abs() < 0.01, "radius {r}");
    let g = interior.bulk.surface_gravity.value().0;
    assert!((g / STANDARD_GRAVITY - 1.0).abs() < 0.01, "gravity {g}");
    // Earth's bulk water is bound in its mantle: no layer of its own.
    assert_eq!(interior.water, Water::InMantle);
    assert_eq!(water_shells(interior), 0);
    assert!(!interior.shells.is_empty());
    // Without an envelope the body is the planet: its whole mass, its
    // shells only.
    assert_eq!(body.bulk().mass, t.mass);
    assert_eq!(body.layers().len(), interior.shells.len());

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
            drawn.water_fraction.node(),
            interior.core_mass.node(),
            interior.mantle_mass.node(),
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
        ]
        .into_iter()
        .chain(
            // The mantle's state is an engine constant, a leaf.
            (*shell.material.value() != Material::Rock).then_some(shell.state.node()),
        ) {
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
        Some(false),
        0.0,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let interior = rocky_of(&body);
    assert_eq!(interior.water, Water::InMantle);
    assert_eq!(water_shells(interior), 0);
}

#[test]
fn a_hot_water_world_keeps_its_water_beyond_the_law() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, HOT_ORBIT_AU, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        WATER_WORLD_FRACTION,
        0.0,
        Some(true),
        0.0,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let interior = rocky_of(&body);
    assert_eq!(water_shells(interior), 0);
    let Water::BeyondLaw { mass, .. } = interior.water else {
        panic!("{:?}", interior.water);
    };
    assert_eq!(
        trace.node(mass.node()).unwrap().code.as_str(),
        codes::WATER_BEYOND_LAW
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
        Some(true),
        0.0,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let interior = rocky_of(&body);
    let Water::Layer { mass, temperature } = interior.water else {
        panic!("{:?}", interior.water);
    };
    assert!(water_shells(interior) > 0);
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
        Some(false),
        0.0,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let interior = rocky_of(&body);
    let heat = heat::heat(
        &t,
        &body,
        &drawn.radiogenic_abundance,
        &drawn.envelope_fraction,
        &mut trace,
    )
    .unwrap();
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
        Some(false),
        0.0,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let heat = heat::heat(
        &t,
        &body,
        &drawn.radiogenic_abundance,
        &drawn.envelope_fraction,
        &mut trace,
    )
    .unwrap();
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

#[test]
fn a_giants_envelope_derives_from_its_drawn_heavy_mass() {
    let mut trace = Trace::new();
    let small = trial(&mut trace, 3, 1.0, 1.0, 0.0);
    let d = draws::draw(small.seed, &small, &mut trace).unwrap();
    assert!(d.heavy_mass.is_none());
    let node = trace.node(d.envelope_fraction.node()).unwrap();
    assert_eq!(node.code, Code::of(codes::ENVELOPE_FRACTION));
    assert_eq!(node.origin, Origin::Leaf(Provenance::Drawn));
    let giant = trial(&mut trace, 3, 318.0, 1.0, 0.0);
    let d = draws::draw(giant.seed, &giant, &mut trace).unwrap();
    let heavy = d.heavy_mass.unwrap();
    let node = trace.node(heavy.node()).unwrap();
    assert_eq!(node.code, Code::of(codes::HEAVY_MASS));
    assert_eq!(node.origin, Origin::Leaf(Provenance::Drawn));
    // A giant's envelope is computed, `1 - Mz / M`: derived from the mass
    // and the heavy-element mass.
    let node = trace.node(d.envelope_fraction.node()).unwrap();
    assert_eq!(node.code, Code::of(codes::ENVELOPE_FRACTION));
    assert_eq!(
        derived_from(&trace, d.envelope_fraction.node()),
        vec![giant.mass.node(), heavy.node()]
    );
    let envelope = d.envelope_fraction.value().get();
    assert!((envelope - (1.0 - heavy.value().0 / 318.0)).abs() < 1e-12);
}

#[test]
fn a_giant_above_the_tables_cites_the_mass_they_were_read_at() {
    // Definition: 4 000 M_E, above the tables' top mass, 3 593.81 M_E
    // (Fortney et al. 2007), at 5.2 au.
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 4_000.0, 5.2, 0.0);
    let drawn = draws::draw(t.seed, &t, &mut trace).unwrap();
    let Body::Giant(record) = interior(&t, &drawn, &mut trace).unwrap() else {
        panic!("a giant");
    };
    let read = derived_from(&trace, record.bulk.radius.node());
    assert_eq!(read, derived_from(&trace, record.luminosity.node()));
    assert!(!read.contains(&t.mass.node()));
    let used = read
        .iter()
        .copied()
        .find(|&n| trace.node(n).unwrap().code == Code::of(codes::TABLE_MASS_USED))
        .expect("the mass read");
    assert_eq!(derived_from(&trace, used), vec![t.mass.node()]);
    // The age asked, 4.5673 Gyr, lies inside the tables and is read as is.
    assert!(read.contains(&t.age.node()));
}

/// Every node `node` derives from, itself included, walking the
/// `Origin::Derived` links back to the leaves.
pub(super) fn ancestry(
    trace: &Trace,
    node: timeline::NodeId,
) -> std::collections::BTreeSet<timeline::NodeId> {
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![node];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if let Origin::Derived { from } = &trace.node(id).unwrap().origin {
            stack.extend(from.iter().copied());
        }
    }
    seen
}

pub(super) fn derived_from(trace: &Trace, node: timeline::NodeId) -> Vec<timeline::NodeId> {
    match &trace.node(node).unwrap().origin {
        Origin::Derived { from } => from.clone(),
        origin => panic!("{origin:?} is not derived"),
    }
}

#[test]
fn central_pressure_cites_every_input() {
    let mut trace = Trace::new();
    // Two water worlds: at 1 au its water is a layer (`Water::Layer`), at
    // 0.4 au beyond the water laws (`Water::BeyondLaw`).
    for au in [1.0, HOT_ORBIT_AU] {
        let t = trial(&mut trace, 1, 1.0, au, 0.0);
        let drawn = Drawn::fixed(
            &t,
            &mut trace,
            EARTH_CORE_MASS_FRACTION,
            WATER_WORLD_FRACTION,
            0.0,
            Some(true),
            0.0,
            None,
            None,
        );
        let body = interior(&t, &drawn, &mut trace).unwrap();
        let interior = rocky_of(&body);
        let from = derived_from(&trace, interior.central_pressure.node());
        for node in [
            t.mass.node(),
            drawn.core_mass_fraction.node(),
            drawn.water_fraction.node(),
            interior.core_mass.node(),
            interior.mantle_mass.node(),
        ] {
            assert!(from.contains(&node), "{node:?} missing from {from:?}");
        }
        match interior.water {
            Water::Layer { mass, temperature } | Water::BeyondLaw { mass, temperature } => {
                assert_eq!(matches!(interior.water, Water::Layer { .. }), au == 1.0);
                assert!(from.contains(&temperature.node()));
                assert_eq!(
                    from.contains(&mass.node()),
                    matches!(interior.water, Water::Layer { .. })
                );
            }
            Water::InMantle => panic!("a water world"),
        }
    }
}

#[test]
fn the_mantle_state_is_a_constant() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, 1.0, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
        Some(false),
        0.0,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let interior = rocky_of(&body);
    let mantle: Vec<_> = interior
        .shells
        .iter()
        .filter(|shell| *shell.material.value() == Material::Rock)
        .collect();
    assert!(!mantle.is_empty());
    for shell in mantle {
        let node = trace.node(shell.state.node()).unwrap();
        assert_eq!(node.code, Code::of(codes::LAYER_STATE));
        assert_eq!(node.origin, Origin::Leaf(Provenance::Imposed));
    }
}

#[test]
fn water_beyond_the_law_is_not_icy() {
    use science::constants::ASTRONOMICAL_UNIT;
    use science::heat::{k2_over_q, tidal};
    use science::units::{Kilogram, Metre};
    let tide = |water: f64, envelope: f64, migrated: bool| {
        let mut trace = Trace::new();
        let t = trial(&mut trace, 1, 1.0, HOT_ORBIT_AU, 0.1);
        let drawn = Drawn::fixed(
            &t,
            &mut trace,
            EARTH_CORE_MASS_FRACTION,
            water,
            0.0,
            Some(migrated),
            envelope,
            None,
            None,
        );
        let body = interior(&t, &drawn, &mut trace).unwrap();
        let heat = heat::heat(
            &t,
            &body,
            &drawn.radiogenic_abundance,
            &drawn.envelope_fraction,
            &mut trace,
        )
        .unwrap();
        let expected = |kind| {
            tidal(
                Kilogram::from(*t.sources.primary().mass.value()),
                Metre::from(*body.bulk().radius.value()),
                Metre(t.orbit.semi_major_axis.value().0 * ASTRONOMICAL_UNIT.0),
                t.orbit.eccentricity.value().get(),
                k2_over_q(kind),
            )
        };
        (
            heat.tidal.value().0,
            expected(TidalKind::Rocky).0,
            expected(TidalKind::Enveloped).0,
        )
    };
    // A water world at 403 K: its water is beyond the law, the body rocky.
    let (power, rocky, _) = tide(WATER_WORLD_FRACTION, 0.0, true);
    assert_eq!(power, rocky);
    // An enveloped planet gets the enveloped response.
    let (power, rocky, enveloped) = tide(EARTH_WATER_FRACTION, 0.01, false);
    assert_eq!(power, enveloped);
    assert_ne!(rocky, enveloped);
}

#[test]
fn water_beyond_the_law_bulk_is_the_condensed_planet() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, 1.0, HOT_ORBIT_AU, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        WATER_WORLD_FRACTION,
        0.0,
        Some(true),
        0.0,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let interior = rocky_of(&body);
    let dry = (1.0 - WATER_WORLD_FRACTION) * t.mass.value().0;
    assert!((interior.bulk.mass.value().0 / dry - 1.0).abs() < 1e-9);
    // The radius is the condensed structure's.
    assert_eq!(
        *interior.bulk.radius.value(),
        science::units::EarthRadius::from(interior.structure.radius)
    );
    let node = trace.node(interior.bulk.mass.node()).unwrap();
    assert_eq!(node.code, Code::of(codes::DRY_MASS));
    let from = derived_from(&trace, interior.bulk.mass.node());
    assert_eq!(from, vec![t.mass.node(), drawn.water_fraction.node()]);
    // The gravity and the density are the dry planet's.
    let kg = science::units::Kilogram::from(*interior.bulk.mass.value()).0;
    let r = interior.structure.radius.0;
    let g = science::constants::GRAVITATIONAL_CONSTANT * kg / (r * r);
    assert!((interior.bulk.surface_gravity.value().0 / g - 1.0).abs() < 1e-9);
    let rho = kg / (4.0 / 3.0 * std::f64::consts::PI * r.powi(3));
    assert!((interior.bulk.mean_density.value().0 / rho - 1.0).abs() < 1e-9);
    // The water mass is recorded apart.
    let Water::BeyondLaw { mass, .. } = interior.water else {
        panic!("{:?}", interior.water);
    };
    assert_eq!(
        trace.node(mass.node()).unwrap().code,
        Code::of(codes::WATER_BEYOND_LAW)
    );
}

/// Source: Jupiter's and Neptune's equatorial radii at 1 bar, 71 492 km
/// and 24 764 km (NASA fact sheets; study `giants-and-envelopes.md` §1.6).
const JUPITER_RADIUS_KM: f64 = 71_492.0;
const NEPTUNE_RADIUS_KM: f64 = 24_764.0;

/// Source: IAU 2015 nominal equatorial Earth radius, 6378.1 km, the unit
/// of `EarthRadius`.
const EARTH_RADIUS_KM: f64 = 6_378.1;

/// Source: Jupiter's mass, 318 M_E, its distance, 5.2 au, and the age of
/// the solar system as Fortney et al. 2007 Table 4 takes it, 4.5 Gyr (study
/// §1.6); a heavy-element core of 10 M_E, inside Wahl et al. 2017's 7 to
/// 25 M_E (study §5.3).
pub(super) const JUPITER: (f64, f64, f64, f64) = (318.0, 5.2, 4.5, 10.0);

/// Source: Neptune's mass, 17.1 M_E, at 30 au, with a 15 % envelope, the
/// Lopez & Fortney 2014 column whose radius the study compares (§4).
const NEPTUNE: (f64, f64, f64) = (17.1, 30.0, 0.15);

/// Source: the study's comparison for Neptune (§4): the Lopez & Fortney
/// 2014 radius lies 5 % below to 15 % above Neptune's.
const NEPTUNE_RADIUS_RANGE: (f64, f64) = (0.95, 1.15);

/// Definition: Review Focus 2's continuity at 1 M_E, where the envelope
/// table is read at its lowest mass: radii within 2 %.
const CONTINUITY: f64 = 0.02;

fn km(radius: science::units::EarthRadius) -> f64 {
    radius.0 * EARTH_RADIUS_KM
}

/// A Jupiter trial: 318 M_E at 5.2 au, 4.5 Gyr, heavy mass 10 M_E.
fn jupiter(trace: &mut Trace) -> (Trial, Drawn) {
    jupiter_at(trace, JUPITER.1)
}

/// Jupiter's mass, age and heavy mass at `au`.
pub(super) fn jupiter_at(trace: &mut Trace, au: f64) -> (Trial, Drawn) {
    let (mass, _, age, heavy) = JUPITER;
    let mut t = trial(trace, 1, mass, au, 0.0);
    t.age = leaf(trace, "test.age", Gyr(age));
    let drawn = Drawn::fixed(
        &t,
        trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
        None,
        1.0 - heavy / mass,
        Some(EarthMass(heavy)),
        None,
    );
    (t, drawn)
}

#[test]
fn a_jupiter_trial() {
    use science::giants::giant;
    let mut trace = Trace::new();
    let (t, drawn) = jupiter(&mut trace);
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let r = km(*body.bulk().radius.value());
    assert!((r / JUPITER_RADIUS_KM - 1.0).abs() < 0.03, "radius {r} km");
    let layers: Vec<_> = body
        .layers()
        .iter()
        .map(|l| (*l.material.value(), *l.state.value()))
        .collect();
    // The layer order of Jupiter's models: a heavy-element core, metallic
    // hydrogen below about 1 Mbar, molecular hydrogen above (study
    // `giants-and-envelopes.md` §5.3, Debras & Chabrier 2019,
    // arXiv:1901.05697).
    assert_eq!(layers.len(), 3, "{layers:?}");
    assert_eq!(layers[0].0, Material::RockIce);
    assert_eq!(layers[1], (Material::Envelope, MatterState::Liquid));
    assert_eq!(layers[2], (Material::Envelope, MatterState::Gas));
    let heat = heat::heat(
        &t,
        &body,
        &drawn.radiogenic_abundance,
        &drawn.envelope_fraction,
        &mut trace,
    )
    .unwrap();
    let (mass, au, age, heavy) = JUPITER;
    let table = giant(
        EarthMass(mass),
        EarthMass(heavy),
        Gyr(age),
        EarthFlux(1.0 / (au * au)),
    )
    .unwrap();
    // Wiring, not a published value: the secular heat is the tables'
    // internal luminosity, read once.
    assert_eq!(*heat.secular.value(), table.luminosity);
    assert_eq!(heat.radiogenic.value().0, 0.0);
}

#[test]
fn a_neptune_trial() {
    let (mass, au, envelope) = NEPTUNE;
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, mass, au, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        EARTH_WATER_FRACTION,
        0.0,
        None,
        envelope,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let ratio = km(*body.bulk().radius.value()) / NEPTUNE_RADIUS_KM;
    let (low, high) = NEPTUNE_RADIUS_RANGE;
    assert!(ratio >= low && ratio <= high, "radius ratio {ratio}");
    let Body::MiniNeptune { core, .. } = &body else {
        panic!("a mini-Neptune");
    };
    let layers = body.layers();
    // The core's shells, then one gas envelope.
    assert_eq!(layers.len(), core.shells.len() + 1);
    let last = layers.last().unwrap();
    assert_eq!(
        (*last.material.value(), *last.state.value()),
        (Material::Envelope, MatterState::Gas)
    );
    assert!(
        layers[..layers.len() - 1]
            .iter()
            .all(|l| *l.material.value() != Material::Envelope)
    );
    // The core's radius has its own code, the planet's is `planet.radius`.
    assert_eq!(
        trace.node(core.bulk.radius.node()).unwrap().code,
        Code::of(codes::RADIUS_WITHOUT_ENVELOPE)
    );
    assert_eq!(
        trace.node(body.bulk().radius.node()).unwrap().code,
        Code::of(codes::RADIUS)
    );
    // The core holds (1 - f) M.
    let core_mass = core.bulk.mass.value().0;
    assert!((core_mass / ((1.0 - envelope) * mass) - 1.0).abs() < 1e-12);
    heat::heat(
        &t,
        &body,
        &drawn.radiogenic_abundance,
        &drawn.envelope_fraction,
        &mut trace,
    )
    .unwrap();
}

#[test]
fn a_negative_envelope_thickness_is_an_error() {
    use super::enveloped::giant_envelope_thickness;
    use super::interior::InteriorError;
    use science::units::EarthRadius;
    // Definition: radii in Earth radii, a core equal to and above the
    // total, and below it.
    let core = EarthRadius(4.0);
    assert_eq!(
        giant_envelope_thickness(core, core),
        Err(InteriorError::NoEnvelope)
    );
    assert_eq!(
        giant_envelope_thickness(EarthRadius(3.0), core),
        Err(InteriorError::NoEnvelope)
    );
    assert_eq!(
        giant_envelope_thickness(EarthRadius(11.0), core),
        Ok(EarthRadius(7.0))
    );
}

#[test]
fn regime_edges_are_finite_and_1_earth_mass_is_continuous() {
    // Review Focus 2 and 3: each regime edge, drawn by the engine, gives a
    // finite radius and the model of its side.
    for mass in [0.099, 0.101, 0.99, 1.01, 19.9, 20.1] {
        for seed in 0..5 {
            let mut trace = Trace::new();
            let t = trial(&mut trace, seed, mass, 1.0, 0.0);
            let drawn = draws::draw(t.seed, &t, &mut trace).unwrap();
            let body = interior(&t, &drawn, &mut trace)
                .unwrap_or_else(|e| panic!("{mass} M_E, seed {seed}: {e}"));
            let r = body.bulk().radius.value().0;
            assert!(r.is_finite() && r > 0.0, "{mass} M_E: radius {r}");
            match (&body, mass) {
                (Body::Rocky(_), m) if m < 0.1 => {}
                (Body::MiniNeptune { .. }, m) if (0.1..20.0).contains(&m) => {}
                (Body::Giant(_), m) if m >= 20.0 => {}
                _ => panic!("{mass} M_E: wrong model"),
            }
        }
    }
    let radius = |mass: f64| {
        let mut trace = Trace::new();
        let t = trial(&mut trace, 1, mass, 1.0, 0.0);
        let drawn = Drawn::fixed(
            &t,
            &mut trace,
            EARTH_CORE_MASS_FRACTION,
            EARTH_WATER_FRACTION,
            0.0,
            Some(false),
            0.01,
            None,
            None,
        );
        interior(&t, &drawn, &mut trace)
            .unwrap()
            .bulk()
            .radius
            .value()
            .0
    };
    let (below, above) = (radius(0.99), radius(1.01));
    assert!(
        (above / below - 1.0).abs() < CONTINUITY,
        "{below} -> {above}"
    );
}

#[test]
fn the_flux_record_cites_every_source() {
    for giant in [false, true] {
        let mut trace = Trace::new();
        let (mut t, drawn) = if giant {
            jupiter(&mut trace)
        } else {
            let t = trial(&mut trace, 1, 5.0, 1.0, 0.0);
            let d = Drawn::fixed(
                &t,
                &mut trace,
                EARTH_CORE_MASS_FRACTION,
                EARTH_WATER_FRACTION,
                0.0,
                Some(false),
                0.01,
                None,
                None,
            );
            (t, d)
        };
        // A second star, a copy of the first with its own nodes.
        let mut stars: Vec<StellarSource> = t.sources.iter().cloned().collect();
        let mut second = stars[0].clone();
        second.flux = leaf(&mut trace, "test.flux", *second.flux.value());
        stars.push(second);
        t.sources = StellarSources::new(stars).unwrap();
        let body = interior(&t, &drawn, &mut trace).unwrap();
        // The node the envelope or the giant's radius reads the flux from.
        let read = match &body {
            Body::MiniNeptune { envelope, .. } => envelope.thickness.node(),
            Body::Giant(record) => record.bulk.radius.node(),
            Body::Rocky(_) => panic!("an enveloped planet"),
        };
        let flux = derived_from(&trace, read)
            .into_iter()
            .find(|&n| trace.node(n).unwrap().code == Code::of(codes::BOLOMETRIC_FLUX))
            .expect("a flux input");
        let every: Vec<_> = t.sources.iter().map(|s| s.flux.node()).collect();
        assert_eq!(derived_from(&trace, flux), every);
    }
}

#[test]
fn a_hot_mini_neptune_bulk_excludes_its_beyond_law_water() {
    // Task 6's rule: the bulk of a planet whose water is beyond the laws
    // describes the condensed planet. Definition: a 5 M_E water world with
    // a 1 % envelope at 403 K.
    let (mass, envelope) = (5.0, 0.01);
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, mass, HOT_ORBIT_AU, 0.0);
    let drawn = Drawn::fixed(
        &t,
        &mut trace,
        EARTH_CORE_MASS_FRACTION,
        WATER_WORLD_FRACTION,
        0.0,
        Some(true),
        envelope,
        None,
        None,
    );
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let Body::MiniNeptune {
        core,
        envelope: record,
        bulk,
    } = &body
    else {
        panic!("a mini-Neptune");
    };
    // The water, w (1 - f) M, is recorded apart.
    let Water::BeyondLaw { mass: water, .. } = core.water else {
        panic!("{:?}", core.water);
    };
    assert_eq!(
        trace.node(water.node()).unwrap().code,
        Code::of(codes::WATER_BEYOND_LAW)
    );
    let water_me = science::units::EarthMass::from(*water.value()).0;
    let expected_water = WATER_WORLD_FRACTION * (1.0 - envelope) * mass;
    assert!((water_me / expected_water - 1.0).abs() < 1e-9);
    // The bulk mass: the core's dry mass plus the envelope.
    let dry = (1.0 - WATER_WORLD_FRACTION) * (1.0 - envelope) * mass + envelope * mass;
    assert!((bulk.mass.value().0 / dry - 1.0).abs() < 1e-9);
    let node = trace.node(bulk.mass.node()).unwrap();
    assert_eq!(node.code, Code::of(codes::DRY_MASS));
    assert_eq!(
        derived_from(&trace, bulk.mass.node()),
        vec![core.bulk.mass.node(), record.mass.node()]
    );
    // Gravity and density from that mass and the total radius.
    let kg = science::units::Kilogram::from(*bulk.mass.value()).0;
    let r = science::units::Metre::from(*bulk.radius.value()).0;
    let g = science::constants::GRAVITATIONAL_CONSTANT * kg / (r * r);
    assert!((bulk.surface_gravity.value().0 / g - 1.0).abs() < 1e-9);
    let rho = kg / (4.0 / 3.0 * std::f64::consts::PI * r.powi(3));
    assert!((bulk.mean_density.value().0 / rho - 1.0).abs() < 1e-9);
}

#[test]
fn the_volatile_draw_leaves_the_other_draws_unchanged() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 7, 1.0, 1.0, 0.0);
    let base = draws::draw(t.seed, &t, &mut trace).unwrap();
    let other = Labels {
        volatiles: "planet/volatiles-other",
        ..LABELS
    };
    let changed = draws::draw_with(t.seed, &t, &mut trace, &other).unwrap();
    assert_eq!(
        base.core_mass_fraction.value(),
        changed.core_mass_fraction.value()
    );
    assert_eq!(base.water_fraction.value(), changed.water_fraction.value());
    assert_eq!(
        base.radiogenic_abundance.value(),
        changed.radiogenic_abundance.value()
    );
    assert_eq!(
        base.envelope_fraction.value(),
        changed.envelope_fraction.value()
    );
    assert_ne!(base.volatiles.co2.value(), changed.volatiles.co2.value());
    assert_ne!(base.volatiles.nh3.value(), changed.volatiles.nh3.value());
}

#[test]
fn the_volatile_scatter_has_its_spread() {
    /// Definition: seeds of the spread test. At 2 000 draws of a normal of
    /// sd 0.3 dex, the standard error of the mean is 0.3 / sqrt(2000) =
    /// 0.0067 dex, that of the sd about 0.3 / sqrt(2 x 1999) = 0.0047 dex.
    const SEEDS: u64 = 2000;
    /// Definition: tolerance of the mean and the standard deviation, 0.02
    /// dex: 3.0 standard errors for the mean, 4.2 for the sd.
    const TOLERANCE: f64 = 0.02;
    let mut columns = vec![Vec::new(); 5];
    for seed in 0..SEEDS {
        let scatter = draws::volatile_scatter(Seed::new(seed).derive(LABELS.volatiles));
        for (column, value) in columns.iter_mut().zip(scatter) {
            column.push(value);
        }
    }
    for column in columns {
        let n = column.len() as f64;
        let mean = column.iter().sum::<f64>() / n;
        let sd = (column.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        assert!(mean.abs() < TOLERANCE, "mean {mean}");
        assert!(
            (sd - science::volatiles::VOLATILE_SPREAD.0).abs() < TOLERANCE,
            "sd {sd}"
        );
    }
}

#[test]
fn the_volatile_scatter_is_recorded_as_drawn() {
    let mut trace = Trace::new();
    let t = trial(&mut trace, 5, 1.0, 1.0, 0.0);
    let d = draws::draw(t.seed, &t, &mut trace).unwrap();
    let v = d.volatiles;
    for (leaf, code) in [
        (v.co2, codes::VOLATILE_SCATTER_CO2),
        (v.n2, codes::VOLATILE_SCATTER_N2),
        (v.s, codes::VOLATILE_SCATTER_S),
        (v.ch4, codes::VOLATILE_SCATTER_CH4),
        (v.nh3, codes::VOLATILE_SCATTER_NH3),
    ] {
        let node = trace.node(leaf.node()).unwrap();
        assert_eq!(node.code.as_str(), code);
        assert_eq!(node.origin, Origin::Leaf(Provenance::Drawn));
    }
    let fixed = Drawn::fixed(
        &t,
        &mut trace,
        0.3,
        0.001,
        0.0,
        Some(false),
        0.0,
        None,
        None,
    );
    for (leaf, code) in [
        (fixed.volatiles.co2, codes::VOLATILE_SCATTER_CO2),
        (fixed.volatiles.nh3, codes::VOLATILE_SCATTER_NH3),
    ] {
        assert_eq!(leaf.value().0, 0.0);
        let node = trace.node(leaf.node()).unwrap();
        assert_eq!(node.code.as_str(), code);
        assert_eq!(node.origin, Origin::Leaf(Provenance::Imposed));
    }
    for leaf in [fixed.volatiles.n2, fixed.volatiles.s, fixed.volatiles.ch4] {
        assert_eq!(leaf.value().0, 0.0);
    }
}

/// A Solar System body of the legacy reference table
/// (`legacy-final:crates/mapgen/src/planet/reference.rs`), lit by the
/// trial's Sun-like source at its semi-major axis.
#[derive(Clone, Copy, Debug)]
pub(super) struct ReferenceBody {
    /// Mass, Earth masses.
    pub mass: f64,
    /// Iron share of the refractory (iron and rock) mass.
    pub core_mass_fraction: f64,
    /// Share of the mass in water.
    pub water_fraction: f64,
    /// Semi-major axis, au.
    pub orbit: f64,
    /// Orbital eccentricity.
    pub eccentricity: f64,
}

/// Source: NASA planetary fact sheet (nssdc.gsfc.nasa.gov/planetary/factsheet):
/// 0.0553 M_E at 0.387 au, eccentricity 0.2056; a core of about 70 % of the
/// mass (Hauck et al. 2013, doi:10.1002/jgre.20091, core radius about
/// 2020 km); dry, as legacy's reference table.
pub(super) const MERCURY: ReferenceBody = ReferenceBody {
    mass: 0.0553,
    core_mass_fraction: 0.70,
    water_fraction: 0.0,
    orbit: 0.387,
    eccentricity: 0.2056,
};

/// Source: NASA planetary fact sheet: 0.815 M_E at 0.723 au, eccentricity
/// 0.0068; Earth-like, slightly less iron, core 0.31 (Aitta 2012, Icarus
/// 218); dry, as legacy's reference table.
pub(super) const VENUS: ReferenceBody = ReferenceBody {
    mass: 0.815,
    core_mass_fraction: 0.31,
    water_fraction: 0.0,
    orbit: 0.723,
    eccentricity: 0.0068,
};

/// Source: NASA planetary fact sheet: 1 M_E at 1 au, eccentricity 0.0167;
/// core 0.325 (Zeng, Sasselov & Jacobsen 2016,
/// doi:10.3847/0004-637X/819/2/127); the bulk water of
/// [`EARTH_WATER_FRACTION`] (Marty 2012), of which the surface share
/// (`science::habitable::SURFACE_WATER_SHARE`) is its ocean.
pub(super) const EARTH: ReferenceBody = ReferenceBody {
    mass: 1.0,
    core_mass_fraction: 0.325,
    water_fraction: EARTH_WATER_FRACTION,
    orbit: 1.0,
    eccentricity: 0.0167,
};

/// Source: NASA planetary fact sheet: 0.107 M_E at 1.524 au, eccentricity
/// 0.0934; a core of about 24 % (Stähler et al. 2021,
/// doi:10.1126/science.abi7730, core radius about 1830 km); dry, as
/// legacy's reference table.
pub(super) const MARS: ReferenceBody = ReferenceBody {
    mass: 0.107,
    core_mass_fraction: 0.24,
    water_fraction: 0.0,
    orbit: 1.524,
    eccentricity: 0.0934,
};

/// Source: Titan, 0.0225 M_E (JPL satellite physical parameters, GM
/// 8978.1371 km^3/s^2, over the IAU nominal Earth GM 3.986004e14 m^3/s^2:
/// 0.022524) at Saturn's 9.57 au, eccentricity 0.052 (NASA planetary fact
/// sheet: 1432.0e6 km over 149.6e6, Saturn's orbit); 45 % water by mass
/// (Sohl et al. 2003, doi:10.1029/2003JE002044: 50 to 60 wt % rock and
/// iron, via Mosqueira 2026, arXiv:2610.34081, Table A1). Its core,
/// 0.20 of the rock, is not sourced: see the field. Study 23 of the Wiki.
pub(super) const TITAN: ReferenceBody = ReferenceBody {
    mass: 0.0225,
    // Definition: legacy's core fraction, kept by analogy as a test
    // input. The rock is chondritic (Kuskov & Kronrod, COSPAR 2006
    // abstract: L/LL chondrite, Fe/Si 0.9 to 1.3), but the Si content
    // that would turn that Fe/Si into an iron share was not read (study
    // 23, part 6).
    core_mass_fraction: 0.20,
    water_fraction: 0.45,
    orbit: 9.57,
    eccentricity: 0.052,
};

/// Source: Callisto, 0.0180 M_E (JPL satellite physical parameters, GM
/// 7179.2834 km^3/s^2, over the IAU nominal Earth GM: 0.018011) at
/// Jupiter's 5.20 au, eccentricity 0.049 (NASA planetary fact sheet:
/// 778.5e6 km over 149.6e6); 50 % water by mass (Kuskov & Kronrod, COSPAR
/// 2006 abstract: "total amount of H2O in Callisto is found to be 48-55
/// wt%"). Its core, 0.20 of the rock, is not sourced: see the field.
/// Study 23 of the Wiki.
pub(super) const CALLISTO: ReferenceBody = ReferenceBody {
    mass: 0.0180,
    // Definition: legacy's core fraction, as [`TITAN`]'s, kept by
    // analogy as a test input, not derived from a source.
    core_mass_fraction: 0.20,
    water_fraction: 0.50,
    orbit: 5.20,
    eccentricity: 0.049,
};

/// Definition: Venus given Earth's bulk water, so Earth's ocean share of
/// it, a runaway planet that has water to lose (the reference Venus is
/// dry; a test choice).
pub(super) const WET_VENUS: ReferenceBody = ReferenceBody {
    water_fraction: EARTH.water_fraction,
    ..VENUS
};

/// The body, heat and water of `t` of composition `drawn`, as the chain
/// derives them before the regime.
pub(super) fn derive(
    trace: &mut Trace,
    t: &Trial,
    drawn: &Drawn,
) -> (Body, timeline::Heat, timeline::Water) {
    let (body, h, w, _) = derive_surface(trace, t, drawn);
    (body, h, w)
}

/// [`derive`], with the surface water the water derives from.
pub(super) fn derive_surface(
    trace: &mut Trace,
    t: &Trial,
    drawn: &Drawn,
) -> (
    Body,
    timeline::Heat,
    timeline::Water,
    Traced<science::units::Kilogram>,
) {
    let body = interior(t, drawn, trace).unwrap();
    let h = heat::heat(
        t,
        &body,
        &drawn.radiogenic_abundance,
        &drawn.envelope_fraction,
        trace,
    )
    .unwrap();
    let surface = super::water::surface_water(t, drawn, &body, trace).unwrap();
    let w = super::water::water(t, &body, surface, trace).unwrap();
    (body, h, w, surface)
}

/// `t` with its source's XUV fluence replaced by `fluence`, J/m².
pub(super) fn with_fluence(trace: &mut Trace, t: &Trial, fluence: f64) -> Trial {
    let mut source = t.sources.primary().clone();
    source.xuv.fluence = trace
        .leaf(
            Code::of("test.xuv-fluence"),
            JoulePerSquareMetre(fluence),
            Provenance::Input,
        )
        .unwrap();
    Trial {
        sources: StellarSources::new(vec![source]).unwrap(),
        ..t.clone()
    }
}

/// The trial of `body` around the trial's Sun-like source, and its
/// composition set by hand: Earth's radiogenic abundance, no envelope,
/// not migrated when it formed inside the ice line, no volatile scatter.
pub(super) fn reference_trial(trace: &mut Trace, body: &ReferenceBody) -> (Trial, Drawn) {
    let t = trial(trace, 1, body.mass, body.orbit, body.eccentricity);
    let inside = t.orbit.semi_major_axis.value().0 < t.ice_line.value().0;
    let drawn = Drawn::fixed(
        &t,
        trace,
        body.core_mass_fraction,
        body.water_fraction,
        0.0,
        inside.then_some(false),
        0.0,
        None,
        None,
    );
    (t, drawn)
}
