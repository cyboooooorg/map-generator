//! The chain step `settled`: the climate of the reference bodies against
//! their measured values, and the causes it records.
//!
//! Tier 1 holds the cheap bodies (Venus, Mercury, Mars, Titan on its
//! measured air, a Triton-like body) and the checks that read no
//! bistable solve. The bistable Earth takes about a minute at opt-level 0
//! (the snowball branches), so its tests are tier 3.

use super::bare::{Bare, bare_with};
use super::climate::Rotation;
use super::climate::branches::{Bistable, Solved, solve};
use super::climate::fixture::{EARTH_OBLIQUITY, MARS_OBLIQUITY, TITAN_OBLIQUITY, rotation};
use super::climate::record::{Climate, record};
use super::climate::{ClimateInputs, Numerics};
use super::codes;
use super::draws::LABELS;
use super::draws::draw;
use super::settled::{Settled, climate_of, settled, settled_with};
use super::tests::{
    EARTH, MARS, MERCURY, ReferenceBody, TITAN, VENUS, ancestry, derived_from, reference_trial,
};
use science::constants::STEFAN_BOLTZMANN;
use science::radiation::gases::PartialPressures;
use science::surface::{condense, titan_like_air};
use science::units::{Bar, Degree, EarthFlux, Fraction, Kelvin, WattPerSquareMetre};
use std::collections::BTreeSet;
use timeline::{
    ClimateState, Code, Gas, LiquidKind, NodeId, NodeValue, Origin, Provenance, Seed, Trace,
};

/// Definition: the seed of the tests, whose snowball draw
/// ([`LABELS`]`.snowball`) a body that is not bistable never reads.
const SEED: u64 = 1;

/// Definition: a body with no ice caps (Mercury, Venus), whose obliquity
/// weights only the light on ice caps: 0 degrees.
const NO_ICE_OBLIQUITY: Degree = Degree(0.0);

/// Source: Venus's surface temperature on the model's spectral column
/// under its 85 bar of CO₂ and its Bond albedo of 0.76: 577 K. The
/// measured 737 K (NASA Venus fact sheet) lies higher by the infrared
/// opacity of its H₂SO₄ clouds and its SO₂, which the model does not
/// carry (#594); the bound pins what the model reaches. Legacy
/// `the_solar_system_has_its_climates`, issue #523.
const VENUS_SURFACE: Kelvin = Kelvin(577.0);

/// Definition: the window on [`VENUS_SURFACE`], 5 K (#594, ported as is).
const VENUS_WINDOW: Kelvin = Kelvin(5.0);

/// Source: Venus's Bond albedo, 0.76 (NASA Venus fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html).
const VENUS_BOND: f64 = 0.76;

/// Definition: the window on [`VENUS_BOND`], 0.05.
const VENUS_BOND_WINDOW: f64 = 0.05;

/// Source: Mercury's Bond albedo, 0.088 (NASA Mercury fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/mercuryfact.html).
const MERCURY_BOND: f64 = 0.088;

/// Definition: the window on [`MERCURY_BOND`], 0.04.
const MERCURY_BOND_WINDOW: f64 = 0.04;

/// Definition: how far an airless body sits from its equilibrium
/// temperature under its own albedo, 1 K (legacy
/// `an_airless_body_sits_at_its_equilibrium_temperature`).
const AIRLESS_WINDOW: Kelvin = Kelvin(1.0);

/// Definition: the agreement of two ways of reading the same light, a
/// relative 1e-9 (legacy `the_solar_system_has_its_albedos`).
const SAME_LIGHT: f64 = 1e-9;

/// Source: Titan's surface pressure, 1.467 bar (1467 hPa; Fulchignoni et
/// al. 2005, *Nature* 438, 785, doi:10.1038/nature04314).
const TITAN_PRESSURE: f64 = 1.467;

/// Source: the methane share of Titan's air near its surface, 5.65 %
/// (Niemann et al. 2010, *J. Geophys. Res. Planets* 115, E12006,
/// doi:10.1029/2010JE003659).
const TITAN_METHANE_SHARE: f64 = 0.0565;

/// Definition: the methane a test inventory holds above what Titan's air
/// keeps, 0.5 bar, so that the sea can form. A test input chosen to
/// exceed the saturation of the air at Titan's temperature, not a
/// measurement: the measured air is what the sea leaves in the air.
const TITAN_METHANE_SURPLUS: f64 = 0.5;

