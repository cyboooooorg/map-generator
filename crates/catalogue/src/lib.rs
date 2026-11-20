//! The catalogue: stores the inputs of each system (the only source of
//! truth) and caches what is regenerated from them (timelines, worlds),
//! with the eviction policy.
//!
//! Owns the `SystemRepository`, `TimelineCache` and `WorldCache` ports; a
//! cache is invalidated by the engine version, never migrated.
