//! The interiors of enveloped planets: a mini-Neptune (a condensed core
//! under a Lopez & Fortney 2014 hydrogen-helium envelope) and a giant (the
//! Fortney et al. 2007 tables, an ice-rock core, a metallic hydrogen layer
//! placed by an index-one polytrope). The study behind the choices is
//! `docs/superpowers/studies/2026-11-21-planet-model-2a/giants-and-envelopes.md`
//! (§2 mini-Neptunes, §1 giants, §5 the metallic hydrogen).
//!
//! Every value is recorded once, by the computation that derives it: the
//! radius, the layers and a giant's internal heat come from one call of
//! the tables each.

use super::codes;
use super::draws::Drawn;
use super::interior::{Interior, InteriorError, ShellRecord, Water, rocky_core};
use crate::Trial;
use science::constants::GRAVITATIONAL_CONSTANT;
use science::envelope::envelope_thickness;
use science::giants::{GAS_GIANT_MIN_MASS, giant, ice_rock_core_radius};
use science::hydrogen::{METALLIC_PRESSURE, polytrope_mass_within, polytrope_radius_at};
use science::units::{
    EarthFlux, EarthMass, EarthRadius, Kilogram, KilogramPerCubicMetre, Metre,
    MetrePerSecondSquared, Watt,
};
use timeline::{Bulk, Code, Material, MatterState, NodeId, Provenance, Trace, Traced};

/// Calibration: the least mass of a giant whose metallic hydrogen layer is
/// placed, 50 M_E. The index-one polytrope stands for Jupiter-like giants
/// only: it matches Debras & Chabrier 2019's Jupiter, is not validated for
/// Saturn or Neptune, and an ice giant's layer boundary is "not computed"
/// (study `giants-and-envelopes.md` §5.2). Searched: a published mass
/// above which a giant has a metallic layer; none found. The page
/// [What are planets made of?](../../../../docs/science/planet-interiors.md#giants-and-the-metallic-hydrogen-boundary),
/// section on giants, says what was searched.
const METALLIC_MIN_MASS: EarthMass = EarthMass(50.0);

/// Calibration: the largest share of a giant's mass in its core for which
/// its metallic hydrogen layer is placed, 0.25: "a planet with a large core
/// is not an n = 1 body" (study §5.2, which proposes the polytrope for a
/// core below 25 % of the mass). Searched: a published core share up to
/// which an index-one polytrope holds; none found (same page section).
const METALLIC_MAX_CORE_SHARE: f64 = 0.25;

/// Calibration: the state of a giant's ice-rock core, solid. A
/// placeholder: the state of a dilute ice-rock core at tens of Mbar is not
/// modelled until the temperature profile of the planet interior plan's
/// later pull requests arrives (the same convention as the imposed solid
/// mantle of a rocky planet). Searched: a published state of a giant's
/// core usable without a temperature profile; none found (Jupiter's may be
/// diluted over a large share of its radius, Wahl et al. 2017). The page
/// [What are planets made of?](../../../../docs/science/planet-interiors.md#giants-and-the-metallic-hydrogen-boundary),
/// section on giants, says what was searched.
const GIANT_CORE_STATE: MatterState = MatterState::Solid;

/// A planet's body: what [`super::interior::interior`] builds.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Body {
    /// A planet without an envelope.
    Rocky(Interior),
    /// A condensed core under a hydrogen-helium envelope.
    MiniNeptune {
        /// The core, `(1 - f) M`, its water a share of it: its
        /// `composition.water_fraction` is the core's share, the planet's
        /// composition for the keyframe being built by PR 2d's chain.
        core: Interior,
        envelope: EnvelopeRecord,
        /// The whole planet: its mass (without a water beyond the laws,
        /// recorded apart in the core), the core's radius plus the
        /// envelope's thickness, the gravity and density of both.
        bulk: Bulk,
    },
    /// A giant read in the Fortney et al. 2007 tables.
    Giant(GiantRecord),
}

impl Body {
    /// The bulk of the whole planet.
    pub(crate) fn bulk(&self) -> &Bulk {
        match self {
            Self::Rocky(interior) => &interior.bulk,
            Self::MiniNeptune { bulk, .. } => bulk,
            Self::Giant(record) => &record.bulk,
        }
    }

