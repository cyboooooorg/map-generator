//! The tectonic regime of a planet, from its internal heat, its mass and
//! the water it kept. Ported from
//! `legacy-final:crates/mapgen/src/planet/regime.rs` (`regime`).

use super::codes;
use super::enveloped::Body;
use super::interior::InteriorError;
use crate::Trial;
use science::relief::volcanoes::heat_flux;
use science::tectonics::{HEAT_PIPE_ONSET_FLUX, PLATES_MIN_MASS};
use science::units::{Metre, Watt};
use timeline::{Code, Heat, NodeId, TectonicRegime, Trace, Traced, Water};

/// The tectonic regime of the planet `trial` of body `body`, internal
/// heat `heat` and water `water`; a calibration of the Solar System
/// (`science::tectonics`):
///
/// - a heat pipe above [`HEAT_PIPE_ONSET_FLUX`] of internal heat, the total
///   power over `4 π R²` (Io);
/// - a stagnant lid below [`PLATES_MIN_MASS`] (Mars, Mercury, the Moon);
/// - plates when the planet kept water and did not run away (Earth): a
///   runaway's steam does not weaken the lithosphere;
/// - episodic otherwise, the mass allowing plates with the water gone or
///   steam (Venus).
///
/// The regime derives from the nodes each rule read, in that order: the
/// heat and the radius, then the mass, then the water.
///
/// # Errors
///
/// The errors of the trace.
pub(crate) fn regime(
    trial: &Trial,
    body: &Body,
    heat: &Heat,
    water: &Water,
    trace: &mut Trace,
) -> Result<Traced<TectonicRegime>, InteriorError> {
    let radius = body.bulk().radius;
    let flux = heat_flux(
        Watt(heat.radiogenic.value().0 + heat.tidal.value().0 + heat.secular.value().0),
        Metre::from(*radius.value()),
    );
    let mut from: Vec<NodeId> = vec![
        heat.radiogenic.node(),
        heat.tidal.node(),
        heat.secular.node(),
        radius.node(),
    ];
    let regime = if flux.0 > HEAT_PIPE_ONSET_FLUX.0 {
        TectonicRegime::HeatPipe
    } else {
        from.push(trial.mass.node());
        if trial.mass.value().0 < PLATES_MIN_MASS.0 {
            TectonicRegime::Stagnant
        } else {
            from.extend([water.inventory.node(), water.runaway.node()]);
            if water.inventory.value().0 > 0.0 && !*water.runaway.value() {
                TectonicRegime::Plates
            } else {
                TectonicRegime::Episodic
            }
        }
    };
    Ok(trace.record(Code::of(codes::REGIME), regime, &from)?)
}