/// Source: Titan's surface temperature, 93.65 ± 0.25 K (Fulchignoni et
/// al. 2005, *Nature* 438, 785), the centre of the validation window.
const TITAN_SURFACE: Kelvin = Kelvin(94.0);

/// Definition: the window on [`TITAN_SURFACE`], 8 K (a validation now
/// that #646 targets the column).
const TITAN_WINDOW: Kelvin = Kelvin(8.0);

/// Definition: the relative slack on a cap the fixed point reaches
/// within its tolerance, 1e-6.
const CAP_SLACK: f64 = 1e-6;

/// Definition: an icy body of 0.0036 Earth masses at 30 au, a Triton-like
/// test input (its methane survives, its air is thin): the body the
/// reducing-air fallback is tested on. Its water share and core share are
/// [`TITAN`]'s, test choices.
const TRITON_LIKE: ReferenceBody = ReferenceBody {
    mass: 0.0036,
    core_mass_fraction: TITAN.core_mass_fraction,
    water_fraction: TITAN.water_fraction,
    orbit: 30.0,
    eccentricity: 0.0,
};

/// Definition: the CO₂ a hand-built frozen branch froze with, 0.01 bar
/// (a test input).
const FROZE: Bar = Bar(0.01);

/// Definition: the CO₂ a hand-built frozen branch is still frozen with,
/// 0.02 bar, above [`FROZE`] (a test input).
const STILL_FROZEN: Bar = Bar(0.02);

/// Definition: a snowball draw of the middle, 0.5, which a hand-built
/// chance of 1 always falls under.
const DRAW: f64 = 0.5;

/// Source: Earth's Bond albedo, 0.306 (NASA Earth fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html).
const EARTH_BOND: f64 = 0.306;

/// Definition: the window on [`EARTH_BOND`], 0.03.
const EARTH_BOND_WINDOW: f64 = 0.03;

/// Source: Earth's surface temperature, 288 K (NASA Earth fact sheet).
const EARTH_SURFACE: Kelvin = Kelvin(288.0);

/// Definition: the window on [`EARTH_SURFACE`], 6 K.
const EARTH_WINDOW: Kelvin = Kelvin(6.0);

/// Definition: the CO₂ under which the thermostat keeps an Earth whose
/// air is not a runaway's, 0.01 bar (legacy `the_solar_system_has_its_climates`).
const EARTH_CO2_BOUND: f64 = 0.01;

/// Source: Earth's cloud cover, 0.68 ± 0.03 (Stubenrauch et al. 2013,
/// *Bull. Am. Meteorol. Soc.* 94, 1031, doi:10.1175/BAMS-D-12-00117.1;
/// ISCCP and MODIS, as `science::radiation::clouds`).
const EARTH_CLOUDS: f64 = 0.68;

/// Definition: the window on [`EARTH_CLOUDS`], 0.03.
const EARTH_CLOUDS_WINDOW: f64 = 0.03;

/// Source: Earth's preindustrial CO₂, 280 ppm, 2.8e-4 bar (Myhre et al.
/// 1998, doi:10.1029/98GL01908).
const PREINDUSTRIAL_CO2: Bar = Bar(2.8e-4);

/// Source: the two temperatures that bound Earth's preindustrial
/// surface, 286 K and 290 K (Myhre et al. 1998 with the thermostat's 288
/// K: legacy `earth_holds_its_preindustrial_co2`).
const PREINDUSTRIAL_RANGE: (Kelvin, Kelvin) = (Kelvin(286.0), Kelvin(290.0));

/// Definition: the bracket of the search for the CO₂ that gives a
/// temperature, 1e-6 to 1e-2 bar, and its 30 bisection steps (legacy).
const CO2_BRACKET: (f64, f64) = (1e-6, 1e-2);

/// Definition: the bisection steps of that search, 30 (legacy).
const CO2_STEPS: usize = 30;

/// Source: the share of Earth's history spent frozen, Hoffman et al.
/// 2017 (f⊕ = 0.014), 0.013 to 0.016 (`science::snowball`).
const EARTH_CHANCE: (f64, f64) = (0.013, 0.016);

/// The seed of the snowball draw of the tests.
fn snowball() -> Seed {
    Seed::new(SEED).derive(LABELS.snowball)
}