    /// The layers from the centre out: a rocky planet's shells, a
    /// mini-Neptune's core shells then its envelope, a giant's core then
    /// its hydrogen.
    pub(crate) fn layers(&self) -> Vec<&ShellRecord> {
        match self {
            Self::Rocky(interior) => interior.shells.iter().collect(),
            Self::MiniNeptune { core, envelope, .. } => core
                .shells
                .iter()
                .chain(std::iter::once(&envelope.layer))
                .collect(),
            Self::Giant(record) => record.layers.iter().collect(),
        }
    }
}

/// A mini-Neptune's envelope.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnvelopeRecord {
    /// `f M`.
    pub mass: Traced<Kilogram>,
    /// Read in the Lopez & Fortney 2014 tables.
    pub thickness: Traced<EarthRadius>,
    /// The envelope as a layer: its mass is [`Self::mass`].
    pub layer: ShellRecord,
}

/// A giant.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GiantRecord {
    pub bulk: Bulk,
    /// Radius of the ice-rock core, the outer radius of the first layer.
    pub core_radius: Traced<Metre>,
    /// The ice-rock core, then the metallic and molecular hydrogen, or one
    /// envelope layer.
    pub layers: Vec<ShellRecord>,
    /// The tables' internal luminosity, the giant's secular heat.
    pub luminosity: Traced<Watt>,
}

/// Builds an enveloped planet: a mini-Neptune below
/// [`GAS_GIANT_MIN_MASS`], a giant from it.
///
/// # Errors
///
/// The errors of the core's structure, of the tables (outside them) and
/// of the trace; [`InteriorError::NoHeavyMassRange`] for a giant drawn
/// without a heavy-element mass; [`InteriorError::NoEnvelope`] for a giant
/// whose radius is not above its core's.
pub(crate) fn enveloped(
    trial: &Trial,
    drawn: &Drawn,
    trace: &mut Trace,
) -> Result<Body, InteriorError> {
    if trial.mass.value().0 < GAS_GIANT_MIN_MASS.0 {
        mini_neptune(trial, drawn, trace)
    } else {
        gas_giant(trial, drawn, trace).map(Body::Giant)
    }
}

/// The flux the tables read: the sum over the stars, citing each one's.
fn bolometric_flux(trial: &Trial, trace: &mut Trace) -> Result<Traced<EarthFlux>, InteriorError> {
    let from: Vec<NodeId> = trial.sources.iter().map(|s| s.flux.node()).collect();
    Ok(trace.record(
        Code::of(codes::BOLOMETRIC_FLUX),
        trial.sources.total_flux(),
        &from,
    )?)
}

/// The bulk of a whole planet of `mass` and `radius`: `G M / R^2` and
/// `M / (4/3 pi R^3)`.
fn bulk_of(
    mass: Traced<EarthMass>,
    radius: Traced<EarthRadius>,
    trace: &mut Trace,
) -> Result<Bulk, InteriorError> {
    let kg = Kilogram::from(*mass.value()).0;
    let r = Metre::from(*radius.value()).0;
    let from = [mass.node(), radius.node()];
    let surface_gravity = trace.record(
        Code::of(codes::SURFACE_GRAVITY),
        MetrePerSecondSquared(GRAVITATIONAL_CONSTANT * kg / (r * r)),
        &from,
    )?;
    let volume = 4.0 / 3.0 * std::f64::consts::PI * r.powi(3);
    let mean_density = trace.record(
        Code::of(codes::MEAN_DENSITY),
        KilogramPerCubicMetre(kg / volume),
        &from,
    )?;
    Ok(Bulk {
        mass,
        radius,
        surface_gravity,
        mean_density,
    })
}

/// The outer radius of a layer that ends at the planet's `radius`.
fn outer_at(
    radius: Traced<EarthRadius>,
    trace: &mut Trace,
) -> Result<Traced<Metre>, InteriorError> {
    Ok(trace.record(
        Code::of(codes::LAYER_OUTER_RADIUS),
        Metre::from(*radius.value()),
        &[radius.node()],
    )?)
}

