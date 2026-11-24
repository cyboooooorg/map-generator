//! The climate inputs of the reference bodies, for the climate's tests.

use super::{ClimateInputs, Rotation};
use crate::planet::bare::bare_with;
use crate::planet::tests::{EARTH, reference_trial};
use science::units::{Bar, Degree, EarthFlux, Kelvin};
use timeline::{Code, Provenance, Trace};

/// Source: Earth's obliquity to its orbit, 23.44° (NASA Earth fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html).
pub(super) const EARTH_OBLIQUITY: Degree = Degree(23.44);

/// A rotation of `obliquity`, `locked` or not, as input leaves of
/// `trace`.
pub(super) fn rotation(trace: &mut Trace, obliquity: Degree, locked: bool) -> Rotation {
    Rotation {
        obliquity: trace
            .leaf(Code::of("test.obliquity"), obliquity, Provenance::Input)
            .unwrap(),
        locked: trace
            .leaf(Code::of("test.locked"), locked, Provenance::Input)
            .unwrap(),
    }
}

/// The climate inputs of the reference Earth at 1 au
/// ([`bare_with`] of [`EARTH`]), at Earth's obliquity, not locked, with
/// no SO₂ frost.
pub(super) fn earth() -> ClimateInputs {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, &EARTH);
    let bare = bare_with(&t, &d, &mut trace).unwrap();
    let spin = rotation(&mut trace, EARTH_OBLIQUITY, false);
    ClimateInputs::of(&bare, &spin, Bar(0.0)).expect("Earth has a surface")
}

/// `inputs` of a body at 1 au moved to `au`: each star's flux falls as
/// the inverse square of the distance and its equilibrium temperature as
/// the inverse square root (a `Definition:`, the trial's own scaling).
/// The rest of the body is kept as it was at 1 au.
pub(super) fn moved(inputs: &ClimateInputs, au: f64) -> ClimateInputs {
    let mut moved = inputs.clone();
    for light in &mut moved.sources {
        light.flux = EarthFlux(light.flux.0 / (au * au));
        light.equilibrium = Kelvin(light.equilibrium.0 / au.sqrt());
    }
    moved
}
