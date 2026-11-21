//! The interior of a planet: its layers from the composition, the
//! hydrostatic structure from `science`, every value recorded in the trace.
//!
//! The temperature profile and the state of the layers that depend on it
//! (a partly molten mantle) come with the climate, in the planet interior
//! plan's second pull request: the mantle is `Solid` here.

use super::codes;
use super::draws::Drawn;
use crate::Trial;
use science::constants::GRAVITATIONAL_CONSTANT;
use science::eos::{EosError, MaterialEos, core_alloy, mantle, water, water_on_melting_curve};
use science::structure::{LayerSpec, Structure, StructureError, integrate};
use science::units::{
    EarthRadius, FractionOutOfRange, Kelvin, Kilogram, KilogramPerCubicMetre, Metre,
    MetrePerSecondSquared, Pascal,
};
use science::water::{V_VI_LIQUID_TEMPERATURE, VI_VII_LIQUID_TEMPERATURE};
use timeline::{
    Bulk, Code, Composition, ContractError, Material, MatterState, NodeId, Trace, Traced,
};

/// Definition: the pressure at the surface of the structure, 0 Pa. The
/// planet has no atmosphere before the climate of the planet interior
/// plan's second pull request, which sets the surface pressure.
const SURFACE_PRESSURE: Pascal = Pascal(0.0);

/// One shell of the structure, recorded.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShellRecord {
    pub outer_radius: Traced<Metre>,
    pub mass: Traced<Kilogram>,
    pub material: Traced<Material>,
    pub state: Traced<MatterState>,
}

/// The interior of a planet in hydrostatic equilibrium.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Interior {
    pub bulk: Bulk,
    pub composition: Composition,
    pub shells: Vec<ShellRecord>,
    pub structure: Structure,
}

/// Why an interior or its heat could not be computed. Crate-internal
/// until the `SystemError` of the draw lands; then a variant of it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum InteriorError {
    Contract(ContractError),
    Structure(StructureError),
    Eos(EosError),
    /// The water layer would sit at or above 355 K, where the water
    /// laws end (the ice VI - ice VII - liquid triple point): hot water
    /// worlds belong to the climate, the planet interior plan's second
    /// pull request.
    WaterTooHot {
        temperature: Kelvin,
    },
    NotAFraction(FractionOutOfRange),
}

impl std::fmt::Display for InteriorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contract(error) => write!(f, "interior-contract({error})"),
            Self::Structure(error) => write!(f, "interior-structure({error})"),
            Self::Eos(error) => write!(f, "interior-eos({error})"),
            Self::WaterTooHot { temperature } => {
                write!(f, "interior-water-too-hot(temperature={})", temperature.0)
            }
            Self::NotAFraction(error) => write!(f, "interior-{error}"),
        }
    }
}

impl std::error::Error for InteriorError {}

impl From<ContractError> for InteriorError {
    fn from(error: ContractError) -> Self {
        Self::Contract(error)
    }
}

impl From<StructureError> for InteriorError {
    fn from(error: StructureError) -> Self {
        Self::Structure(error)
    }
}

impl From<EosError> for InteriorError {
    fn from(error: EosError) -> Self {
        Self::Eos(error)
    }
}

/// What a layer is, to name its shells.
#[derive(Clone, Copy)]
enum Kind {
    Core,
    Mantle,
    /// Water; `liquid` when the material's first phase is liquid water.
    Water {
        liquid: bool,
    },
}

