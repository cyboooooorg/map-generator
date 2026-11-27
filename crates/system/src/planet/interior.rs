//! The interior of a planet: its layers from the composition, the
//! hydrostatic structure from `science`, every value recorded in the trace.
//!
//! [`interior`] dispatches on the envelope: a planet without one is
//! [`rocky`]; an enveloped planet (a mini-Neptune or a giant) is built in
//! [`super::enveloped`], a mini-Neptune's core by [`rocky_core`], the same structure.
//!
//! The drawn water fraction is the planet's bulk water. A dry planet's
//! water stays in its mantle (hydrated silicate; its ocean comes from
//! outgassing, in the planet interior plan's second pull request). Only a
//! water world (formed beyond the ice line or migrated from there) has a
//! water layer; a water world at 355 K or more, past the water laws, keeps
//! its water as an inventory outside the structure ([`rocky`]). A steam
//! world, whose climate boiled its water, takes the steam envelope of
//! [`steam_interior`] on top of its rock (legacy `steam_interior`, Turbet
//! et al. 2020): the water is then [`Water::Steam`], a layer of the bulk.
//!
//! The temperature profile and the state of the layers that depend on it
//! (a partly molten mantle, a molten or frozen core) come with the climate,
//! in the second pull request: the mantle is `Solid` here, and the core's
//! and the water's states are the phases of their equations of state,
//! which pressure selects.

use super::codes;
use super::draws::Drawn;
use super::enveloped::{Body, enveloped};
use crate::Trial;
use science::constants::GRAVITATIONAL_CONSTANT;
use science::envelope::EnvelopeError;
use science::eos::{EosError, MaterialEos, core_alloy, mantle, water, water_on_melting_curve};
use science::giants::GiantError;
use science::steam::envelope as steam_envelope;
use science::stellar::StellarError;
use science::structure::{LayerSpec, Structure, StructureError, integrate};
use science::units::{
    EarthMass, EarthRadius, Fraction, FractionOutOfRange, Kelvin, Kilogram, KilogramPerCubicMetre,
    Metre, MetrePerSecondSquared, Pascal,
};
use science::water::V_VI_LIQUID_TEMPERATURE;
use timeline::{
    Bulk, Code, Composition, ContractError, Material, MatterState, NodeId, Provenance, Trace,
    Traced,
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

/// Where a planet's water is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Water {
    /// Bound in the mantle (a dry planet): its mass is in the mantle's.
    InMantle,
    /// A layer of the structure (a water world below 355 K).
    Layer {
        mass: Traced<Kilogram>,
        temperature: Traced<Kelvin>,
    },
    /// Outside the structure, recorded apart from the condensed planet (a
    /// water world at 355 K or more, past the water laws).
    BeyondLaw {
        mass: Traced<Kilogram>,
        temperature: Traced<Kelvin>,
    },
    /// The steam envelope of a steam world, a layer counted in the bulk.
    Steam { mass: Traced<Kilogram> },
}

/// The interior of a planet in hydrostatic equilibrium.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Interior {
    pub bulk: Bulk,
    /// The fractions the structure is built with. For a mini-Neptune's
    /// core, `water_fraction` is the core's share, of `(1 - f) M`; the
    /// planet's composition for the keyframe is built by PR 2d's chain.
    pub composition: Composition,
    pub core_mass: Traced<Kilogram>,
    /// The silicate mantle, the mass the heat sources scale with.
    pub mantle_mass: Traced<Kilogram>,
    pub water: Water,
    /// The structure node: the central pressure of the integration.
    pub central_pressure: Traced<Pascal>,
    pub shells: Vec<ShellRecord>,
    pub structure: Structure,
}

/// Why the planet chain could not compute a planet: its interior, its
/// heat, its envelope or a trace error. A variant of the system's error,
/// `SystemError::Planet`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum InteriorError {
    Contract(ContractError),
    Structure(StructureError),
    Eos(EosError),
    NotAFraction(FractionOutOfRange),
    /// A gas giant of this mass (M_E) has no heavy-element mass in the
    /// range its tables and Thorngren et al. 2016's fit share.
    NoHeavyMassRange(f64),
    /// A giant whose total radius is not above its core's: no room for an
    /// envelope.
    NoEnvelope,
    Giant(GiantError),
    Envelope(EnvelopeError),
    /// A star's high-energy history, which the methane photolysis reads,
    /// has no answer at the trial's age (the draw places main-sequence
    /// stars only, so it is not expected).
    Stellar(StellarError),
}