/// A hydrogen-helium layer: its material derived from the envelope
/// fraction, and its state either set by the engine (`by`: `None`) or read
/// from the metallic boundary.
fn envelope_layer(
    outer_radius: Traced<Metre>,
    mass: Traced<Kilogram>,
    drawn: &Drawn,
    state: MatterState,
    by: Option<NodeId>,
    trace: &mut Trace,
) -> Result<ShellRecord, InteriorError> {
    let material = trace.record(
        Code::of(codes::LAYER_MATERIAL),
        Material::Envelope,
        &[drawn.envelope_fraction.node()],
    )?;
    let state = match by {
        Some(boundary) => {
            trace.record(Code::of(codes::LAYER_STATE_BY_PRESSURE), state, &[boundary])?
        }
        None => trace.leaf(Code::of(codes::LAYER_STATE), state, Provenance::Imposed)?,
    };
    Ok(ShellRecord {
        outer_radius,
        mass,
        material,
        state,
    })
}

/// A mini-Neptune: the core `(1 - f) M` built by [`rocky_core`] (its water
/// a share of it), under the envelope thickness of Lopez & Fortney 2014 for
/// the whole planet. The radius is the core's plus the thickness; the bulk
/// is the whole mass at that radius, or the condensed planet's (the core's
/// dry mass plus the envelope) when the core's water is beyond the laws.
fn mini_neptune(trial: &Trial, drawn: &Drawn, trace: &mut Trace) -> Result<Body, InteriorError> {
    let (mass, f) = (trial.mass, drawn.envelope_fraction);
    let share = f.value().get();
    let core_mass = trace.record(
        Code::of(codes::MASS_WITHOUT_ENVELOPE),
        *mass.value() * (1.0 - share),
        &[mass.node(), f.node()],
    )?;
    let core = rocky_core(&core_mass, drawn, trial, trace)?;
    let flux = bolometric_flux(trial, trace)?;
    let thickness = trace.record(
        Code::of(codes::ENVELOPE_THICKNESS),
        envelope_thickness(*mass.value(), *f.value(), *flux.value(), *trial.age.value())?,
        &[mass.node(), f.node(), flux.node(), trial.age.node()],
    )?;
    let envelope_mass = trace.record(
        Code::of(codes::ENVELOPE_MASS),
        Kilogram::from(*mass.value()) * share,
        &[mass.node(), f.node()],
    )?;
    let radius = trace.record(
        Code::of(codes::RADIUS),
        *core.bulk.radius.value() + *thickness.value(),
        &[core.bulk.radius.node(), thickness.node()],
    )?;
    // A core whose water is beyond the laws: the bulk describes the
    // condensed planet, the core's dry mass plus the envelope, the water
    // being recorded apart (`Water::BeyondLaw`).
    let bulk_mass = if matches!(core.water, Water::BeyondLaw { .. }) {
        trace.record(
            Code::of(codes::DRY_MASS),
            *core.bulk.mass.value() + EarthMass::from(*envelope_mass.value()),
            &[core.bulk.mass.node(), envelope_mass.node()],
        )?
    } else {
        mass
    };
    let bulk = bulk_of(bulk_mass, radius, trace)?;
    let outer = outer_at(radius, trace)?;
    let layer = envelope_layer(outer, envelope_mass, drawn, MatterState::Gas, None, trace)?;
    Ok(Body::MiniNeptune {
        core,
        envelope: EnvelopeRecord {
            mass: envelope_mass,
            thickness,
            layer,
        },
        bulk,
    })
}

/// The thickness of a giant's envelope, `total - core`.
///
/// # Errors
///
/// [`InteriorError::NoEnvelope`] when it is zero or negative: the radius
/// is never clipped.
pub(crate) fn giant_envelope_thickness(
    total: EarthRadius,
    core: EarthRadius,
) -> Result<EarthRadius, InteriorError> {
    if total.0 > core.0 {
        Ok(total - core)
    } else {
        Err(InteriorError::NoEnvelope)
    }
}

