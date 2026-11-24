//! Tier 1: the water a planet keeps, the steam a runaway loses, the planet
//! left after it and the tectonic regime, ported from
//! `legacy-final:crates/mapgen/src/planet/regime.rs` (its tests) on the
//! reference bodies of `planet/tests.rs`.

use super::codes;
use super::draws::Drawn;
use super::enveloped::{Body, Exposed};
use super::interior::{Water as Held, interior};
use super::regime::regime;
use super::tests::{
    EARTH, EARTH_CORE_MASS_FRACTION, LARGER_RADIUS, MARS, MERCURY, ReferenceBody, VENUS,
    WATER_WORLD_FRACTION, WET_VENUS, derive, derive_surface, derived_from, larger_exposed,
    reference_trial, trial, with_fluence,
};
use super::water::{after_steam, boiled, recondensed, surface_water, water};
use crate::Trial;
use science::habitable::{SURFACE_WATER_SHARE, runaway_flux};
use science::units::{EarthFlux, Kelvin, Kilogram, Watt};
use timeline::{Code, Heat, Provenance, StellarSources, TectonicRegime, Trace, Traced, Water};

/// Source: Io in legacy's reference table: 0.0150 M_E (JPL satellite
/// physical parameters, GM over Earth's), an Fe-FeS core up to 20 % of the
/// mass (Anderson et al. 1996-2001, Galileo gravity), dry, at Jupiter's
/// 5.2 au on a circular orbit around the Sun: its tide is set by hand
/// below.
const IO: ReferenceBody = ReferenceBody {
    mass: 0.0150,
    core_mass_fraction: 0.20,
    water_fraction: 0.0,
    orbit: 5.2,
    eccentricity: 0.0,
};

/// Source: Io's tidal heating in legacy's test, 1e14 W, about 2.4 W/m²
/// over Io's 4.17e13 m² (Moore & Webb 2013 take about 2.5 W/m² from
/// Veeder et al. 2004).
const IO_TIDE: f64 = 1.0e14;

/// Source: the low end of Lainey et al. 2009 in legacy's test, 7.5e13 W,
/// 1.79 W/m² over Io's 4.17e13 m².
const IO_TIDE_LOW: f64 = 7.5e13;

/// Source: the dry super-Earth of legacy's test, 5 M_E at 1.5 au with
/// Earth's core and no water.
const DRY_SUPER_EARTH: ReferenceBody = ReferenceBody {
    mass: 5.0,
    core_mass_fraction: EARTH_CORE_MASS_FRACTION,
    water_fraction: 0.0,
    orbit: 1.5,
    eccentricity: 0.0,
};

/// Definition: two orbits of a runaway water world around the trial's
/// Sun-like source, 0.4 au (403 K, past the 355 K where the water laws
/// end) and 0.8 au (285 K, within them), both above the runaway limit.
const HOT_ORBITS_AU: [f64; 2] = [0.4, 0.8];

/// Definition: a relative step either side of a threshold, 1 %.
const STEP: f64 = 0.01;

/// Source: the coolest and hottest stars of the two-star case, 3000 K
/// and 7000 K, inside Kopparapu et al. 2014's 2600 to 7200 K.
const TWO_STARS_K: [f64; 2] = [3000.0, 7000.0];

/// Source: the Sun's effective temperature, 5772 K (IAU 2015 Resolution
/// B3), the trial's source.
const SUN_TEFF: f64 = 5772.0;

/// Definition: a numerical tolerance on a mass computed two ways, relative.
const ROUNDING: f64 = 1e-12;

/// The water and regime of the reference `body`.
fn reference(body: &ReferenceBody) -> (Water, TectonicRegime) {
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, body);
    let (b, h, w) = derive(&mut trace, &t, &drawn);
    let r = regime(&t, &b, &h, &w, &mut trace).unwrap();
    (w, *r.value())
}

