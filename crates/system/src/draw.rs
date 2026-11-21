//! The system draw. Owner: mapgen3.
//!
//! Future signature:
//!
//! ```text
//! pub fn draw(inputs: &Inputs, trace: &mut Trace) -> Result<Drawn, SystemError>
//! ```
//!
//! `Drawn` holds the star's designation, seed and `StarState`, and one
//! `Placement` per planet. `SystemError` does not exist yet.