/// The climate of the reference `body` at `obliquity`, not locked.
fn settled_of(body: &ReferenceBody, obliquity: Degree) -> (Settled, Rotation, Trace) {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, body);
    let rot = rotation(&mut trace, obliquity, false);
    let settled = settled_with(&t, &d, &rot, snowball(), &mut trace).unwrap();
    (settled, rot, trace)
}

/// The climate of `settled`.
fn climate(settled: &Settled) -> &Climate {
    settled.climate.as_ref().expect("a body with a surface")
}

/// The temperature of `settled`.
fn temperature(settled: &Settled) -> f64 {
    climate(settled).global.surface_temperature.value().0
}

/// The Bond albedo of `settled`.
fn albedo(settled: &Settled) -> f64 {
    climate(settled).global.bond_albedo.value().get()
}

/// The partial pressures of the air `climate` leaves.
fn air_of(climate: &Climate) -> PartialPressures {
    let p = |gas: Gas| climate.atmosphere.partial[&gas].value().0;
    PartialPressures {
        n2: p(Gas::N2),
        co2: p(Gas::Co2),
        h2o: p(Gas::H2o),
        ch4: p(Gas::Ch4),
        nh3: p(Gas::Nh3),
        so2: p(Gas::So2),
        o2: p(Gas::O2),
    }
}

/// Every node of `trace` recorded under `code`, in order.
fn nodes_of(trace: &Trace, code: &str) -> Vec<(NodeId, NodeValue)> {
    (0..trace.len())
        .map(|i| serde_json::from_value::<NodeId>(i.into()).unwrap())
        .filter_map(|id| {
            let node = trace.node(id).unwrap();
            (node.code.as_str() == code).then(|| (id, node.value.clone()))
        })
        .collect()
}

/// The codes of every node `node` derives from, itself included.
fn causes(trace: &Trace, node: NodeId) -> BTreeSet<String> {
    ancestry(trace, node)
        .into_iter()
        .map(|id| trace.node(id).unwrap().code.as_str().to_owned())
        .collect()
}

#[test]
fn settled_draws_then_settles_from_its_snowball_seed() {
    let mut trace = Trace::new();
    let (t, _) = reference_trial(&mut trace, &MERCURY);
    let rot = rotation(&mut trace, NO_ICE_OBLIQUITY, false);
    let seed = Seed::new(SEED);
    let mut copy = trace.clone();
    let whole = settled(&t, seed, &rot, &mut trace).unwrap();
    let drawn = draw(seed, &t, &mut copy).unwrap();
    let parts = settled_with(&t, &drawn, &rot, seed.derive(LABELS.snowball), &mut copy).unwrap();
    assert_eq!(whole, parts);
}

#[test]
fn venus_is_pinned_where_the_column_puts_it() {
    let (venus, ..) = settled_of(&VENUS, NO_ICE_OBLIQUITY);
    let c = climate(&venus);
    let t = temperature(&venus);
    assert!(
        (t - VENUS_SURFACE.0).abs() <= VENUS_WINDOW.0,
        "{t} K, {c:?}"
    );
    assert!(c.liquid.is_none(), "{c:?}");
    assert!(
        (albedo(&venus) - VENUS_BOND).abs() <= VENUS_BOND_WINDOW,
        "{c:?}"
    );
}

#[test]
fn the_equilibrium_is_the_starlight_under_the_albedo() {
    let (venus, ..) = settled_of(&VENUS, NO_ICE_OBLIQUITY);
    let c = climate(&venus);
    let flux: f64 = venus
        .bare
        .trial
        .sources
        .iter()
        .map(|s| s.flux.value().0)
        .sum();
    let top = WattPerSquareMetre::from(EarthFlux(flux));
    let starlight = top.0 / 4.0 * (1.0 - albedo(&venus));
    let emitted = STEFAN_BOLTZMANN * c.global.equilibrium.value().0.powi(4);
    assert!(
        (emitted - starlight).abs() <= SAME_LIGHT * starlight,
        "{emitted} against {starlight}"
    );
}

#[test]
fn mercury_sits_at_its_equilibrium_temperature() {
    let (mercury, ..) = settled_of(&MERCURY, NO_ICE_OBLIQUITY);
    let c = climate(&mercury);
    let equilibrium = c.global.equilibrium.value().0;
    assert!(
        (temperature(&mercury) - equilibrium).abs() < AIRLESS_WINDOW.0,
        "{c:?}"
    );
    assert!(c.liquid.is_none(), "{c:?}");
    assert!(!c.regulated.value(), "{c:?}");
    assert_eq!(*c.global.state.value(), ClimateState::Temperate);
    assert!(
        (albedo(&mercury) - MERCURY_BOND).abs() <= MERCURY_BOND_WINDOW,
        "{c:?}"
    );
}