/// `t` lit by sources of the fluxes and effective temperatures `lights`
/// instead of its own, each a copy of its source otherwise.
fn lit(trace: &mut Trace, t: &Trial, lights: &[(f64, f64)]) -> Trial {
    let sources = lights
        .iter()
        .map(|&(flux, teff)| {
            let mut source = t.sources.primary().clone();
            source.flux = trace
                .leaf(Code::of("test.flux"), EarthFlux(flux), Provenance::Input)
                .unwrap();
            source.effective_temperature = trace
                .leaf(Code::of("test.teff"), Kelvin(teff), Provenance::Input)
                .unwrap();
            source
        })
        .collect();
    Trial {
        sources: StellarSources::new(sources).unwrap(),
        ..t.clone()
    }
}

/// Whether Earth, lit by `lights`, runs away.
fn runs_away(lights: &[(f64, f64)]) -> bool {
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &EARTH);
    let t = lit(&mut trace, &t, lights);
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let surface = surface_water(&t, &drawn, &body, &mut trace).unwrap();
    let w = water(&t, &body.exposed(), surface, &mut trace).unwrap();
    *w.runaway.value()
}

#[test]
fn venus_ran_away_and_dried() {
    let (w, _) = reference(&VENUS);
    assert!(*w.runaway.value());
    assert_eq!(w.inventory.value().0, 0.0);
    // Given an Earth ocean, it would have lost it too.
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &WET_VENUS);
    let (_, _, w, surface) = derive_surface(&mut trace, &t, &drawn);
    assert!(*w.runaway.value());
    assert_eq!(w.inventory.value().0, 0.0);
    // The inventory derives from the surface water and the steam lost,
    // which derives from the runaway, the radius, the mass, the XUV light
    // and the surface water: the steam is the ocean, not the mantle's water.
    let from = derived_from(&trace, w.inventory.node());
    assert_eq!(from[0], surface.node());
    let lost = *from.last().unwrap();
    assert_eq!(
        trace.node(lost).unwrap().code,
        Code::of(codes::WATER_STEAM_LOST)
    );
    let lost_from = derived_from(&trace, lost);
    assert_eq!(lost_from[0], w.runaway.node());
    assert!(lost_from.contains(&t.sources.primary().xuv.fluence.node()));
    assert!(lost_from.contains(&surface.node()));
    assert!(!*w.boiled.value());
}

#[test]
fn earth_keeps_its_ocean() {
    // Earth drawn at its bulk water (Marty 2012) keeps its ocean, not its
    // ten oceans of bulk water (the ocean's mass is
    // `science::habitable`'s test).
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &EARTH);
    let (_, _, w, surface) = derive_surface(&mut trace, &t, &drawn);
    assert!(!*w.runaway.value());
    // The ocean is the surface water, the share of the bulk water derived
    // from the mass and the two fractions; the rest stays in the mantle.
    assert_eq!(w.inventory.value(), surface.value());
    assert_eq!(
        derived_from(&trace, w.inventory.node()),
        vec![surface.node(), w.runaway.node()]
    );
    assert_eq!(
        trace.node(surface.node()).unwrap().code,
        Code::of(codes::WATER_SURFACE)
    );
    assert_eq!(
        derived_from(&trace, surface.node()),
        vec![
            t.mass.node(),
            drawn.water_fraction.node(),
            drawn.envelope_fraction.node()
        ]
    );
    let bulk = EARTH.water_fraction * Kilogram::from(*t.mass.value()).0;
    assert!((surface.value().0 / (SURFACE_WATER_SHARE * bulk) - 1.0).abs() < ROUNDING);
}

