//! The engine entry point.
//!
//! Future signature:
//!
//! ```text
//! pub fn generate(inputs: &Inputs) -> Result<Vec<Timeline>, SystemError>
//! ```
//!
//! One `Trace` for the system, one timeline per body. `SystemError`
//! does not exist yet.
