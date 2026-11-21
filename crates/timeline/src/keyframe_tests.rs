use super::*;
use crate::{
    Atmosphere, Bulk, ClimateState, Code, Composition, Core, CoreState, Gas, GlobalClimate, Heat,
    InteriorProfile, Layer, Life, LiquidKind, Material, MatterState, NodeValueOf, Orbit,
    PlanetClass, Provenance, RadialSample, RadialSamples, Renewal, Spin, StarPhase, StellarSource,
    StellarSources, Surface, SurfaceLiquid, TectonicRegime, Terrain, TerrainUnit, Traced, Water,
};
use science::units::{
    AstronomicalUnit, Bar, Day, Degree, Dex, EarthFlux, EarthMass, EarthRadius, Fraction, Kelvin,
    Kilogram, KilogramPerCubicMetre, Metre, MetrePerSecondSquared, Pascal, SolarLuminosity,
    SolarMass, SolarRadius, Tesla, Watt,
};
use std::collections::BTreeMap;

fn leaf<T: NodeValueOf>(trace: &mut Trace, code: &'static str, value: T) -> Traced<T> {
    trace
        .leaf(Code::of(code), value, Provenance::Imposed)
        .unwrap()
}

fn frac(x: f64) -> Fraction {
    Fraction::new(x).unwrap()
}

fn interior(trace: &mut Trace, radii: [f64; 2]) -> InteriorProfile {
    let layers = radii
        .iter()
        .map(|&r| Layer {
            outer_radius: leaf(trace, "interior.layer.radius", Metre(r)),
            mass: leaf(trace, "interior.layer.mass", Kilogram(1.0e24)),
            material: Material::Rock,
            state: leaf(trace, "interior.layer.state", MatterState::Solid),
        })
        .collect();
    let samples = RadialSamples(
        radii
            .iter()
            .map(|&r| RadialSample {
                radius: Metre(r),
                density: KilogramPerCubicMetre(5000.0),
                pressure: Pascal(1.0e9),
                temperature: Kelvin(3000.0),
                transport: None,
                fusion_share: None,
            })
            .collect(),
    );
    InteriorProfile {
        layers,
        samples: leaf(trace, "interior.samples", samples),
    }
}

fn earth(trace: &mut Trace) -> PlanetState {
    let source = StellarSource {
        flux: leaf(trace, "star.flux", EarthFlux(1.0)),
        equilibrium: leaf(trace, "star.equilibrium", Kelvin(255.0)),
        effective_temperature: leaf(trace, "star.teff", Kelvin(5772.0)),
        mass: leaf(trace, "star.mass", SolarMass(1.0)),
        luminosity: leaf(trace, "star.luminosity", SolarLuminosity(1.0)),
    };
    let surface = Surface {
        regime: leaf(trace, "surface.regime", TectonicRegime::Plates),
        core: Core {
            state: leaf(trace, "core.state", CoreState::Liquid),
            field: leaf(trace, "core.field", Tesla(5.0e-5)),
        },
        water: Water {
            inventory: leaf(trace, "water.inventory", Kilogram(1.4e21)),
            runaway: leaf(trace, "water.runaway", false),
            boiled: leaf(trace, "water.boiled", false),
        },
        atmosphere: Some(Atmosphere {
            surface_pressure: leaf(trace, "atmosphere.pressure", Bar(1.0)),
            partial: BTreeMap::from([
                (Gas::N2, leaf(trace, "atmosphere.n2", Bar(0.78))),
                (Gas::O2, leaf(trace, "atmosphere.o2", Bar(0.21))),
            ]),
        }),
        climate: GlobalClimate {
            surface_temperature: leaf(trace, "climate.surface", Kelvin(288.0)),
            equilibrium: leaf(trace, "climate.equilibrium", Kelvin(255.0)),
            bond_albedo: leaf(trace, "climate.albedo", frac(0.3)),
            state: leaf(trace, "climate.state", ClimateState::Temperate),
        },
        liquid: Some(SurfaceLiquid {
            kind: leaf(trace, "liquid.kind", LiquidKind::Water),
            inventory: leaf(trace, "liquid.inventory", Kilogram(1.4e21)),
            ocean_share: leaf(trace, "liquid.share", frac(0.71)),
        }),
        terrain: Terrain {
            units: vec![TerrainUnit {
                age: leaf(trace, "terrain.age", science::units::Gyr(0.1)),
                share: leaf(trace, "terrain.share", frac(1.0)),
                renewal: Renewal::Plates,
            }],
        },
        life: Life {
            arose: leaf(trace, "life.arose", true),
            oxygenated: leaf(trace, "life.oxygenated", true),
        },
    };
    PlanetState {
        sources: StellarSources::new(vec![source]).unwrap(),
        age: leaf(trace, "planet.age", Gyr(4.57)),
        orbit: Orbit {
            semi_major_axis: leaf(trace, "orbit.a", AstronomicalUnit(1.0)),
            eccentricity: leaf(trace, "orbit.e", frac(0.017)),
            period: leaf(trace, "orbit.period", Day(365.25)),
            perihelion_longitude: leaf(trace, "orbit.longitude", Degree(102.9)),
        },
        spin: Spin {
            rotation_period: leaf(trace, "spin.period", Day(1.0)),
            obliquity: leaf(trace, "spin.obliquity", Degree(23.4)),
        },
        bulk: Bulk {
            mass: leaf(trace, "bulk.mass", EarthMass(1.0)),
            radius: leaf(trace, "bulk.radius", EarthRadius(1.0)),
            surface_gravity: leaf(trace, "bulk.gravity", MetrePerSecondSquared(9.8)),
            mean_density: leaf(trace, "bulk.density", KilogramPerCubicMetre(5514.0)),
        },
        composition: Composition {
            core_mass_fraction: leaf(trace, "composition.core", frac(0.33)),
            water_fraction: leaf(trace, "composition.water", frac(0.0002)),
        },
        class: leaf(trace, "planet.class", PlanetClass::Rocky),
        heat: Heat {
            radiogenic: leaf(trace, "heat.radiogenic", Watt(2.0e13)),
            tidal: leaf(trace, "heat.tidal", Watt(1.0e11)),
            secular: leaf(trace, "heat.secular", Watt(1.0e13)),
        },
        interior: interior(trace, [3.48e6, 6.371e6]),
        envelope_fraction: leaf(trace, "planet.envelope", frac(0.0)),
        surface: Some(surface),
    }
}