#[test]
fn a_dry_runaway_keeps_its_mantles_water() {
    // Venus given Earth's bulk water loses its ocean, the surface share,
    // to the steam; the mantle keeps the rest, and the planet left weighs
    // it.
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &WET_VENUS);
    let (_, _, w, surface) = derive_surface(&mut trace, &t, &drawn);
    assert!(*w.runaway.value());
    let (t2, d2) = after_steam(&t, &drawn, surface, &w, &mut trace)
        .unwrap()
        .expect("the ocean was lost");
    let total = Kilogram::from(*t.mass.value()).0;
    let bulk = WET_VENUS.water_fraction * total;
    let lost = surface.value().0 - w.inventory.value().0;
    assert!(lost > 0.0);
    // The mass after the steam is the mass less the surface water lost.
    let mass = Kilogram::from(*t2.mass.value()).0;
    assert!((mass / (total - lost) - 1.0).abs() < ROUNDING, "{mass}");
    // The water left is the bulk less what was lost: the mantle's
    // `(1 - s)` of the bulk, plus what the surface kept.
    let left = d2.water_fraction.value().get() * mass;
    let mantle = (1.0 - SURFACE_WATER_SHARE) * bulk;
    assert!(
        (left / (mantle + w.inventory.value().0) - 1.0).abs() < ROUNDING,
        "{left} {mantle}"
    );
}

#[test]
fn a_water_worlds_whole_water_is_at_its_surface() {
    // Its water layer is its ocean: the surface share is a dry planet's.
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
    let (body, _, w, surface) = derive_surface(&mut trace, &t, &drawn);
    let Body::Rocky(interior) = &body else {
        panic!("a water world without an envelope is rocky");
    };
    let Held::Layer { mass, .. } = interior.water else {
        panic!("a water world at 1 au has a water layer");
    };
    assert_eq!(surface, mass);
    assert!(!*w.runaway.value());
    assert_eq!(w.inventory.value(), mass.value());
    let bulk = WATER_WORLD_FRACTION * Kilogram::from(*t.mass.value()).0;
    assert!((w.inventory.value().0 / bulk - 1.0).abs() < ROUNDING);
}

#[test]
fn the_solar_system_has_its_regimes() {
    assert_eq!(reference(&EARTH).1, TectonicRegime::Plates);
    assert_eq!(reference(&VENUS).1, TectonicRegime::Episodic);
    assert_eq!(reference(&MARS).1, TectonicRegime::Stagnant);
    assert_eq!(reference(&MERCURY).1, TectonicRegime::Stagnant);
    // Io: an Io-mass rocky body with its tide set by hand.
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &IO);
    let (body, h, w) = derive(&mut trace, &t, &drawn);
    for tide in [IO_TIDE, IO_TIDE_LOW] {
        let tidal = trace
            .leaf(Code::of("test.tide"), Watt(tide), Provenance::Input)
            .unwrap();
        let h = Heat { tidal, ..h.clone() };
        let r = regime(&t, &body, &h, &w, &mut trace).unwrap();
        assert_eq!(*r.value(), TectonicRegime::HeatPipe, "{tide} W");
        // A heat pipe reads the heat and the radius only.
        assert_eq!(
            derived_from(&trace, r.node()),
            vec![
                h.radiogenic.node(),
                tidal.node(),
                h.secular.node(),
                body.bulk().radius.node()
            ]
        );
    }
    // Plates read the heat, the radius, the mass and the water.
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &EARTH);
    let (body, h, w) = derive(&mut trace, &t, &drawn);
    let r = regime(&t, &body, &h, &w, &mut trace).unwrap();
    assert_eq!(trace.node(r.node()).unwrap().code, Code::of(codes::REGIME));
    assert_eq!(
        derived_from(&trace, r.node())[4..],
        [t.mass.node(), w.inventory.node(), w.runaway.node()]
    );
}

#[test]
fn a_dry_super_earth_is_not_plated() {
    let (w, r) = reference(&DRY_SUPER_EARTH);
    assert!(!*w.runaway.value());
    assert_ne!(r, TectonicRegime::Plates);
}

