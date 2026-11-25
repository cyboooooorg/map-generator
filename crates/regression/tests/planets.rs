//! The planets of the shared reference systems, modelled by the planet
//! chain (`system::planet`).

use system::{ArchetypeId, draw_with, planet};
use timeline::{Code, Trace};

/// Definition: the index of the `earth-like` request in
/// `regression::reference_requests()` (seed 7).
const EARTH_LIKE_REFERENCE: usize = 6;

#[test]
fn the_earth_like_reference_has_a_surface() {
    // The archetype's promise (a definition, not a pin): an asked
    // earth-like is a planet with a surface, whatever its envelope draw
    // would have given (#695).
    let earth_like = ArchetypeId(Code::of("earth-like"));
    let request = &regression::reference_requests()[EARTH_LIKE_REFERENCE];
    let mut trace = Trace::new();
    let drawn = draw_with(request, &mut trace, planet::class_of).unwrap();
    let placement = drawn
        .placements
        .iter()
        .find(|p| p.trial.archetype.as_ref() == Some(&earth_like))
        .expect("reference 7 asks an earth-like");
    let state = planet::model(placement, &mut trace).unwrap();
    assert!(state.surface.is_some(), "{:?}", state.class);
    assert_eq!(state.envelope_fraction.value().get(), 0.0);
}
