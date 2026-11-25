//! The class: each rule of the table on a synthetic view, the Solar
//! System's bodies that settle cheaply, and the giants on their trial.
//! Ported from `legacy-final:crates/mapgen/src/planet/classify.rs`
//! (`mod tests`) and re-sourced; what is not ported is listed in the
//! report of the task. The classes that need a bistable solve are tier 3
//! (`climate::heavy_tests`), as are the two water worlds here over 5 s.

use super::classify::{BodyKind, ClassView, SurfaceView, class, classify};
use super::climate::fixture::{EARTH_OBLIQUITY, MARS_OBLIQUITY, rotation};
use super::codes;
use super::draws::{Drawn, LABELS};
use super::heat::ICY_MIN_WATER;
use super::settled::{Settled, settled_with};
use super::tests::{
    CALLISTO, EARTH_CORE_MASS_FRACTION, EARTH_WATER_FRACTION, HOT_ORBIT_AU, MARS, MERCURY,
    ReferenceBody, VENUS, WATER_WORLD_FRACTION, jupiter_at, neptune_at, reference_trial, trial,
    water_world_trial,
};
use crate::Trial;
use science::classes::{
    DENSE_AIR_BAR, HOT_GIANT_MAX_PERIOD, ICE_WORLD_MIN_OCEANS, SUPER_EARTH_MIN_RADIUS,
};
use science::units::{Bar, Day, Degree, EarthMass, EarthRadius};
use science::volatiles::methane_triple_pressure;
use timeline::{Code, LiquidKind, NodeValue, PlanetClass, Seed, TectonicRegime, Trace};

/// Definition: the least mass of an ice giant, 10 Earth masses
/// (`envelope::MINI_NEPTUNE_MAX_MASS`), written out for the boundary tests.
const TEN: f64 = 10.0;

/// Definition: a mass a hair under [`TEN`], 9.99 Earth masses.
const JUST_UNDER_TEN: f64 = 9.99;

/// Definition: a period past any hot giant's, 100 days.
const LONG_PERIOD: Day = Day(100.0);

/// Definition: a radius a hair under the super-Earth line, 1.2499 Earth
/// radii.
const JUST_UNDER_SUPER_EARTH: f64 = 1.2499;

/// Definition: a surface that matches no rule of the table: dry, no
/// runaway, thin air, a bulk without ice, plates, a small radius. Each
/// test changes the fields of its row.
fn plain() -> SurfaceView {
    SurfaceView {
        hides_surface: false,
        liquid: None,
        runaway: false,
        air: Bar(0.0),
        oceans: 0.0,
        capacity: 3.7,
        water_fraction: 0.0,
        regime: TectonicRegime::Plates,
        radius: EarthRadius(1.0),
    }
}

/// A rocky view on `surface`, a day's orbit away from nothing: a period
/// no rule of a rocky body reads.
fn rocky(surface: SurfaceView) -> ClassView {
    ClassView {
        kind: BodyKind::Rocky,
        mass: EarthMass(1.0),
        period: LONG_PERIOD,
        surface: Some(surface),
    }
}

/// An enveloped view of `kind`, `mass` Earth masses, with `period`.
fn enveloped(kind: BodyKind, mass: f64, period: Day) -> ClassView {
    ClassView {
        kind,
        mass: EarthMass(mass),
        period,
        surface: None,
    }
}

