//! The state of a planet at a date.

use crate::macros::{contract_enum, traced_fields};
use crate::{ContractError, InteriorProfile, StellarSources, Traced};
use science::units::{
    AstronomicalUnit, Bar, Day, Degree, EarthMass, EarthRadius, Fraction, Gyr, Kelvin, Kilogram,
    KilogramPerCubicMetre, MetrePerSecondSquared, Tesla, Watt,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The orbit around the primary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Orbit {
    pub semi_major_axis: Traced<AstronomicalUnit>,
    /// Below 1, checked by `validate`.
    pub eccentricity: Traced<Fraction>,
    pub period: Traced<Day>,
    pub perihelion_longitude: Traced<Degree>,
}

traced_fields!(Orbit {
    semi_major_axis,
    eccentricity,
    period,
    perihelion_longitude
});

impl Orbit {
    /// An eccentricity of 1 or more is not a bound orbit.
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.eccentricity.value().get() >= 1.0 {
            return Err(ContractError::EccentricityNotBelowOne);
        }
        Ok(())
    }
}

/// The rotation of the planet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spin {
    pub rotation_period: Traced<Day>,
    pub obliquity: Traced<Degree>,
}

traced_fields!(Spin {
    rotation_period,
    obliquity
});

/// Mass, size and what follows from them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bulk {
    pub mass: Traced<EarthMass>,
    pub radius: Traced<EarthRadius>,
    pub surface_gravity: Traced<MetrePerSecondSquared>,
    pub mean_density: Traced<KilogramPerCubicMetre>,
}

traced_fields!(Bulk {
    mass,
    radius,
    surface_gravity,
    mean_density
});

/// The bulk composition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    pub core_mass_fraction: Traced<Fraction>,
    pub water_fraction: Traced<Fraction>,
}

traced_fields!(Composition {
    core_mass_fraction,
    water_fraction
});

contract_enum! {
    /// The kind of a planet: its own class, also read by the system draw.
    PlanetClass {
        Rocky => "rocky",
        SuperEarth => "super-earth",
        IceWorld => "ice-world",
        MiniNeptune => "mini-neptune",
        IceGiant => "ice-giant",
        GasGiant => "gas-giant",
        HotJupiter => "hot-jupiter",
        HotNeptune => "hot-neptune",
        LavaWorld => "lava-world",
        VolcanicWorld => "volcanic-world",
        RunawayGreenhouse => "runaway-greenhouse",
        HydrocarbonWorld => "hydrocarbon-world",
        OceanWorld => "ocean-world",
        SteamWorld => "steam-world",
    }
}

/// The internal heat sources.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Heat {
    pub radiogenic: Traced<Watt>,
    pub tidal: Traced<Watt>,
    pub secular: Traced<Watt>,
}

traced_fields!(Heat {
    radiogenic,
    tidal,
    secular
});

/// The water of the planet and what happened to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Water {
    pub inventory: Traced<Kilogram>,
    /// Runaway by insolation.
    pub runaway: Traced<bool>,
    /// Boiled away by the climate.
    pub boiled: Traced<bool>,
}

traced_fields!(Water {
    inventory,
    runaway,
    boiled
});

contract_enum! {
    /// How a terrain unit was last renewed.
    Renewal {
        Formation => "formation",
        Plates => "plates",
        Episodic => "episodic",
        HeatPipe => "heat-pipe",
        IceLid => "ice-lid",
        LiquidCycle => "liquid-cycle",
        VolatileIce => "volatile-ice",
        TidalEpisode => "tidal-episode",
    }
}

/// A share of the surface of one age.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainUnit {
    pub age: Traced<Gyr>,
    pub share: Traced<Fraction>,
    pub renewal: Traced<Renewal>,
}

traced_fields!(TerrainUnit {
    age,
    share,
    renewal
});

/// The ages of the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Terrain {
    pub units: Vec<TerrainUnit>,
}

traced_fields!(Terrain { units });