fn star(trace: &mut Trace) -> StarState {
    StarState {
        phase: StarPhase::MainSequence,
        mass: leaf(trace, "star.mass", SolarMass(1.0)),
        radius: leaf(trace, "star.radius", SolarRadius(1.0)),
        luminosity: leaf(trace, "star.luminosity", SolarLuminosity(1.0)),
        effective_temperature: leaf(trace, "star.teff", Kelvin(5772.0)),
        metallicity: leaf(trace, "star.metallicity", Dex(0.0)),
        interior: None,
    }
}

fn planet_keyframe(date: f64) -> Keyframe {
    let mut trace = Trace::new();
    let state = earth(&mut trace);
    Keyframe::new(Gyr(date), BodyState::Planet(state), &trace).unwrap()
}

fn star_keyframe(date: f64) -> Keyframe {
    let mut trace = Trace::new();
    let state = star(&mut trace);
    Keyframe::new(Gyr(date), BodyState::Star(state), &trace).unwrap()
}

fn timeline(keyframes: Vec<Keyframe>) -> Result<Timeline, ContractError> {
    Timeline::new(
        "MGS 12345 b".parse().unwrap(),
        Seed::new(7),
        EngineVersion(1),
        Gyr(4.57),
        keyframes,
    )
}

#[test]
fn a_planet_keyframe_explains_every_field() {
    let mut trace = Trace::new();
    let state = earth(&mut trace);
    let k = Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace).unwrap();
    k.validate().unwrap();
    let mut ids = Vec::new();
    let mut state = k.state().clone();
    state.for_each_node(&mut |id| ids.push(*id));
    assert!(ids.len() == 50);
    assert!(ids.iter().all(|id| k.causes().node(*id).is_some()));
}

#[test]
fn a_keyframe_round_trips_through_json() {
    for k in [planet_keyframe(4.57), star_keyframe(0.0)] {
        let json = serde_json::to_string(&k).unwrap();
        let back: Keyframe = serde_json::from_str(&json).unwrap();
        assert_eq!(back, k);
        back.validate().unwrap();
    }
}

#[test]
fn a_corrupted_keyframe_is_reported() {
    let k = planet_keyframe(4.57);
    let mut json: serde_json::Value = serde_json::to_value(&k).unwrap();
    *json.pointer_mut("/state/planet/age/node").unwrap() = serde_json::json!(9999);
    let back: Keyframe = serde_json::from_value(json).unwrap();
    assert_eq!(
        back.validate(),
        Err(ContractError::UnknownNode { node: 9999 })
    );
}

#[test]
fn layers_out_of_order_are_refused() {
    let mut trace = Trace::new();
    let mut state = earth(&mut trace);
    state.interior = interior(&mut trace, [6.371e6, 3.48e6]);
    assert_eq!(
        Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace),
        Err(ContractError::LayersNotOrdered)
    );
}

#[test]
fn an_eccentricity_of_one_is_refused() {
    let mut trace = Trace::new();
    let mut state = earth(&mut trace);
    state.orbit.eccentricity = leaf(&mut trace, "orbit.e", frac(1.0));
    assert_eq!(
        Keyframe::new(Gyr(4.57), BodyState::Planet(state), &trace),
        Err(ContractError::EccentricityNotBelowOne)
    );
}

#[test]
fn a_timeline_needs_ordered_keyframes_of_one_body_kind() {
    assert_eq!(timeline(vec![]), Err(ContractError::NoKeyframe));
    assert_eq!(
        timeline(vec![planet_keyframe(4.57), planet_keyframe(4.0)]),
        Err(ContractError::KeyframesNotOrdered)
    );
    assert_eq!(
        timeline(vec![star_keyframe(0.0), planet_keyframe(4.57)]),
        Err(ContractError::MixedBodyKinds)
    );
    assert!(timeline(vec![planet_keyframe(1.0), planet_keyframe(4.57)]).is_ok());
}

#[test]
fn contract_enums_report_their_serialised_name() {
    assert_eq!(
        PlanetClass::HotJupiter.node_value(),
        crate::NodeValue::Code(Code::of("hot-jupiter"))
    );
    assert_eq!(
        serde_json::to_string(&Renewal::HeatPipe).unwrap(),
        "\"heat-pipe\""
    );
    assert_eq!(serde_json::to_string(&Gas::Co2).unwrap(), "\"co2\"");
}