#[test]
fn each_rule_of_the_table() {
    use PlanetClass::*;
    let hot = HOT_GIANT_MAX_PERIOD;
    let later = Day(hot.0 + 1.0);
    // Giants: a hot Jupiter at the line, a gas giant one day more.
    assert_eq!(class(&enveloped(BodyKind::Giant, 318.0, hot)), HotJupiter);
    assert_eq!(class(&enveloped(BodyKind::Giant, 318.0, later)), GasGiant);
    // Ice giants, by the bulk's mass: exactly 10 is one, 9.99 is not,
    // whatever its period.
    assert_eq!(
        class(&enveloped(BodyKind::MiniNeptune, TEN, hot)),
        HotNeptune
    );
    assert_eq!(
        class(&enveloped(BodyKind::MiniNeptune, TEN, later)),
        IceGiant
    );
    assert_eq!(
        class(&enveloped(BodyKind::MiniNeptune, JUST_UNDER_TEN, hot)),
        MiniNeptune
    );
    assert_eq!(
        class(&enveloped(BodyKind::MiniNeptune, JUST_UNDER_TEN, later)),
        MiniNeptune
    );
    // The steam world beats lava and a runaway over 1 bar.
    let steam = SurfaceView {
        hides_surface: true,
        liquid: Some(LiquidKind::Lava),
        runaway: true,
        air: Bar(1.0e5),
        ..plain()
    };
    assert_eq!(class(&rocky(steam)), SteamWorld);
    // Lava beats a runaway.
    let lava = SurfaceView {
        hides_surface: false,
        ..steam
    };
    assert_eq!(class(&rocky(lava)), LavaWorld);
    // A runaway needs more than 1 bar: exactly 1 bar is a bare rock.
    let runaway = SurfaceView {
        liquid: None,
        air: Bar(DENSE_AIR_BAR.0 + 1.0),
        ..lava
    };
    assert_eq!(class(&rocky(runaway)), RunawayGreenhouse);
    let thin = SurfaceView {
        air: DENSE_AIR_BAR,
        ..runaway
    };
    assert_eq!(class(&rocky(thin)), Rocky);
    // Hydrocarbons over methane's triple point, not at it.
    let seas = SurfaceView {
        liquid: Some(LiquidKind::Hydrocarbons),
        air: Bar(methane_triple_pressure().0 * 2.0),
        ..plain()
    };
    assert_eq!(class(&rocky(seas)), HydrocarbonWorld);
    let thin_seas = SurfaceView {
        air: methane_triple_pressure(),
        ..seas
    };
    assert_eq!(class(&rocky(thin_seas)), Rocky);
    // An ocean world: water over the basins' capacity, not at it; the
    // heat-pipe regime does not change it.
    let ocean = SurfaceView {
        liquid: Some(LiquidKind::Water),
        oceans: 3.7 + 1.0,
        regime: TectonicRegime::HeatPipe,
        ..plain()
    };
    assert_eq!(class(&rocky(ocean)), OceanWorld);
    let full = SurfaceView {
        oceans: ocean.capacity,
        ..ocean
    };
    assert_eq!(class(&rocky(full)), VolcanicWorld);
    // A heat-pipe ice world stays an ice world; with no ice it is a
    // volcanic one.
    let icy = SurfaceView {
        water_fraction: ICY_MIN_WATER,
        oceans: ICE_WORLD_MIN_OCEANS,
        regime: TectonicRegime::HeatPipe,
        ..plain()
    };
    assert_eq!(class(&rocky(icy)), IceWorld);
    let dry = SurfaceView {
        water_fraction: 0.0,
        ..icy
    };
    assert_eq!(class(&rocky(dry)), VolcanicWorld);
    // The radius line: at it a super-Earth, a hair under rocky.
    let big = SurfaceView {
        radius: SUPER_EARTH_MIN_RADIUS,
        ..plain()
    };
    assert_eq!(class(&rocky(big)), SuperEarth);
    let small = SurfaceView {
        radius: EarthRadius(JUST_UNDER_SUPER_EARTH),
        ..plain()
    };
    assert_eq!(class(&rocky(small)), Rocky);
    // An icy body with just under the surface water that makes an ice
    // world has lost its ice to its star (#452).
    let lost = SurfaceView {
        water_fraction: ICY_MIN_WATER,
        oceans: ICE_WORLD_MIN_OCEANS * 0.99,
        ..plain()
    };
    assert_eq!(class(&rocky(lost)), Rocky);
}

/// The class of the trial `t` of composition `d`, settled with a
/// rotation at `obliquity`.
fn class_of(trace: &mut Trace, t: &Trial, d: &Drawn, obliquity: Degree) -> (PlanetClass, Settled) {
    let rot = rotation(trace, obliquity, false);
    let snowball = Seed::new(1).derive(LABELS.snowball);
    let settled = settled_with(t, d, &rot, snowball, trace).unwrap();
    let class = classify(t, &settled, trace).unwrap();
    (*class.value(), settled)
}

/// The class of the reference `body`.
fn reference_class(body: &ReferenceBody, obliquity: Degree) -> (PlanetClass, Settled, Trace) {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, body);
    let (class, settled) = class_of(&mut trace, &t, &d, obliquity);
    (class, settled, trace)
}

