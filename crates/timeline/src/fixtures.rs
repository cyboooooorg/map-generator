//! Structural fixtures for the tests of the crates that consume the
//! contract (behind the `fixtures` feature, and in this crate's tests).

use crate::{
    Atmosphere, BandLight, Bulk, ClimateState, Code, Composition, Core, CoreState, Gas,
    GlobalClimate, Heat, InteriorProfile, Layer, Life, LiquidKind, Material, MatterState,
    NodeValueOf, Orbit, PlanetClass, PlanetState, Provenance, RadialSample, RadialSamples, Renewal,
    Spin, StellarSource, StellarSources, Surface, SurfaceLiquid, TargetOutcome, TectonicRegime,
    Terrain, TerrainUnit, Trace, Traced, Transport, Water,
};
use science::constants::{EARTH_MEAN_RADIUS, MARS_SURFACE_GRAVITY, STANDARD_GRAVITY};
use science::units::{
    AstronomicalUnit, Bar, Day, Degree, EarthFlux, EarthMass, EarthRadius, Fraction, Gyr,
    JoulePerSquareMetre, Kelvin, Kilogram, KilogramPerCubicMetre, Metre, MetrePerSecondSquared,
    Pascal, SolarLuminosity, SolarMass, Tesla, Watt, WattPerSquareMetre,
};
use std::collections::BTreeMap;

pub fn leaf<T: NodeValueOf>(trace: &mut Trace, code: &'static str, value: T) -> Traced<T> {
    trace
        .leaf(Code::of(code), value, Provenance::Imposed)
        .unwrap()
}

pub fn frac(x: f64) -> Fraction {
    Fraction::new(x).unwrap()
}

pub fn interior(trace: &mut Trace, radii: [f64; 2]) -> InteriorProfile {
    let layers = radii
        .iter()
        .map(|&r| Layer {
            outer_radius: leaf(trace, "interior.layer.radius", Metre(r)),
            mass: leaf(trace, "interior.layer.mass", Kilogram(1.0e24)),
            material: leaf(trace, "interior.layer.material", Material::Rock),
            state: leaf(trace, "interior.layer.state", MatterState::Solid),
            transport: Some(leaf(
                trace,
                "interior.layer.transport",
                Transport::Convective,
            )),
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

/// Plausible Earth-like values used as structural inputs: never asserted
/// on, and not physically consistent with one another.
pub fn earth_like_fixture(trace: &mut Trace, age: f64) -> PlanetState {
    let source = StellarSource {
        flux: leaf(trace, "star.flux", EarthFlux(1.0)),
        equilibrium: leaf(trace, "star.equilibrium", Kelvin(255.0)),
        effective_temperature: leaf(trace, "star.teff", Kelvin(5772.0)),
        mass: leaf(trace, "star.mass", SolarMass(1.0)),
        luminosity: leaf(trace, "star.luminosity", SolarLuminosity(1.0)),
        xuv: BandLight {
            flux: leaf(trace, "star.xuv.flux", WattPerSquareMetre(4.6e-3)),
            fluence: leaf(trace, "star.xuv.fluence", JoulePerSquareMetre(5.64e15)),
        },
        lyman_alpha: BandLight {
            flux: leaf(trace, "star.lyman-alpha.flux", WattPerSquareMetre(6.4e-3)),
            fluence: leaf(
                trace,
                "star.lyman-alpha.fluence",
                JoulePerSquareMetre(2.49e15),
            ),
        },
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
                renewal: leaf(trace, "terrain.renewal", Renewal::Plates),
            }],
        },
        life: Life {
            arose: leaf(trace, "life.arose", true),
            oxygenated: leaf(trace, "life.oxygenated", true),
        },
    };
    PlanetState {
        sources: StellarSources::new(vec![source]).unwrap(),
        age: leaf(trace, "planet.age", Gyr(age)),
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
        targets: vec![leaf(
            trace,
            "archetype.target.ocean",
            TargetOutcome::Unreachable,
        )],
    }
}

/// Source: Mars's volumetric mean radius, 3389.5 km (NASA Mars Fact
/// Sheet, <https://nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html>).
pub const MARS_RADIUS: Metre = Metre(3389.5e3);

/// Source: Venus's volumetric mean radius, 6051.8 km (NASA Venus Fact
/// Sheet, <https://nssdc.gsfc.nasa.gov/planetary/factsheet/venusfact.html>).
const VENUS_RADIUS: Metre = Metre(6051.8e3);

/// Source: Venus's equatorial surface gravity, 8.87 m/s^2 (NASA Venus
/// Fact Sheet).
const VENUS_GRAVITY: MetrePerSecondSquared = MetrePerSecondSquared(8.87);

pub fn relief_body(
    trace: &mut Trace,
    regime: TectonicRegime,
    radius: Metre,
    gravity: f64,
    ocean_share: Option<f64>,
) -> PlanetState {
    let mut state = earth_like_fixture(trace, 4.57);
    state.bulk.radius = leaf(trace, "bulk.radius", EarthRadius::from(radius));
    state.bulk.surface_gravity = leaf(trace, "bulk.gravity", MetrePerSecondSquared(gravity));
    let surface = state.surface.as_mut().unwrap();
    surface.regime = leaf(trace, "surface.regime", regime);
    match ocean_share {
        Some(share) => {
            let liquid = surface.liquid.as_mut().unwrap();
            liquid.ocean_share = leaf(trace, "liquid.share", Fraction::new(share).unwrap());
        }
        None => surface.liquid = None,
    }
    state
}

/// Earth: mean radius 6371.0 km, standard gravity 9.80665 m/s^2 (NASA
/// Planetary Fact Sheet, <https://nssdc.gsfc.nasa.gov/planetary/factsheet/>;
/// the constants of the science crate). Plates, 71 % ocean.
pub fn earth_relief_fixture(trace: &mut Trace) -> PlanetState {
    relief_body(
        trace,
        TectonicRegime::Plates,
        EARTH_MEAN_RADIUS,
        STANDARD_GRAVITY.0,
        Some(0.71),
    )
}

/// Mars: [`MARS_RADIUS`], mean surface gravity [`MARS_SURFACE_GRAVITY`]
/// (NASA Mars Fact Sheet). A stagnant lid.
pub fn mars_relief_fixture(trace: &mut Trace, ocean_share: Option<f64>) -> PlanetState {
    relief_body(
        trace,
        TectonicRegime::Stagnant,
        MARS_RADIUS,
        MARS_SURFACE_GRAVITY.0,
        ocean_share,
    )
}

/// Venus: [`VENUS_RADIUS`] and [`VENUS_GRAVITY`]; no liquid, episodic
/// resurfacing.
pub fn venus_relief_fixture(trace: &mut Trace) -> PlanetState {
    relief_body(
        trace,
        TectonicRegime::Episodic,
        VENUS_RADIUS,
        VENUS_GRAVITY.0,
        None,
    )
}
