//! The eviction policy of the caches.

use timeline::EngineVersion;

use crate::{SystemId, WorldKey};

/// Definition: the default byte budget of the caches (world and
/// catalogue design; the user sets it with --cache-budget).
pub const DEFAULT_BUDGET: u64 = 2 * 1024 * 1024 * 1024;

/// What a cache entry is keyed by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheKey {
    /// The timelines of a system for an engine version.
    Timelines {
        system: SystemId,
        engine: EngineVersion,
    },
    /// A generated world.
    World(WorldKey),
}

/// The engine versions a cache is current for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Engines {
    /// The system engine, which makes the timelines.
    pub system: EngineVersion,
    /// The map generator, which makes the worlds.
    pub world: EngineVersion,
}

impl CacheKey {
    /// The engine version that produced the entry.
    pub fn engine(&self) -> EngineVersion {
        match self {
            CacheKey::Timelines { engine, .. } => *engine,
            CacheKey::World(key) => key.engine,
        }
    }

    /// Whether the entry was made by the engines `current`: timelines
    /// depend on the system engine, worlds on both.
    pub fn is_current(&self, current: Engines) -> bool {
        match self {
            CacheKey::Timelines { engine, .. } => *engine == current.system,
            CacheKey::World(key) => key.engine == current.system && key.world == current.world,
        }
    }
}

/// One cached entry: its size and when it was last used (a counter the
/// adapter maintains, larger is more recent).
#[derive(Clone, Debug, PartialEq)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub bytes: u64,
    pub last_use: u64,
}

/// The keys to delete: every entry of another engine version (system or world) (a cache
/// is never migrated), then the current ones from the least recently
/// used (ties by key order) until the bytes left are at most `budget`.
/// Deterministic: the result depends only on the arguments.
pub fn evict(entries: &[CacheEntry], current: Engines, budget: u64) -> Vec<CacheKey> {
    let (mut kept, stale): (Vec<&CacheEntry>, Vec<&CacheEntry>) =
        entries.iter().partition(|e| e.key.is_current(current));
    let mut stale = stale;
    stale.sort_by(|a, b| a.key.cmp(&b.key));
    kept.sort_by(|a, b| a.last_use.cmp(&b.last_use).then_with(|| a.key.cmp(&b.key)));

    let mut left: u64 = kept.iter().fold(0, |sum, e| sum.saturating_add(e.bytes));
    let mut gone: Vec<CacheKey> = stale.iter().map(|e| e.key.clone()).collect();
    for entry in kept {
        if left <= budget {
            break;
        }
        left = left.saturating_sub(entry.bytes);
        gone.push(entry.key.clone());
    }
    gone
}

#[cfg(test)]
#[path = "evict_tests.rs"]
mod tests;