/// Titan's measured air as the inventory of a reference Titan, with a
/// methane inventory above what the air keeps (an imposed test input, as
/// Mars's air is; #679: the reference trial's 2d-2 air holds 2.876 bar of
/// N₂ against the measured 1.384 bar, which settles it at 104.8 K).
fn titan_on_its_measured_air() -> (Bare, Rotation, Trace) {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, &TITAN);
    let rot = rotation(&mut trace, TITAN_OBLIQUITY, false);
    let mut bare = bare_with(&t, &d, &mut trace).unwrap();
    // Refs #679: the measured air replaces the reference trial's.
    let measured = [
        (
            Gas::N2,
            "test.titan.n2",
            TITAN_PRESSURE * (1.0 - TITAN_METHANE_SHARE),
        ),
        (
            Gas::Ch4,
            "test.titan.ch4",
            TITAN_PRESSURE * TITAN_METHANE_SHARE + TITAN_METHANE_SURPLUS,
        ),
        (Gas::Co2, "test.titan.co2", 0.0),
        (Gas::H2o, "test.titan.h2o", 0.0),
        (Gas::Nh3, "test.titan.nh3", 0.0),
        (Gas::So2, "test.titan.so2", 0.0),
        (Gas::O2, "test.titan.o2", 0.0),
    ];
    let surface = bare.surface.as_mut().unwrap();
    for (gas, code, p) in measured {
        let leaf = trace
            .leaf(Code::of(code), Bar(p), Provenance::Imposed)
            .unwrap();
        surface.atmosphere.partial.insert(gas, leaf);
    }
    (bare, rot, trace)
}

#[test]
fn titan_has_its_seas() {
    let (bare, rot, mut trace) = titan_on_its_measured_air();
    let (c, air) = climate_of(&bare, &rot, DRAW, &mut trace).unwrap();
    let t = c.global.surface_temperature.value().0;
    assert!(
        (t - TITAN_SURFACE.0).abs() <= TITAN_WINDOW.0,
        "{t} K, {c:?}"
    );
    assert_eq!(c.liquid.map(|l| *l.value()), Some(LiquidKind::Hydrocarbons));
    assert!(c.condensate.contains_key(&Gas::Ch4), "{c:?}");
    // The air keeps at most what condense lets it keep at that
    // temperature, the inventory being the measured air and the surplus.
    let inventory = ClimateInputs::of(&bare, &rot, Bar(0.0)).unwrap().air;
    let cap = condense(&inventory, Kelvin(t)).air.ch4;
    assert!(air.ch4 <= cap * (1.0 + CAP_SLACK), "{} over {cap}", air.ch4);
    assert_eq!(*c.global.state.value(), ClimateState::Temperate);
    // The sea carries its causes: the temperature, the pressure and the
    // methane on the ground.
    let sea = c.liquid.unwrap().node();
    let from = derived_from(&trace, sea);
    for node in [
        c.global.surface_temperature.node(),
        c.atmosphere.surface_pressure.node(),
        c.condensate[&Gas::Ch4].node(),
    ] {
        assert!(from.contains(&node), "{node:?} missing from {from:?}");
    }
}

