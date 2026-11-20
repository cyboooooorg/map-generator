//! The system engine: computes a system from its birth to its horizon
//! (a hierarchical tree of orbits, stars with phases, global climate,
//! interiors, events) and publishes one timeline per body.
//!
//! Owns the request and archetype types (the inputs) and the
//! `EnvironmentSource` port. Never depends on the map generator.