impl std::fmt::Display for InteriorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contract(error) => write!(f, "interior-contract({error})"),
            Self::Structure(error) => write!(f, "interior-structure({error})"),
            Self::Eos(error) => write!(f, "interior-eos({error})"),
            Self::NotAFraction(error) => write!(f, "interior-{error}"),
            Self::NoHeavyMassRange(mass) => write!(f, "interior-no-heavy-mass-range({mass})"),
            Self::NoEnvelope => write!(f, "interior-no-envelope"),
            Self::Giant(error) => write!(f, "interior-{error}"),
            Self::Envelope(error) => write!(f, "interior-{error}"),
            Self::Stellar(error) => write!(f, "interior-stellar({error})"),
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

impl From<GiantError> for InteriorError {
    fn from(error: GiantError) -> Self {
        Self::Giant(error)
    }
}

impl From<EnvelopeError> for InteriorError {
    fn from(error: EnvelopeError) -> Self {
        Self::Envelope(error)
    }
}

impl From<StellarError> for InteriorError {
    fn from(error: StellarError) -> Self {
        Self::Stellar(error)
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

/// Builds the body of `trial` from the composition `drawn`: [`rocky`] at
/// the trial's mass when the planet has no envelope, [`enveloped`]
/// otherwise.
///
/// # Errors
///
/// The errors of [`rocky`] and [`enveloped`].
pub(crate) fn interior(
    trial: &Trial,
    drawn: &Drawn,
    trace: &mut Trace,
) -> Result<Body, InteriorError> {
    if drawn.envelope_fraction.value().get() == 0.0 {
        Ok(Body::Rocky(rocky(&trial.mass, drawn, trial, trace)?))
    } else {
        enveloped(trial, drawn, trace)
    }
}

/// Builds the condensed interior of `mass` (the trial's mass, or a
/// mini-Neptune's mass without its envelope) from the composition `drawn`,
/// integrated at zero surface pressure. Layers from the centre out, `M`
/// being `mass`:
///
/// - Fe-alloy core, `cmf (1 - w) M`;
/// - rock mantle, `(1 - cmf)(1 - w) M`, plus `w M` for a dry planet (its
///   water bound in the silicate);
/// - for a water world, water `w M` on the isotherm of the trial's
///   equilibrium temperature (its own node), which stands for the surface
///   temperature until the climate replaces it: along the isotherm in
///   [273.31 K, 355 K), on the melting curve when colder. At 355 K or more
///   (or within 0.1 mK below, where the water laws leave ice VI no range),
///   the water is an inventory outside the structure, which then holds
///   `(1 - w) M`: the surface gravity and the mean density are those of
///   the condensed planet under it.
///
/// The structure node (the central pressure) derives from the mass, the
/// two fractions and the water temperature when it is used; the radius
/// and every shell derive from it, each shell also from its layer's mass.
///
/// # Errors
///
/// The errors of the structure, of the equations of state and of the
/// trace.
pub(crate) fn rocky(
    mass: &Traced<EarthMass>,
    drawn: &Drawn,
    trial: &Trial,
    trace: &mut Trace,
) -> Result<Interior, InteriorError> {
    condensed(mass, drawn, trial, trace, codes::RADIUS)
}

/// [`rocky`] for a mini-Neptune's core: the same structure, its radius
/// recorded as [`codes::RADIUS_WITHOUT_ENVELOPE`] so the trace tells it
/// from the planet's.
///
/// # Errors
///
/// Those of [`rocky`].
pub(crate) fn rocky_core(
    mass: &Traced<EarthMass>,
    drawn: &Drawn,
    trial: &Trial,
    trace: &mut Trace,
) -> Result<Interior, InteriorError> {
    condensed(mass, drawn, trial, trace, codes::RADIUS_WITHOUT_ENVELOPE)
}

/// The body of [`rocky`], its radius recorded under `radius_code`.
fn condensed(
    mass: &Traced<EarthMass>,
    drawn: &Drawn,
    trial: &Trial,
    trace: &mut Trace,
    radius_code: &'static str,
) -> Result<Interior, InteriorError> {
    let total = Kilogram::from(*mass.value()).0;
    let cmf = drawn.core_mass_fraction.value().get();
    let w = drawn.water_fraction.value().get();
    let fractions = [
        mass.node(),
        drawn.core_mass_fraction.node(),
        drawn.water_fraction.node(),
    ];
    let mut temperature_node = None;

    let core_mass = trace.record(
        Code::of(codes::CORE_MASS),
        Kilogram(cmf * (1.0 - w) * total),
        &fractions,
    )?;
    let silicate = (1.0 - cmf) * (1.0 - w) * total;
    let mantle_mass = if drawn.is_water_world() {
        silicate
    } else {
        silicate + w * total
    };
    let mantle_mass = trace.record(
        Code::of(codes::MANTLE_MASS),
        Kilogram(mantle_mass),
        &fractions,
    )?;

    let mut layers: Vec<(Kind, MaterialEos, Traced<Kilogram>)> = vec![
        (Kind::Core, core_alloy(), core_mass),
        (Kind::Mantle, mantle(), mantle_mass),
    ];
    let mut water_state = Water::InMantle;
    if drawn.is_water_world() {
        let sources: Vec<NodeId> = trial.sources.iter().map(|s| s.equilibrium.node()).collect();
        let t = trial.sources.equilibrium_k();
        let temperature = trace.record(Code::of(codes::WATER_TEMPERATURE), t, &sources)?;
        let mut from = fractions.to_vec();
        from.push(temperature.node());
        // The structure depends on the water temperature whenever the
        // water is a layer; a water beyond the law is cited too, as the
        // reason it is not one.
        temperature_node = Some(temperature.node());
        match water_law(t)? {
            Some((eos, liquid)) => {
                let mass = trace.record(
                    Code::of(codes::WATER_LAYER_MASS),
                    Kilogram(w * total),
                    &from,
                )?;
                layers.push((Kind::Water { liquid }, eos, mass));
                water_state = Water::Layer { mass, temperature };
            }
            None => {
                let mass = trace.record(
                    Code::of(codes::WATER_BEYOND_LAW),
                    Kilogram(w * total),
                    &from,
                )?;
                water_state = Water::BeyondLaw { mass, temperature };
            }
        }
    }
    let specs: Vec<LayerSpec<'_>> = layers
        .iter()
        .map(|(_, eos, mass)| LayerSpec {
            mass: *mass.value(),
            material: eos,
        })
        .collect();
    let structure = integrate(&specs, SURFACE_PRESSURE)?;
    let condensed: f64 = layers.iter().map(|(_, _, mass)| mass.value().0).sum();
    let layer_nodes: Vec<NodeId> = layers.iter().map(|(_, _, mass)| mass.node()).collect();
    let mut inputs: Vec<NodeId> = fractions.to_vec();
    inputs.extend(&layer_nodes);
    inputs.extend(temperature_node);

    let central_pressure = trace.record(
        Code::of(codes::CENTRAL_PRESSURE),
        structure.samples[0].pressure,
        &inputs,
    )?;
    // A planet whose water is beyond the laws is described by its condensed
    // planet: the bulk mass is the dry mass, the water mass is apart.
    let bulk_mass = if matches!(water_state, Water::BeyondLaw { .. }) {
        trace.record(
            Code::of(codes::DRY_MASS),
            EarthMass::from(Kilogram(condensed)),
            &[mass.node(), drawn.water_fraction.node()],
        )?
    } else {
        *mass
    };
    let radius = trace.record(
        Code::of(radius_code),
        EarthRadius::from(structure.radius),
        &[central_pressure.node()],
    )?;
    let mut from = layer_nodes.clone();
    from.push(radius.node());
    let g = GRAVITATIONAL_CONSTANT * condensed / structure.radius.0.powi(2);
    let surface_gravity = trace.record(
        Code::of(codes::SURFACE_GRAVITY),
        MetrePerSecondSquared(g),
        &from,
    )?;
    let volume = 4.0 / 3.0 * std::f64::consts::PI * structure.radius.0.powi(3);
    let mean_density = trace.record(
        Code::of(codes::MEAN_DENSITY),
        KilogramPerCubicMetre(condensed / volume),
        &from,
    )?;

    let mut shells = Vec::with_capacity(structure.shells.len());
    for shell in &structure.shells {
        let (kind, _, layer_mass) = &layers[shell.layer];
        let (material, state, state_code) = match kind {
            // Phase 0 of the alloy is the liquid outer core, phase 1 the
            // solid inner core, switched at PREM's inner-core boundary
            // pressure (Valencia et al. 2006 fit): a phase by pressure, not
            // a melting state.
            Kind::Core => (
                Material::Iron,
                if shell.phase == 0 {
                    MatterState::Liquid
                } else {
                    MatterState::Solid
                },
                codes::LAYER_STATE_BY_PRESSURE,
            ),
            Kind::Mantle => (Material::Rock, MatterState::Solid, codes::LAYER_STATE),
            // Phase 0 of the isotherm's water is liquid; every other phase,
            // and all of the melting curve's, is an ice. At the surface
            // (0 Pa, below the vapour pressure of water at any temperature)
            // the liquid would boil: the vapour and the surface pressure
            // belong to the atmosphere of the second pull request, so the
            // state here is the phase of the equation of state.
            Kind::Water { liquid: true } if shell.phase == 0 => (
                Material::Water,
                MatterState::Liquid,
                codes::LAYER_STATE_BY_PRESSURE,
            ),
            Kind::Water { .. } => (
                Material::Ice,
                MatterState::Solid,
                codes::LAYER_STATE_BY_PRESSURE,
            ),
        };
        let from = [central_pressure.node(), layer_mass.node()];
        shells.push(ShellRecord {
            outer_radius: trace.record(
                Code::of(codes::LAYER_OUTER_RADIUS),
                shell.outer_radius,
                &from,
            )?,
            mass: trace.record(Code::of(codes::LAYER_MASS), shell.mass, &from)?,
            material: trace.record(Code::of(codes::LAYER_MATERIAL), material, &from)?,
            state: if matches!(kind, Kind::Mantle) {
                // An engine constant until the temperatures of the second
                // pull request: no node it derives from.
                trace.leaf(Code::of(state_code), state, Provenance::Imposed)?
            } else {
                trace.record(Code::of(state_code), state, &from)?
            },
        });
    }

    Ok(Interior {
        bulk: Bulk {
            mass: bulk_mass,
            radius,
            surface_gravity,
            mean_density,
        },
        composition: Composition {
            core_mass_fraction: drawn.core_mass_fraction,
            water_fraction: drawn.water_fraction,
        },
        core_mass,
        mantle_mass,
        water: water_state,
        central_pressure,
        shells,
        structure,
    })
}

/// The water law of a water world at `t`, with whether its first phase is
/// liquid: the melting curve below 273.31 K, the isotherm up to 355 K, and
/// `None` where the water laws end (the isotherm's
/// [`EosError::TemperatureOutOfRange`]: 355 K or more, or the last 0.1 mK
/// below it where ice VI has no range).
fn water_law(t: Kelvin) -> Result<Option<(MaterialEos, bool)>, InteriorError> {
    if t.0 < V_VI_LIQUID_TEMPERATURE.0 {
        return Ok(Some((water_on_melting_curve(), false)));
    }
    match water(t) {
        Ok(eos) => Ok(Some((eos, true))),
        Err(EosError::TemperatureOutOfRange { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// The interior of a steam world (legacy `interior::steam_interior`): the
/// iron and rock shells of `rock` under a steam envelope of `steam`, its
/// thickness from [`steam_envelope`] (Turbet et al. 2020) for the core of
/// those shells, the steam's share of core and steam as the water share,
/// and the sum of `trial`'s fluxes. `hides` (the steam hides the surface)
/// sets the envelope's shell as gas. Every steam node is recorded from
/// the one call.
///
/// The water layer of `rock`, if any, gives way to the steam, the water
/// the runaway kept. The core is the iron and rock shells, its mass the
/// whole less the steam (so that Turbet's `x` is the steam's share of the
/// whole), its radii the condensed structure's. A dry body's water is in
/// its rock shells: each gives up its share, its mass scaled by `1 -
/// steam / (the rock shells' mass)`, so the shells add up to the whole. The bulk is
/// the core's shells and the steam (the whole mass of a dry body) at
/// the radius of the core plus the thickness. `rock` is returned as it is
/// when `steam` is 0.
///
/// # Errors
///
/// A steam beyond what the rock shells hold, and the errors of the
/// trace.
pub(crate) fn steam_interior(
    rock: &Interior,
    steam: Traced<Kilogram>,
    trial: &Trial,
    hides: Traced<bool>,
    trace: &mut Trace,
) -> Result<Interior, InteriorError> {
    let steam_kg = steam.value().0;
    if steam_kg <= 0.0 {
        return Ok(rock.clone());
    }
    let is_core =
        |shell: &&ShellRecord| matches!(shell.material.value(), Material::Iron | Material::Rock);
    let mut core: Vec<ShellRecord> = rock.shells.iter().filter(is_core).cloned().collect();
    // A dry body's rock shells hold the water: they give up the steam.
    if rock.water == Water::InMantle {
        let held: f64 = core
            .iter()
            .filter(|s| *s.material.value() == Material::Rock)
            .map(|s| s.mass.value().0)
            .sum();
        let given = Fraction::try_from(steam_kg / held).map_err(InteriorError::NotAFraction)?;
        let kept = 1.0 - given.get();
        for shell in core
            .iter_mut()
            .filter(|s| *s.material.value() == Material::Rock)
        {
            let from = [shell.mass.node(), steam.node()];
            shell.mass = trace.record(
                Code::of(codes::LAYER_MASS),
                Kilogram(shell.mass.value().0 * kept),
                &from,
            )?;
        }
    }
    let top = core.last().ok_or(InteriorError::NoEnvelope)?;
    let core_radius = *top.outer_radius.value();
    let core_mass = Kilogram(core.iter().map(|s| s.mass.value().0).sum());
    let share = Fraction::try_from(steam_kg / (core_mass.0 + steam_kg))
        .map_err(InteriorError::NotAFraction)?;
    let one_call = steam_envelope(core_mass, core_radius, share, trial.sources.total_flux());
    let mut from: Vec<NodeId> = core.iter().map(|s| s.mass.node()).collect();
    from.push(top.outer_radius.node());
    from.push(steam.node());
    from.extend(trial.sources.iter().map(|s| s.flux.node()));
    trace.record(
        Code::of(codes::STEAM_TEMPERATURE),
        one_call.temperature,
        &from,
    )?;
    let thickness = trace.record(Code::of(codes::STEAM_THICKNESS), one_call.thickness, &from)?;

    // The whole mass is the core's shells and the steam: a water layer or
    // a water beyond the laws gives way to the steam, and what escape took
    // is gone. For a dry body it is the bulk's mass, the shells having
    // given up the steam.
    let mut whole_from: Vec<NodeId> = core.iter().map(|s| s.mass.node()).collect();
    whole_from.push(steam.node());
    let whole = Kilogram(core_mass.0 + steam_kg);
    let mass = trace.record(
        Code::of(codes::MASS_WITH_STEAM),
        EarthMass::from(whole),
        &whole_from,
    )?;
    // The mantle of a dry body gave up the steam: its mass again, from its
    // scaled shells.
    let mantle_mass = if rock.water == Water::InMantle {
        let rock_shells: Vec<&ShellRecord> = core
            .iter()
            .filter(|s| *s.material.value() == Material::Rock)
            .collect();
        trace.record(
            Code::of(codes::MANTLE_MASS),
            Kilogram(rock_shells.iter().map(|s| s.mass.value().0).sum()),
            &rock_shells
                .iter()
                .map(|s| s.mass.node())
                .collect::<Vec<_>>(),
        )?
    } else {
        rock.mantle_mass
    };
    let outer = Metre(core_radius.0 + thickness.value().0);
    let radius = trace.record(
        Code::of(codes::RADIUS_STEAM),
        EarthRadius::from(outer),
        &[top.outer_radius.node(), thickness.node()],
    )?;
    let from = [mass.node(), radius.node()];
    let gravity = trace.record(
        Code::of(codes::SURFACE_GRAVITY),
        MetrePerSecondSquared(GRAVITATIONAL_CONSTANT * whole.0 / outer.0.powi(2)),
        &from,
    )?;
    let density = trace.record(
        Code::of(codes::MEAN_DENSITY),
        KilogramPerCubicMetre(whole.0 / (4.0 / 3.0 * std::f64::consts::PI * outer.0.powi(3))),
        &from,
    )?;

    let layer = [radius.node(), steam.node()];
    core.push(ShellRecord {
        outer_radius: trace.record(Code::of(codes::LAYER_OUTER_RADIUS), outer, &layer)?,
        mass: trace.record(
            Code::of(codes::LAYER_MASS),
            steam.value().to_owned(),
            &layer,
        )?,
        material: trace.record(Code::of(codes::LAYER_MATERIAL), Material::Steam, &layer)?,
        state: trace.record(
            Code::of(codes::LAYER_STATE),
            MatterState::Gas,
            &[hides.node()],
        )?,
    });
    Ok(Interior {
        bulk: Bulk {
            mass,
            radius,
            surface_gravity: gravity,
            mean_density: density,
        },
        mantle_mass,
        water: Water::Steam { mass: steam },
        shells: core,
        ..rock.clone()
    })
}
