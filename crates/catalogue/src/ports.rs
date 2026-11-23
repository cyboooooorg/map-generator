//! The ports the adapters implement: what is stored and what is cached.

use timeline::{Designation, EngineVersion, Timeline};

use crate::{StoredSystem, SystemId};

/// The key of a cached world: a cache entry is never migrated, a new
/// engine version (of the system engine or of the map generator) is a
/// new key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldKey {
    pub system: SystemId,
    pub body: Designation,
    pub keyframe: u32,
    pub grid: u32,
    /// The system engine of the timeline the world is built from.
    pub engine: EngineVersion,
    /// The version of the map generator that makes the world.
    pub world: EngineVersion,
}

/// Where the stored systems live.
pub trait SystemRepository {
    type Error: std::error::Error;
    fn put(&self, system: &StoredSystem) -> Result<(), Self::Error>;
    fn get(&self, id: SystemId) -> Result<Option<StoredSystem>, Self::Error>;
    /// The ids, sorted.
    fn list(&self) -> Result<Vec<SystemId>, Self::Error>;
    /// Whether the system existed.
    fn remove(&self, id: SystemId) -> Result<bool, Self::Error>;
}

/// Where the timelines of a system are cached, per engine version.
pub trait TimelineCache {
    type Error: std::error::Error;
    fn get(
        &self,
        system: SystemId,
        engine: EngineVersion,
    ) -> Result<Option<Vec<Timeline>>, Self::Error>;
    fn put(
        &self,
        system: SystemId,
        engine: EngineVersion,
        timelines: &[Timeline],
    ) -> Result<(), Self::Error>;
}

/// Where the generated worlds are cached.
pub trait WorldCache {
    type World;
    type Error: std::error::Error;
    fn get(&self, key: &WorldKey) -> Result<Option<Self::World>, Self::Error>;
    fn put(&self, key: &WorldKey, world: &Self::World) -> Result<(), Self::Error>;
}