/// Builds the interior of `trial` from the composition `drawn`: layers
/// Fe-alloy `cmf (1 - w) M`, rock `(1 - cmf)(1 - w) M`, water `w M` from
/// the centre out, integrated at zero surface pressure.
///
/// The water layer sits on the isotherm of the trial's equilibrium
/// temperature (its own node), which stands for the surface temperature
/// until the climate of the planet interior plan's second pull request
/// replaces it: along the isotherm when it lies in [273.31 K, 355 K), on
/// the melting curve when colder, an error when hotter.
///
/// # Errors
///
/// [`InteriorError::WaterTooHot`] for a water layer at 355 K or more; the
/// errors of the structure and of the trace otherwise.
pub(crate) fn interior(
    trial: &Trial,
    drawn: &Drawn,
    trace: &mut Trace,
) -> Result<Interior, InteriorError> {
    let total = Kilogram::from(*trial.mass.value()).0;
    let cmf = drawn.core_mass_fraction.value().get();
    let w = drawn.water_fraction.value().get();
    let mut inputs: Vec<NodeId> = vec![
        trial.mass.node(),
        drawn.core_mass_fraction.node(),
        drawn.water_fraction.node(),
    ];

    let core_eos = core_alloy();
    let mantle_eos = mantle();
    let mut layers: Vec<(Kind, MaterialEos, Kilogram)> = vec![
        (Kind::Core, core_eos, Kilogram(cmf * (1.0 - w) * total)),
        (
            Kind::Mantle,
            mantle_eos,
            Kilogram((1.0 - cmf) * (1.0 - w) * total),
        ),
    ];
    if w > 0.0 {
        let sources: Vec<NodeId> = trial.sources.iter().map(|s| s.equilibrium.node()).collect();
        let t = trial.sources.equilibrium_k();
        let temperature = trace.record(Code::of(codes::WATER_TEMPERATURE), t, &sources)?;
        inputs.push(temperature.node());
        let (eos, liquid) = if t.0 >= VI_VII_LIQUID_TEMPERATURE.0 {
            return Err(InteriorError::WaterTooHot { temperature: t });
        } else if t.0 >= V_VI_LIQUID_TEMPERATURE.0 {
            (water(t)?, true)
        } else {
            (water_on_melting_curve(), false)
        };
        layers.push((Kind::Water { liquid }, eos, Kilogram(w * total)));
    }
    let specs: Vec<LayerSpec<'_>> = layers
        .iter()
        .map(|(_, eos, mass)| LayerSpec {
            mass: *mass,
            material: eos,
        })
        .collect();
    let structure = integrate(&specs, SURFACE_PRESSURE)?;

    let radius = trace.record(
        Code::of(codes::RADIUS),
        EarthRadius::from(structure.radius),
        &inputs,
    )?;
    let g = GRAVITATIONAL_CONSTANT * total / structure.radius.0.powi(2);
    let surface_gravity = trace.record(
        Code::of(codes::SURFACE_GRAVITY),
        MetrePerSecondSquared(g),
        &[trial.mass.node(), radius.node()],
    )?;
    let volume = 4.0 / 3.0 * std::f64::consts::PI * structure.radius.0.powi(3);
    let mean_density = trace.record(
        Code::of(codes::MEAN_DENSITY),
        KilogramPerCubicMetre(total / volume),
        &[trial.mass.node(), radius.node()],
    )?;

    let mut shells = Vec::with_capacity(structure.shells.len());
    for shell in &structure.shells {
        let (material, state) = match layers[shell.layer].0 {
            Kind::Core => (
                Material::Iron,
                // Phase 0 of the alloy is the liquid outer core, phase 1
                // the solid inner core (PREM inner-core boundary).
                if shell.phase == 0 {
                    MatterState::Liquid
                } else {
                    MatterState::Solid
                },
            ),
            Kind::Mantle => (Material::Rock, MatterState::Solid),
            // Phase 0 of the isotherm's water is liquid; every other
            // phase, and all of the melting curve's, is an ice.
            Kind::Water { liquid: true } if shell.phase == 0 => {
                (Material::Water, MatterState::Liquid)
            }
            Kind::Water { .. } => (Material::Ice, MatterState::Solid),
        };
        let from = [radius.node()];
        shells.push(ShellRecord {
            outer_radius: trace.record(
                Code::of(codes::LAYER_OUTER_RADIUS),
                shell.outer_radius,
                &from,
            )?,
            mass: trace.record(Code::of(codes::LAYER_MASS), shell.mass, &from)?,
            material: trace.record(Code::of(codes::LAYER_MATERIAL), material, &from)?,
            state: trace.record(Code::of(codes::LAYER_STATE), state, &from)?,
        });
    }

    Ok(Interior {
        bulk: Bulk {
            mass: trial.mass,
            radius,
            surface_gravity,
            mean_density,
        },
        composition: Composition {
            core_mass_fraction: drawn.core_mass_fraction,
            water_fraction: drawn.water_fraction,
        },
        shells,
        structure,
    })
}