contract_enum! {
    /// How the lithosphere renews itself.
    TectonicRegime {
        Plates => "plates",
        Stagnant => "stagnant",
        Episodic => "episodic",
        HeatPipe => "heat-pipe",
    }
}

contract_enum! {
    /// The state of the core.
    CoreState {
        None => "none",
        Liquid => "liquid",
        Frozen => "frozen",
    }
}

/// The core and its magnetic field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Core {
    pub state: Traced<CoreState>,
    pub field: Traced<Tesla>,
}

traced_fields!(Core { state, field });

contract_enum! {
    /// A gas of an atmosphere.
    Gas {
        N2 => "n2",
        Co2 => "co2",
        H2o => "h2o",
        Ch4 => "ch4",
        Nh3 => "nh3",
        So2 => "so2",
        O2 => "o2",
    }
}

/// The atmosphere at the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Atmosphere {
    pub surface_pressure: Traced<Bar>,
    pub partial: BTreeMap<Gas, Traced<Bar>>,
}

traced_fields!(Atmosphere {
    surface_pressure,
    partial
});

contract_enum! {
    /// The global state of the climate.
    ClimateState {
        Temperate => "temperate",
        Snowball => "snowball",
        Runaway => "runaway",
    }
}

/// The global climate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlobalClimate {
    pub surface_temperature: Traced<Kelvin>,
    pub equilibrium: Traced<Kelvin>,
    pub bond_albedo: Traced<Fraction>,
    pub state: Traced<ClimateState>,
}

traced_fields!(GlobalClimate {
    surface_temperature,
    equilibrium,
    bond_albedo,
    state
});

contract_enum! {
    /// What fills the oceans.
    LiquidKind {
        Water => "water",
        Hydrocarbons => "hydrocarbons",
        Nitrogen => "nitrogen",
        Lava => "lava",
    }
}

/// The liquid at the surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceLiquid {
    pub kind: Traced<LiquidKind>,
    /// When the liquid is water, the same node as `Water::inventory`
    /// (one code path, never computed twice).
    pub inventory: Traced<Kilogram>,
    pub ocean_share: Traced<Fraction>,
}

traced_fields!(SurfaceLiquid {
    kind,
    inventory,
    ocean_share
});

/// Life on the planet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Life {
    pub arose: Traced<bool>,
    pub oxygenated: Traced<bool>,
}

traced_fields!(Life { arose, oxygenated });

/// What a planet with a solid surface has.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    pub regime: Traced<TectonicRegime>,
    pub core: Core,
    pub water: Water,
    pub atmosphere: Option<Atmosphere>,
    pub climate: GlobalClimate,
    pub liquid: Option<SurfaceLiquid>,
    pub terrain: Terrain,
    pub life: Life,
}

traced_fields!(Surface {
    regime,
    core,
    water,
    atmosphere,
    climate,
    liquid,
    terrain,
    life
});

contract_enum! {
    /// Whether the archetype's backward search reached one of its
    /// targets; the target is the node's code, the reasons its sources.
    TargetOutcome {
        Reached => "reached",
        Unreachable => "unreachable",
    }
}

/// A planet at a date.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanetState {
    pub sources: StellarSources,
    pub age: Traced<Gyr>,
    pub orbit: Orbit,
    pub spin: Spin,
    pub bulk: Bulk,
    pub composition: Composition,
    pub class: Traced<PlanetClass>,
    pub heat: Heat,
    pub interior: InteriorProfile,
    pub envelope_fraction: Traced<Fraction>,
    /// None for a giant.
    pub surface: Option<Surface>,
    /// One outcome per archetype target, empty without an archetype, so
    /// a missed target that changes no value still reaches the keyframe.
    pub targets: Vec<Traced<TargetOutcome>>,
}

traced_fields!(PlanetState {
    sources,
    age,
    orbit,
    spin,
    bulk,
    composition,
    class,
    heat,
    interior,
    envelope_fraction,
    surface,
    targets
});

impl PlanetState {
    /// The orbit is bound and the interior is well formed.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.orbit.validate()?;
        self.interior.validate()
    }
}
