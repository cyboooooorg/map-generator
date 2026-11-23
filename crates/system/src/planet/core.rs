//! The iron core of a rocky planet and its dynamo. Ported from
//! `legacy-final:crates/mapgen/src/planet/core.rs` (`core`), the law in
//! `science::dynamo`.

use super::codes;
use super::interior::{Interior, InteriorError};
use crate::Trial;
use science::dynamo::{
    CorePhase, NUSSELT_EPISODIC, NUSSELT_LID, NUSSELT_PLATES, cooling_time, core_phase,
    surface_field,
};
use science::units::{KilogramPerCubicMetre, Metre, Tesla};
use timeline::{Code, Core, CoreState, Material, NodeId, TectonicRegime, Trace, Traced};

/// The core of a planet and whether a dynamo runs in it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CoreRecord {
    pub core: Core,
    pub dynamo: Traced<bool>,
}

/// The core of the rocky planet `trial` of interior `interior` in the
/// tectonic regime `regime`, at the trial's age; a calibration on Earth,
/// Venus, Mars, Mercury and the Moon (`science::dynamo`). The present
/// regime is taken for the whole history.
///
/// The core is the iron shells of the structure: its radius is the outer
/// radius of the outermost one, its mass the interior's core mass. Without
/// an iron shell or with no core mass, the core is
/// [`CoreState::None`], without a dynamo and with 0 T, each derived from
/// the core mass. Otherwise:
///
/// - the cooling time ([`codes::CORE_COOLING_TIME`]) derives from the
///   core's mass and radius, the planet's radius and the regime, whose
///   Nusselt number it takes ([`NUSSELT_LID`] for a stagnant lid and a
///   heat pipe);
/// - the state ([`codes::CORE_STATE`]) from the cooling time and the age;
/// - the dynamo ([`codes::CORE_DYNAMO`]) from those and the regime, plates
///   keeping the thermal dynamo running;
/// - the field ([`codes::CORE_FIELD`]), the surface equatorial dipole of
///   `science::dynamo::surface_field` for the dynamo's kind, from the
///   dynamo, the cooling time, the core's mass and radius, the planet's
///   radius and the age; 0 T from the dynamo alone when none runs.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn core(
    trial: &Trial,
    interior: &Interior,
    regime: &Traced<TectonicRegime>,
    trace: &mut Trace,
) -> Result<CoreRecord, InteriorError> {
    let iron = interior
        .shells
        .iter()
        .filter(|shell| *shell.material.value() == Material::Iron)
        .max_by(|a, b| {
            a.outer_radius
                .value()
                .0
                .total_cmp(&b.outer_radius.value().0)
        });
    let core_mass = interior.core_mass;
    let Some(iron) = iron.filter(|_| core_mass.value().0 > 0.0) else {
        let from = [core_mass.node()];
        let state = trace.record(Code::of(codes::CORE_STATE), CoreState::None, &from)?;
        let dynamo = trace.record(Code::of(codes::CORE_DYNAMO), false, &[state.node()])?;
        let field = trace.record(Code::of(codes::CORE_FIELD), Tesla(0.0), &[dynamo.node()])?;
        return Ok(CoreRecord {
            core: Core { state, field },
            dynamo,
        });
    };
    let core_radius = *iron.outer_radius.value();
    let radius = Metre::from(*interior.bulk.radius.value());
    // Definition: the volume of a sphere, (4/3) π r³.
    let volume = 4.0 / 3.0 * std::f64::consts::PI * core_radius.0.powi(3);
    let density = KilogramPerCubicMetre(core_mass.value().0 / volume);
    let mantle = Metre((radius.0 - core_radius.0).max(0.0));
    let nusselt = match regime.value() {
        TectonicRegime::Plates => NUSSELT_PLATES,
        TectonicRegime::Episodic => NUSSELT_EPISODIC,
        // A stagnant lid and a heat pipe, which cools the lithosphere, not
        // the mantle below (the contract's enum is non-exhaustive).
        _ => NUSSELT_LID,
    };
    let body: [NodeId; 3] = [
        core_mass.node(),
        iron.outer_radius.node(),
        interior.bulk.radius.node(),
    ];
    let mut from = body.to_vec();
    from.push(regime.node());
    let tau = trace.record(
        Code::of(codes::CORE_COOLING_TIME),
        cooling_time(density, core_radius, mantle, nusselt),
        &from,
    )?;
    let age = trial.age;
    let plates = *regime.value() == TectonicRegime::Plates;
    let phase = core_phase(*age.value(), *tau.value(), plates);
    let (state, kind) = match phase {
        CorePhase::Liquid { dynamo } => (CoreState::Liquid, dynamo),
        CorePhase::Frozen => (CoreState::Frozen, None),
    };
    let state = trace.record(
        Code::of(codes::CORE_STATE),
        state,
        &[tau.node(), age.node()],
    )?;
    let dynamo = trace.record(
        Code::of(codes::CORE_DYNAMO),
        kind.is_some(),
        &[tau.node(), age.node(), regime.node()],
    )?;
    let field = match kind {
        Some(kind) => {
            let mut from = vec![dynamo.node(), tau.node()];
            from.extend(body);
            from.push(age.node());
            trace.record(
                Code::of(codes::CORE_FIELD),
                surface_field(
                    density,
                    core_radius,
                    radius,
                    *age.value(),
                    *tau.value(),
                    kind,
                ),
                &from,
            )?
        }
        None => trace.record(Code::of(codes::CORE_FIELD), Tesla(0.0), &[dynamo.node()])?,
    };
    Ok(CoreRecord {
        core: Core { state, field },
        dynamo,
    })
}