#[test]
fn a_hot_water_world_runs_away_and_rebuilds() {
    for au in HOT_ORBITS_AU {
        let mut trace = Trace::new();
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
        let (_, _, w, surface) = derive_surface(&mut trace, &t, &drawn);
        assert!(*w.runaway.value(), "{au} au");
        let total = Kilogram::from(*t.mass.value()).0;
        let bulk = WATER_WORLD_FRACTION * total;
        let kept = w.inventory.value().0;
        assert!(kept > 0.0 && kept < bulk, "{au} au: {kept} of {bulk}");

        let (t2, d2) = after_steam(&t, &drawn, surface, &w, &mut trace)
            .unwrap()
            .expect("some steam was lost");
        // The mass is the dry mass plus the water kept, and the water
        // fraction what was kept of it.
        let mass = Kilogram::from(*t2.mass.value()).0;
        assert!((mass / (total - bulk + kept) - 1.0).abs() < ROUNDING);
        let fraction = d2.water_fraction.value().get();
        assert!((fraction / (kept / mass) - 1.0).abs() < ROUNDING);
        assert_eq!(
            derived_from(&trace, t2.mass.node()),
            vec![t.mass.node(), surface.node(), w.inventory.node()]
        );
        assert_eq!(
            derived_from(&trace, d2.water_fraction.node()),
            vec![
                t.mass.node(),
                drawn.water_fraction.node(),
                surface.node(),
                w.inventory.node(),
                t2.mass.node()
            ]
        );
        assert_eq!(
            trace.node(d2.water_fraction.node()).unwrap().code,
            Code::of(codes::WATER_KEPT_FRACTION)
        );
        // The rest of the composition is kept.
        assert_eq!(d2.core_mass_fraction, drawn.core_mass_fraction);
        assert_eq!(d2.volatiles, drawn.volatiles);

        // The body is built again, on either side of the water laws.
        let (body, h, w2) = derive(&mut trace, &t2, &d2);
        let Body::Rocky(rebuilt) = &body else {
            panic!("{au} au: not rocky");
        };
        let beyond = matches!(rebuilt.water, Held::BeyondLaw { .. });
        assert_eq!(
            beyond,
            au == HOT_ORBITS_AU[0],
            "{au} au: {:?}",
            rebuilt.water
        );
        regime(&t2, &body, &h, &w2, &mut trace).unwrap();
    }
}

#[test]
fn nothing_lost_rebuilds_nothing() {
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &EARTH);
    let (_, _, w, surface) = derive_surface(&mut trace, &t, &drawn);
    assert_eq!(
        after_steam(&t, &drawn, surface, &w, &mut trace).unwrap(),
        None
    );
}

#[test]
fn the_runaway_threshold_reads_every_source() {
    // One source just above and just below its limit, against two
    // identical sources of half its flux each.
    let limit = runaway_flux(Kelvin(SUN_TEFF)).0;
    for flux in [limit * (1.0 + STEP), limit * (1.0 - STEP)] {
        let one = runs_away(&[(flux, SUN_TEFF)]);
        let two = runs_away(&[(flux / 2.0, SUN_TEFF), (flux / 2.0, SUN_TEFF)]);
        assert_eq!(one, flux > limit);
        assert_eq!(two, one, "{flux}");
    }

    // A cool and a hot star at the same flux each: the sum of each flux
    // over its own limit, not the limit at their flux-weighted temperature.
    let [cool, hot] = TWO_STARS_K;
    let inverse = 1.0 / runaway_flux(Kelvin(cool)).0 + 1.0 / runaway_flux(Kelvin(hot)).0;
    let by_sum = 2.0 / inverse;
    let blended = runaway_flux(Kelvin((cool + hot) / 2.0)).0;
    assert!(
        (by_sum / blended - 1.0).abs() > 2.0 * STEP,
        "{by_sum} {blended}"
    );
    let total = (by_sum + blended) / 2.0;
    assert_eq!(
        runs_away(&[(total / 2.0, cool), (total / 2.0, hot)]),
        total > by_sum
    );

    // The runaway derives from every source's flux and temperature.
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &EARTH);
    let t = lit(&mut trace, &t, &[(total / 2.0, cool), (total / 2.0, hot)]);
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let surface = surface_water(&t, &drawn, &body, &mut trace).unwrap();
    let w = water(&t, &body.exposed(), surface, &mut trace).unwrap();
    let light: Vec<_> = t
        .sources
        .iter()
        .flat_map(|s| [s.flux.node(), s.effective_temperature.node()])
        .collect();
    assert_eq!(derived_from(&trace, w.runaway.node()), light);
    assert_eq!(
        trace.node(w.runaway.node()).unwrap().code,
        Code::of(codes::WATER_RUNAWAY)
    );
}