/// A giant: one reading of the Fortney et al. 2007 tables for its radius
/// and internal luminosity, at its heavy-element mass as the core, and
/// their Eq. 7 for the core's radius. Its layers: the ice-rock core of the
/// heavy mass; then, for a giant from [`METALLIC_MIN_MASS`] with a core
/// below [`METALLIC_MAX_CORE_SHARE`] of its mass, the metallic hydrogen up
/// to where the planet's index-one polytrope reaches
/// [`METALLIC_PRESSURE`], holding the polytrope's mass inside that radius
/// less the core, and the molecular hydrogen above; otherwise one envelope
/// layer.
fn gas_giant(
    trial: &Trial,
    drawn: &Drawn,
    trace: &mut Trace,
) -> Result<GiantRecord, InteriorError> {
    let mass = trial.mass;
    let heavy = drawn
        .heavy_mass
        .ok_or(InteriorError::NoHeavyMassRange(mass.value().0))?;
    let flux = bolometric_flux(trial, trace)?;
    let table = giant(
        *mass.value(),
        *heavy.value(),
        *trial.age.value(),
        *flux.value(),
    )?;
    let core = ice_rock_core_radius(*heavy.value());
    giant_envelope_thickness(table.radius, core)?;

    // The tables may read another age or mass than asked: each is then a
    // node of its own, derived from the asked value, and the radius and
    // luminosity cite it instead.
    let age_read = if table.age_used == *trial.age.value() {
        trial.age.node()
    } else {
        trace
            .record(
                Code::of(codes::TABLE_AGE_USED),
                table.age_used,
                &[trial.age.node()],
            )?
            .node()
    };
    let mass_read = if table.mass_used == *mass.value() {
        mass.node()
    } else {
        trace
            .record(
                Code::of(codes::TABLE_MASS_USED),
                table.mass_used,
                &[mass.node()],
            )?
            .node()
    };
    let from = [mass_read, heavy.node(), age_read, flux.node()];
    let radius = trace.record(Code::of(codes::RADIUS), table.radius, &from)?;
    let luminosity = trace.record(Code::of(codes::GIANT_LUMINOSITY), table.luminosity, &from)?;
    let core_radius = trace.record(
        Code::of(codes::GIANT_CORE_RADIUS),
        Metre::from(core),
        &[heavy.node()],
    )?;
    let bulk = bulk_of(mass, radius, trace)?;

    let total = Kilogram::from(*mass.value());
    let heavy_kg = Kilogram::from(*heavy.value());
    let core_layer = ShellRecord {
        outer_radius: core_radius,
        mass: trace.record(Code::of(codes::LAYER_MASS), heavy_kg, &[heavy.node()])?,
        material: trace.record(
            Code::of(codes::LAYER_MATERIAL),
            Material::RockIce,
            &[heavy.node()],
        )?,
        state: trace.leaf(
            Code::of(codes::LAYER_STATE),
            GIANT_CORE_STATE,
            Provenance::Imposed,
        )?,
    };
    let mut layers = vec![core_layer];

    let planet_radius = Metre::from(table.radius);
    let metallic = (mass.value().0 > METALLIC_MIN_MASS.0
        && heavy.value().0 < METALLIC_MAX_CORE_SHARE * mass.value().0)
        .then(|| polytrope_radius_at(total, planet_radius, METALLIC_PRESSURE))
        .flatten()
        .map(|r| (r, polytrope_mass_within(total, planet_radius, r)))
        .filter(|&(r, inside)| r.0 > core_radius.value().0 && inside.0 > heavy_kg.0);
    let outer = outer_at(radius, trace)?;
    match metallic {
        Some((boundary, inside)) => {
            let boundary = trace.record(
                Code::of(codes::METALLIC_RADIUS),
                boundary,
                &[mass.node(), radius.node()],
            )?;
            let metallic_mass = trace.record(
                Code::of(codes::LAYER_MASS),
                inside - heavy_kg,
                &[mass.node(), heavy.node(), boundary.node()],
            )?;
            layers.push(envelope_layer(
                boundary,
                metallic_mass,
                drawn,
                MatterState::Liquid,
                Some(boundary.node()),
                trace,
            )?);
            let molecular_mass = trace.record(
                Code::of(codes::LAYER_MASS),
                total - inside,
                &[mass.node(), boundary.node()],
            )?;
            layers.push(envelope_layer(
                outer,
                molecular_mass,
                drawn,
                MatterState::Gas,
                Some(boundary.node()),
                trace,
            )?);
        }
        None => {
            let envelope_mass = trace.record(
                Code::of(codes::LAYER_MASS),
                total - heavy_kg,
                &[mass.node(), heavy.node()],
            )?;
            layers.push(envelope_layer(
                outer,
                envelope_mass,
                drawn,
                MatterState::Gas,
                None,
                trace,
            )?);
        }
    }
    Ok(GiantRecord {
        bulk,
        core_radius,
        layers,
        luminosity,
    })
}