#[test]
fn the_climate_records_its_causes() {
    for (name, body, obliquity) in [
        ("Mercury", MERCURY, NO_ICE_OBLIQUITY),
        ("Mars", MARS, MARS_OBLIQUITY),
    ] {
        let (settled, rot, trace) = settled_of(&body, obliquity);
        let c = climate(&settled);
        let surface = settled.bare.surface.as_ref().unwrap();
        if name == "Mars" {
            assert_eq!(*c.global.state.value(), ClimateState::Temperate, "{c:?}");
        }
        // Each code is there, with a value: the snowball nodes, the
        // condensates, the liquid and the Titan test only where their
        // condition holds, which a body that is not bistable, dry and not
        // reducing does not meet.
        assert!(c.snowball.is_none(), "{name}");
        assert!(c.liquid.is_none() && c.condensate.is_empty(), "{name}");
        for (code, id) in [
            (
                codes::CLIMATE_SURFACE_TEMPERATURE,
                c.global.surface_temperature.node(),
            ),
            (codes::CLIMATE_EQUILIBRIUM, c.global.equilibrium.node()),
            (codes::CLIMATE_BOND_ALBEDO, c.global.bond_albedo.node()),
            (codes::CLIMATE_STATE, c.global.state.node()),
            (codes::CLIMATE_EMISSIVITY, c.emissivity.node()),
            (codes::CLIMATE_REGULATED, c.regulated.node()),
            (codes::CLIMATE_CONVERGED, c.converged.node()),
            (
                codes::CLIMATE_AIR_PRESSURE,
                c.atmosphere.surface_pressure.node(),
            ),
        ] {
            assert_eq!(trace.node(id).unwrap().code.as_str(), code, "{name}");
        }
        for (gas, code) in codes::CLIMATE_AIR_GASES {
            assert_eq!(
                trace
                    .node(c.atmosphere.partial[&gas].node())
                    .unwrap()
                    .code
                    .as_str(),
                code,
                "{name}"
            );
        }
        for absent in [
            codes::CLIMATE_FROZE_CO2,
            codes::CLIMATE_THAWS,
            codes::CLIMATE_THAW_CO2,
            codes::CLIMATE_SNOWBALL_CHANCE,
            codes::CLIMATE_SNOWBALL_DRAW,
            codes::CLIMATE_TITAN_LIKE,
            codes::CLIMATE_CONDENSATE_N2,
            codes::CLIMATE_CONDENSATE_CH4,
            codes::SURFACE_LIQUID,
        ] {
            assert!(nodes_of(&trace, absent).is_empty(), "{name}: {absent}");
        }

        // The temperature follows every source's flux and temperature,
        // and what the body is made of.
        let from = ancestry(&trace, c.global.surface_temperature.node());
        for s in settled.bare.trial.sources.iter() {
            assert!(from.contains(&s.flux.node()), "{name}");
            assert!(from.contains(&s.effective_temperature.node()), "{name}");
        }
        for node in [
            surface.water.inventory.node(),
            surface.regime.node(),
            surface.volcanism.node(),
            surface.outgassed_co2.node(),
            settled.bare.heat.radiogenic.node(),
            rot.obliquity.node(),
            rot.locked.node(),
            c.global.state.node(),
        ] {
            assert!(from.contains(&node), "{name}: {node:?}");
        }
        // The albedo, the emissivity and the flags follow the temperature;
        // the equilibrium follows the albedo and the fluxes.
        for node in [
            c.global.bond_albedo.node(),
            c.emissivity.node(),
            c.regulated.node(),
            c.converged.node(),
        ] {
            assert!(
                ancestry(&trace, node).contains(&c.global.surface_temperature.node()),
                "{name}"
            );
        }
        let from = derived_from(&trace, c.global.equilibrium.node());
        assert!(from.contains(&c.global.bond_albedo.node()), "{name}");
        for s in settled.bare.trial.sources.iter() {
            assert!(from.contains(&s.flux.node()), "{name}");
        }
        // The CO₂ of the air follows what was outgassed and frozen, and
        // the vapour the water.
        let co2 = causes(&trace, c.atmosphere.partial[&Gas::Co2].node());
        assert!(co2.contains(codes::AIR_OUTGASSED_CO2), "{name}");
        assert!(co2.contains(codes::AIR_FROZEN_CO2), "{name}");
        let h2o = derived_from(&trace, c.atmosphere.partial[&Gas::H2o].node());
        assert!(h2o.contains(&surface.water.inventory.node()), "{name}");
        // The pressure is the sum of the partials.
        let pressure = derived_from(&trace, c.atmosphere.surface_pressure.node());
        for p in c.atmosphere.partial.values() {
            assert!(pressure.contains(&p.node()), "{name}");
        }
        // The graph the climate leaves is a valid one.
        let mut global = c.global.clone();
        trace.freeze(&mut global).unwrap().validate().unwrap();
    }
}

