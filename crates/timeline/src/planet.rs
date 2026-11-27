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
    /// A gas of an atmosphere. Argon (`Ar`) is the radiogenic argon-40
    /// the rock outgasses (study 29, #694).
    Gas {
        N2 => "n2",
        Co2 => "co2",
        H2o => "h2o",
        Ch4 => "ch4",
        Nh3 => "nh3",
        So2 => "so2",
        O2 => "o2",
        Ar => "ar",
    }
}

/// The atmosphere at the surface.
///
/// Its gases are weights, not partial pressures: `partial[gas]` is the
/// gas's mass times the surface gravity over the planet's area,
/// `m_i g / (4 π R²)`, in bar. Weights add: their sum is
/// `surface_pressure`. A gas's partial pressure, its share by volume of
/// that pressure, `p_i = (w_i / M_i) / Σ_j (w_j / M_j) × Σ_j w_j` over the
/// dry gases (water vapour added on top), depends on the whole air; a
/// reader that needs it converts the whole air
/// (`science::radiation::gases::Weights::partial_pressures`, issue #749).
/// The water vapour's weight is taken as its pressure at the surface, the
/// column's convention: over a wet surface, the saturation pressure at its
/// temperature; for a runaway, the steam its air keeps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Atmosphere {
    /// The surface pressure, bar: the sum of the gases' weights.
    pub surface_pressure: Traced<Bar>,
    /// Each gas's weight, bar (see [`Atmosphere`]); the name is the
    /// contract's, kept since the format reads it.
    pub partial: BTreeMap<Gas, Traced<Bar>>,
}

traced_fields!(Atmosphere {
    surface_pressure,
    partial
});

contract_enum! {
    /// The global state of the climate: temperate (the warm branch, or
    /// a body with no water to freeze), a snowball (frozen over until
    /// volcanic CO₂ thaws it), frozen for good (a body with water,
    /// frozen over, which no CO₂ it can hold thaws) or a runaway
    /// greenhouse.
    ClimateState {
        Temperate => "temperate",
        Snowball => "snowball",
        Frozen => "frozen",
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
    pub life: Traced<Biosphere>,
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
    /// The air life has made: none, the air it found, CH4 gone, modern O2.
    Biosphere {
        Lifeless => "lifeless",
        Anoxic => "anoxic",
        Oxidising => "oxidising",
        Modern => "modern",
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Code, Provenance, Trace, TracedFields};

    #[test]
    fn orbit_and_spin_visit_every_traced_field() {
        let mut trace = Trace::new();
        let input = Provenance::Input;
        let mut orbit = Orbit {
            semi_major_axis: trace
                .leaf(Code::of("orbit.a"), AstronomicalUnit(1.0), input)
                .unwrap(),
            eccentricity: trace
                .leaf(Code::of("orbit.e"), Fraction::new(0.1).unwrap(), input)
                .unwrap(),
            period: trace.leaf(Code::of("orbit.p"), Day(365.0), input).unwrap(),
            perihelion_longitude: trace
                .leaf(Code::of("orbit.w"), Degree(10.0), input)
                .unwrap(),
        };
        let mut spin = Spin {
            rotation_period: trace.leaf(Code::of("spin.p"), Day(1.0), input).unwrap(),
            obliquity: trace.leaf(Code::of("spin.o"), Degree(23.0), input).unwrap(),
        };
        let expected = [
            orbit.semi_major_axis.node(),
            orbit.eccentricity.node(),
            orbit.period.node(),
            orbit.perihelion_longitude.node(),
            spin.rotation_period.node(),
            spin.obliquity.node(),
        ];
        let mut seen = Vec::new();
        orbit.for_each_node(&mut |id| seen.push(*id));
        spin.for_each_node(&mut |id| seen.push(*id));
        assert_eq!(seen, expected);
        let graph = trace.freeze(&mut vec![orbit]).unwrap();
        assert_eq!(graph.nodes.len(), 4);
    }

    #[test]
    fn biosphere_keys_are_kebab_case() {
        for (stage, key) in [
            (Biosphere::Lifeless, "lifeless"),
            (Biosphere::Anoxic, "anoxic"),
            (Biosphere::Oxidising, "oxidising"),
            (Biosphere::Modern, "modern"),
        ] {
            let json = serde_json::to_string(&stage).unwrap();
            assert_eq!(json, format!("\"{key}\""));
            assert_eq!(serde_json::from_str::<Biosphere>(&json).unwrap(), stage);
        }
    }
}