/// Definition: an obliquity for a body whose ice caps the class does not
/// read, 0 degrees.
const NO_OBLIQUITY: Degree = Degree(0.0);

#[test]
fn the_solar_system_classifies_as_itself() {
    use PlanetClass::*;
    // Venus ran away by the insolation limit, not by boiling.
    let (venus, settled, _) = reference_class(&VENUS, NO_OBLIQUITY);
    assert_eq!(venus, RunawayGreenhouse);
    let water = &settled.bare.surface.as_ref().unwrap().water;
    assert!(
        *water.runaway.value() && !*water.boiled.value(),
        "{water:?}"
    );
    assert_eq!(reference_class(&MERCURY, NO_OBLIQUITY).0, Rocky);
    assert_eq!(reference_class(&MARS, MARS_OBLIQUITY).0, Rocky);
    // Photolysis leaves Callisto no methane, so no seas of it.
    let (callisto, settled, _) = reference_class(&CALLISTO, EARTH_OBLIQUITY);
    assert_eq!(callisto, IceWorld);
    let climate = settled.climate.as_ref().unwrap();
    assert!(climate.liquid.is_none(), "{climate:?}");
    assert_eq!(
        climate.atmosphere.partial[&timeline::Gas::Ch4].value().0,
        0.0
    );
    // The giants, on their trial.
    let mut trace = Trace::new();
    let (t, d) = jupiter_at(&mut trace, 5.2);
    assert_eq!(class_of(&mut trace, &t, &d, NO_OBLIQUITY).0, GasGiant);
    let (t, d) = neptune_at(&mut trace, 30.0);
    assert_eq!(class_of(&mut trace, &t, &d, NO_OBLIQUITY).0, IceGiant);
}

#[test]
fn the_class_records_the_nodes_its_rules_read() {
    let mut trace = Trace::new();
    let (t, d) = neptune_at(&mut trace, 30.0);
    let (class, _) = class_of(&mut trace, &t, &d, NO_OBLIQUITY);
    assert_eq!(class, PlanetClass::IceGiant);
    let nodes = super::settled_tests::nodes_of(&trace, codes::CLASS);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].1, NodeValue::Code(Code::of("ice-giant")));
    let from = super::tests::derived_from(&trace, nodes[0].0);
    assert_eq!(from, [t.mass.node(), t.orbit.period.node()]);
}

#[test]
fn a_close_gas_giant_is_a_hot_jupiter() {
    let mut trace = Trace::new();
    let (t, d) = jupiter_at(&mut trace, 0.05);
    assert!(t.orbit.period.value().0 <= HOT_GIANT_MAX_PERIOD.0);
    assert_eq!(
        class_of(&mut trace, &t, &d, NO_OBLIQUITY).0,
        PlanetClass::HotJupiter
    );
    let (t, d) = jupiter_at(&mut trace, 0.2);
    assert!(t.orbit.period.value().0 > HOT_GIANT_MAX_PERIOD.0);
    assert_eq!(
        class_of(&mut trace, &t, &d, NO_OBLIQUITY).0,
        PlanetClass::GasGiant
    );
}

#[test]
fn a_close_ice_giant_is_a_hot_neptune() {
    use super::enveloped::Body;
    let mut trace = Trace::new();
    // 0.08 au: a period of 8.3 days in the test's Kepler law.
    let (t, d) = neptune_at(&mut trace, 0.08);
    assert!(t.orbit.period.value().0 <= HOT_GIANT_MAX_PERIOD.0);
    let (class, settled) = class_of(&mut trace, &t, &d, NO_OBLIQUITY);
    assert!(matches!(settled.bare.body, Body::MiniNeptune { .. }));
    assert_eq!(class, PlanetClass::HotNeptune);
    let (t, d) = neptune_at(&mut trace, 30.0);
    assert_eq!(
        class_of(&mut trace, &t, &d, NO_OBLIQUITY).0,
        PlanetClass::IceGiant
    );
}

/// Definition: a water world drawn above the ice giants' line, 12 Earth
/// masses, with a tenth of it in gas (inside the ice giants' envelope
/// draw, 0.09 to 0.20 of the mass).
const BEYOND_LAW_GIANT: (f64, f64) = (12.0, 0.1);