#[test]
fn an_unreachable_thaw_records_no_infinity() {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, &MARS);
    let rot = rotation(&mut trace, MARS_OBLIQUITY, false);
    let bare = bare_with(&t, &d, &mut trace).unwrap();
    let inputs = ClimateInputs::of(&bare, &rot, Bar(0.0)).unwrap();
    // A bistable body, frozen over, which no CO₂ it can hold thaws: the
    // `Frozen` state of its frozen branch.
    let solved = Solved {
        state: ClimateState::Frozen,
        bistable: Some(Bistable {
            froze: FROZE,
            still_frozen: STILL_FROZEN,
            thaw: None,
            chance: Fraction::new(1.0).unwrap(),
        }),
        ..solve(&inputs, DRAW)
    };
    let c = record(&solved, &bare, &rot, DRAW, &mut trace).unwrap();
    let nodes = c.snowball.as_ref().expect("a bistable body");
    assert_eq!(*nodes.froze.value(), FROZE);
    assert!(!nodes.thaws.value());
    assert!(nodes.thaw.is_none());
    assert_eq!(nodes.chance.value().get(), 1.0);
    assert_eq!(*c.global.state.value(), ClimateState::Frozen);
    assert!(nodes_of(&trace, codes::CLIMATE_THAW_CO2).is_empty());
    // The chance derives from the flag, the CO₂ it froze with and the
    // volcanism; the draw is a drawn leaf; the state reads both.
    let from = derived_from(&trace, nodes.chance.node());
    let volcanism = bare.surface.as_ref().unwrap().volcanism.node();
    for node in [nodes.thaws.node(), nodes.froze.node(), volcanism] {
        assert!(from.contains(&node), "{node:?} missing from {from:?}");
    }
    assert_eq!(
        trace.node(nodes.draw.node()).unwrap().origin,
        Origin::Leaf(Provenance::Drawn)
    );
    let from = derived_from(&trace, c.global.state.node());
    assert!(from.contains(&nodes.chance.node()) && from.contains(&nodes.draw.node()));
    let mut global = c.global.clone();
    trace.freeze(&mut global).unwrap().validate().unwrap();
}

#[test]
fn a_reducing_air_keeps_its_co2_only_when_titan_like() {
    // Titan: its climate air is a Titan's, so it keeps the air made
    // without its CO₂, once.
    let (titan, _, trace) = settled_of(&TITAN, TITAN_OBLIQUITY);
    let surface = titan.bare.surface.as_ref().unwrap();
    assert!(*surface.reducing.value());
    assert_eq!(surface.outgassed_co2.value().0, 0.0);
    assert!(titan_like_air(&air_of(climate(&titan))));
    let like = nodes_of(&trace, codes::CLIMATE_TITAN_LIKE);
    assert_eq!(like.len(), 1);
    assert_eq!(like[0].1, NodeValue::Flag(true));
    assert_eq!(
        nodes_of(&trace, codes::CLIMATE_SURFACE_TEMPERATURE).len(),
        1
    );

    // A Triton-like body: its methane survives, but its thin air is not
    // a Titan's, so the air is made again with its CO₂ and settled again;
    // the first pass stays in the trace.
    let (triton, _, trace) = settled_of(&TRITON_LIKE, TITAN_OBLIQUITY);
    let surface = triton.bare.surface.as_ref().unwrap();
    assert!(!surface.reducing.value());
    assert!(surface.outgassed_co2.value().0 > 0.0);
    let reducing: Vec<_> = nodes_of(&trace, codes::AIR_REDUCING)
        .into_iter()
        .map(|(_, v)| v)
        .collect();
    assert_eq!(reducing, [NodeValue::Flag(true), NodeValue::Flag(false)]);
    let outgassed: Vec<_> = nodes_of(&trace, codes::AIR_OUTGASSED_CO2)
        .into_iter()
        .map(|(_, v)| v)
        .collect();
    assert_eq!(outgassed.len(), 2);
    assert!(matches!(outgassed[0], NodeValue::Number { value, .. } if value == 0.0));
    assert!(matches!(outgassed[1], NodeValue::Number { value, .. } if value > 0.0));
    let like = nodes_of(&trace, codes::CLIMATE_TITAN_LIKE);
    assert_eq!(like.len(), 1);
    assert_eq!(like[0].1, NodeValue::Flag(false));
    assert_eq!(
        nodes_of(&trace, codes::CLIMATE_SURFACE_TEMPERATURE).len(),
        2
    );
    assert!(!titan_like_air(&air_of(climate(&triton))));
}

