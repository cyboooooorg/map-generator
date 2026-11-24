//! The climate inputs of the reference bodies, for the climate's tests.

use super::{ClimateInputs, Rotation};
use crate::planet::bare::bare_with;
use crate::planet::tests::{EARTH, MARS, ReferenceBody, TITAN, reference_trial};
use science::units::{Bar, Degree, EarthFlux, Kelvin};
use timeline::{Code, Provenance, Trace};

/// Source: Earth's obliquity to its orbit, 23.44° (NASA Earth fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/earthfact.html).
pub(super) const EARTH_OBLIQUITY: Degree = Degree(23.44);

/// Source: Mars's obliquity to its orbit, 25.19° (NASA Mars fact sheet,
/// nssdc.gsfc.nasa.gov/planetary/factsheet/marsfact.html).
pub(super) const MARS_OBLIQUITY: Degree = Degree(25.19);

/// Source: Saturn's obliquity to its orbit, 26.73° (NASA Saturn fact
/// sheet, nssdc.gsfc.nasa.gov/planetary/factsheet/saturnfact.html), the
/// tilt of Titan's equator to the Sun: Titan orbits in Saturn's
/// equatorial plane, locked to Saturn and not to the Sun.
pub(super) const TITAN_OBLIQUITY: Degree = Degree(26.73);

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
    reference(&EARTH, EARTH_OBLIQUITY)
}

/// The climate inputs of the reference Mars ([`bare_with`] of [`MARS`]),
/// at Mars's obliquity, not locked, with no SO₂ frost: its bare air, not
/// the imposed one of its temperature test.
pub(super) fn mars() -> ClimateInputs {
    reference(&MARS, MARS_OBLIQUITY)
}

/// The climate inputs of the reference Titan ([`bare_with`] of
/// [`TITAN`]), at [`TITAN_OBLIQUITY`], not locked to the Sun, with no
/// SO₂ frost.
pub(super) fn titan() -> ClimateInputs {
    reference(&TITAN, TITAN_OBLIQUITY)
}

/// The climate inputs of `body` ([`bare_with`] of its
/// [`reference_trial`]) at `obliquity`, not locked, with no SO₂ frost.
fn reference(body: &ReferenceBody, obliquity: Degree) -> ClimateInputs {
    let mut trace = Trace::new();
    let (t, d) = reference_trial(&mut trace, body);
    let bare = bare_with(&t, &d, &mut trace).unwrap();
    let spin = rotation(&mut trace, obliquity, false);
    ClimateInputs::of(&bare, &spin, Bar(0.0)).expect("the body has a surface")
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