/// Definition: the XUV fluence of the steam tests, J/m², far under
/// `WET_VENUS`'s own: at its own fluence its 9 H exceeds its whole ocean
/// and both losses are capped at the surface water, which would hide
/// their dependence on the radius.
const LOW_FLUENCE: f64 = 1.0e13;

/// `WET_VENUS` under [`LOW_FLUENCE`], its body and surface water, and the
/// radius `LARGER_RADIUS` times larger.
fn wet_venus_low() -> (Trace, Trial, Body, Traced<Kilogram>, Exposed, Exposed) {
    let mut trace = Trace::new();
    let (t, drawn) = reference_trial(&mut trace, &WET_VENUS);
    let t = with_fluence(&mut trace, &t, LOW_FLUENCE);
    let body = interior(&t, &drawn, &mut trace).unwrap();
    let surface = surface_water(&t, &drawn, &body, &mut trace).unwrap();
    let exposed = body.exposed();
    let larger = larger_exposed(&mut trace, &exposed);
    (trace, t, body, surface, exposed, larger)
}

/// The steam lost by `w`, the last node its inventory derives from.
fn lost_by(trace: &Trace, w: &Water) -> f64 {
    let id = *derived_from(trace, w.inventory.node()).last().unwrap();
    let node = trace.node(id).unwrap();
    assert_eq!(node.code, Code::of(codes::WATER_STEAM_LOST));
    match &node.value {
        timeline::NodeValue::Number { value, .. } => *value,
        value => panic!("{value:?} is not a number"),
    }
}

#[test]
fn a_boil_forces_the_steam() {
    let (mut trace, t, _, surface, exposed, _) = wet_venus_low();
    let by_light = water(&t, &exposed, surface, &mut trace).unwrap();
    assert!(*by_light.runaway.value());
    let lost = lost_by(&trace, &by_light);
    assert!(0.0 < lost && lost < surface.value().0, "precondition");
    let verdict = trace
        .leaf(Code::of("test.verdict"), true, Provenance::Input)
        .unwrap();
    let w = boiled(&t, &exposed, surface, verdict, &mut trace).unwrap();
    assert!(*w.runaway.value() && *w.boiled.value());
    assert_eq!(derived_from(&trace, w.runaway.node()), [verdict.node()]);
    assert_eq!(derived_from(&trace, w.boiled.node()), [verdict.node()]);
    assert_eq!(
        trace.node(w.boiled.node()).unwrap().code,
        Code::of(codes::WATER_BOILED)
    );
    // One code path: the forced steam loses what the light's does.
    assert_eq!(lost_by(&trace, &w), lost);
}

#[test]
fn a_larger_radius_loses_more_steam() {
    let (mut trace, t, _, surface, exposed, larger) = wet_venus_low();
    let small = water(&t, &exposed, surface, &mut trace).unwrap();
    let big = water(&t, &larger, surface, &mut trace).unwrap();
    let (a, b) = (lost_by(&trace, &small), lost_by(&trace, &big));
    assert!(
        b < surface.value().0,
        "precondition: the larger is uncapped"
    );
    // Owen & Wu 2017 eq. 19: the energy-limited loss grows as R^3 / M.
    assert!((b / a - LARGER_RADIUS.powi(3)).abs() < 1e-9, "{}", b / a);
}

#[test]
fn a_leftover_condenses_back() {
    let (mut trace, t, _, surface, exposed, _) = wet_venus_low();
    let verdict = trace
        .leaf(Code::of("test.verdict"), true, Provenance::Input)
        .unwrap();
    let steam = boiled(&t, &exposed, surface, verdict, &mut trace).unwrap();
    let w = recondensed(&steam, &mut trace).unwrap();
    assert!(!*w.runaway.value() && *w.boiled.value());
    assert_eq!(w.inventory, steam.inventory);
    assert_eq!(w.boiled, steam.boiled);
    assert_eq!(
        derived_from(&trace, w.runaway.node()),
        [steam.inventory.node(), steam.runaway.node()]
    );
}