/// The temperature of the wet climate of `inputs` with `co2` of CO₂
/// under the settled albedo and emissivity of `c`.
fn wet_temperature(inputs: &ClimateInputs, c: &Climate, co2: f64) -> f64 {
    use super::climate::body::ClimateBody;
    use super::climate::wet::wet_climate;
    let mut body = ClimateBody::new(inputs, Numerics::DEFAULT);
    let starlight = body.starlight(*c.global.bond_albedo.value());
    let air = PartialPressures { co2, ..air_of(c) };
    wet_climate(&mut body, starlight, &air, *c.emissivity.value())
        .1
        .0
}

#[test]
#[ignore = "heavy: a bistable Earth settles in about a minute at opt-level 0 (the snowball branches), and the CO₂ bounds solve a wet climate sixty times"]
fn earth_has_its_climate() {
    use super::climate::body::ClimateBody;
    let (earth, rot, _) = settled_of(&EARTH, EARTH_OBLIQUITY);
    let c = climate(&earth);
    let t = temperature(&earth);
    assert!(
        (t - EARTH_SURFACE.0).abs() <= EARTH_WINDOW.0,
        "{t} K, {c:?}"
    );
    assert_eq!(
        c.liquid.map(|l| *l.value()),
        Some(LiquidKind::Water),
        "{c:?}"
    );
    assert!(*c.regulated.value(), "{c:?}");
    assert_eq!(*c.global.state.value(), ClimateState::Temperate, "{c:?}");
    let co2 = c.atmosphere.partial[&Gas::Co2].value().0;
    assert!(co2 < EARTH_CO2_BOUND, "{co2} bar");
    assert!(
        (albedo(&earth) - EARTH_BOND).abs() <= EARTH_BOND_WINDOW,
        "{c:?}"
    );

    // The cloud cover the climate settles under.
    let inputs = ClimateInputs::of(&earth.bare, &rot, Bar(0.0)).unwrap();
    let air = air_of(c);
    let clouds = ClimateBody::new(&inputs, Numerics::DEFAULT)
        .underneath(Kelvin(t), &air)
        .clouds
        .get();
    assert!(
        (clouds - EARTH_CLOUDS).abs() <= EARTH_CLOUDS_WINDOW,
        "{clouds}"
    );

    // 280 ppm under its own albedo gives 286 to 290 K, and the
    // thermostat's CO₂ lies between those that give 286 and 290 K.
    let preindustrial = wet_temperature(&inputs, c, PREINDUSTRIAL_CO2.0);
    assert!(
        (PREINDUSTRIAL_RANGE.0.0..=PREINDUSTRIAL_RANGE.1.0).contains(&preindustrial),
        "{preindustrial} K"
    );
    let co2_for = |target: f64| {
        let (mut low, mut high) = (CO2_BRACKET.0.ln(), CO2_BRACKET.1.ln());
        for _ in 0..CO2_STEPS {
            let mid = 0.5 * (low + high);
            if wet_temperature(&inputs, c, mid.exp()) < target {
                low = mid;
            } else {
                high = mid;
            }
        }
        (0.5 * (low + high)).exp()
    };
    let (cold, warm) = (
        co2_for(PREINDUSTRIAL_RANGE.0.0),
        co2_for(PREINDUSTRIAL_RANGE.1.0),
    );
    assert!((cold..=warm).contains(&co2), "{co2} outside {cold}..{warm}");

    // Earth is bistable, with a share of its history frozen as dated.
    let snowball = c.snowball.as_ref().expect("Earth is bistable");
    let chance = snowball.chance.value().get();
    assert!(
        (EARTH_CHANCE.0..=EARTH_CHANCE.1).contains(&chance),
        "{chance}"
    );
}

#[test]
#[ignore = "heavy: two bistable-Earth solves, about two minutes at opt-level 0"]
fn earth_twice_is_the_same_earth() {
    // The rule "same planet, same answer": settling Earth twice from one
    // seed, on a clone of the trace and on the trace, gives the same
    // values and the same nodes.
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, &EARTH);
    let rot = rotation(&mut trace, EARTH_OBLIQUITY, false);
    let mut copy = trace.clone();
    let first = settled_with(&t, &d, &rot, snowball(), &mut trace).unwrap();
    let second = settled_with(&t, &d, &rot, snowball(), &mut copy).unwrap();
    assert_eq!(first, second);
    assert_eq!(trace.len(), copy.len());
    assert!(first.climate.as_ref().unwrap().snowball.is_some());
}