#[test]
fn a_giant_whose_water_is_beyond_the_laws_stays_a_giant() {
    use super::enveloped::Body;
    // Task 6b: the 10 M_E line splits the envelope draw's regimes, which
    // read the trial's mass; the class reads the same mass, not the
    // condensed bulk (here (1 - 0.5) 0.9 12 + 1.2 = 6.6 M_E).
    let (mass, envelope) = BEYOND_LAW_GIANT;
    let mut trace = Trace::new();
    let t = trial(&mut trace, 1, mass, HOT_ORBIT_AU, 0.0);
    let d = Drawn::fixed(
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
    let (class, settled) = class_of(&mut trace, &t, &d, NO_OBLIQUITY);
    let Body::MiniNeptune { bulk, .. } = &settled.bare.body else {
        panic!("{:?}", settled.bare.body);
    };
    assert!(bulk.mass.value().0 < TEN, "{:?}", bulk.mass);
    assert!(t.orbit.period.value().0 > HOT_GIANT_MAX_PERIOD.0);
    assert_eq!(class, PlanetClass::IceGiant);
    let nodes = super::settled_tests::nodes_of(&trace, codes::CLASS);
    let from = super::tests::derived_from(&trace, nodes[0].0);
    assert_eq!(from, [t.mass.node(), t.orbit.period.node()]);
}

/// A planet of `mass` Earth masses with `envelope` of it in gas, Earth's
/// core and a `water` share, at `au`.
fn with_envelope(
    trace: &mut Trace,
    mass: f64,
    envelope: f64,
    water: f64,
    au: f64,
) -> (Trial, Drawn) {
    let t = trial(trace, 1, mass, au, 0.0);
    let d = Drawn::fixed(
        &t,
        trace,
        EARTH_CORE_MASS_FRACTION,
        water,
        0.0,
        None,
        envelope,
        None,
        None,
    );
    (t, d)
}

#[test]
fn an_enveloped_planet_is_a_mini_neptune() {
    let mut trace = Trace::new();
    let (t, d) = with_envelope(&mut trace, 5.0, 0.02, EARTH_WATER_FRACTION, 0.5);
    let (class, settled) = class_of(&mut trace, &t, &d, NO_OBLIQUITY);
    assert_eq!(class, PlanetClass::MiniNeptune);
    assert!(settled.climate.is_none());
    // With no envelope, and a dry surface out of the runaway, it is a
    // rock, by its radius.
    let (t, d) = with_envelope(&mut trace, 5.0, 0.0, 0.0, 1.5);
    let (class, settled) = class_of(&mut trace, &t, &d, NO_OBLIQUITY);
    let radius = settled.bare.body.bulk().radius.value().0;
    // The radius of a 5 M_E rock is well over the line.
    assert!(radius >= SUPER_EARTH_MIN_RADIUS.0, "{radius}");
    assert_eq!(class, PlanetClass::SuperEarth);
}

#[test]
#[ignore = "heavy: a water world settled, 7.6 s at opt-level 0"]
fn a_planet_that_lost_its_water_is_not_an_ice_world() {
    // The case of #452 (0.22 M_E, 37 % water, hot): at 0.6 au it keeps
    // under 0.01 ocean and gives no runaway class, so only its bulk made
    // it an ice world.
    let mut trace = Trace::new();
    let (t, d) = water_world_trial(&mut trace, 0.22, EARTH_CORE_MASS_FRACTION, 0.37, 0.6);
    let (class, settled) = class_of(&mut trace, &t, &d, EARTH_OBLIQUITY);
    let water = &settled.bare.surface.as_ref().unwrap().water;
    let oceans = water.inventory.value().0 / science::habitable::OCEAN_MASS.0;
    assert!(oceans < ICE_WORLD_MIN_OCEANS, "{oceans}");
    assert_ne!(class, PlanetClass::IceWorld);
}

#[test]
#[ignore = "heavy: a water world settled, 6.3 s at opt-level 0"]
fn a_cold_icy_planet_stays_an_ice_world() {
    let mut trace = Trace::new();
    let (t, d) = water_world_trial(&mut trace, 0.5, EARTH_CORE_MASS_FRACTION, 0.3, 4.0);
    assert_eq!(
        class_of(&mut trace, &t, &d, EARTH_OBLIQUITY).0,
        PlanetClass::IceWorld
    );
}
