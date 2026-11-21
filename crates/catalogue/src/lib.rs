//! The catalogue: stores the inputs of each system (the only source of
//! truth) and caches what is regenerated from them (timelines, worlds),
//! with the eviction policy.
//!
//! Owns the `SystemRepository`, `TimelineCache` and `WorldCache` ports; a
//! cache is invalidated by the engine version, never migrated.

mod evict;
mod id;
mod ports;
mod stored;

pub use evict::{CacheEntry, CacheKey, DEFAULT_BUDGET, evict};
pub use id::{CatalogueError, SystemId};
pub use ports::{SystemRepository, TimelineCache, WorldCache, WorldKey};
pub use stored::StoredSystem;

#[cfg(test)]
#[path = "catalogue_tests.rs"]
mod tests;
