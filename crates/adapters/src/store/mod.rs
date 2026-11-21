//! The filesystem store: the systems (the only truth) and the cache of
//! their timelines, laid out as
//!
//! ```text
//! <root>/systems/<id>/inputs.json
//! <root>/cache/<engine>/<id>/timelines.json
//! <root>/cache/<engine>/<id>/worlds/<slug>/<keyframe>-<grid>.world
//! <root>/cache/uses.json
//! ```
//!
//! Every write is staged in `<file>.tmp` then renamed, so a reader never
//! sees half a file. A cache file that cannot be read or parsed is a
//! miss, never an error: the cache is regenerated from the inputs.

mod paths;
mod uses;

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use catalogue::{
    CacheEntry, CacheKey, CatalogueError, StoredSystem, SystemId, SystemRepository, TimelineCache,
};
use timeline::{EngineVersion, Timeline};

use uses::Uses;

/// What the store can fail on.
#[derive(Debug)]
pub enum StoreError {
    /// A file or directory could not be read or written.
    Io(io::Error),
    /// A stored system is not valid JSON of the expected shape.
    Json(serde_json::Error),
    /// The catalogue refused what was stored.
    Catalogue(CatalogueError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(error) => write!(f, "io({error})"),
            StoreError::Json(error) => write!(f, "json({error})"),
            StoreError::Catalogue(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> StoreError {
        StoreError::Io(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> StoreError {
        StoreError::Json(error)
    }
}

impl From<CatalogueError> for StoreError {
    fn from(error: CatalogueError) -> StoreError {
        StoreError::Catalogue(error)
    }
}

/// The store on a directory, with the byte budget of its caches.
#[derive(Clone, Debug)]
pub struct FsStore {
    root: PathBuf,
    budget: u64,
}

/// Writes `bytes` to `path` through `<path>.tmp` and a rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut staging = path.as_os_str().to_owned();
    staging.push(".tmp");
    let staging = PathBuf::from(staging);
    fs::write(&staging, bytes)?;
    fs::rename(&staging, path)
}

/// Reads `path`, `None` when it does not exist.
fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The subdirectories of `dir` with their names; empty when `dir` is missing.
fn subdirs(dir: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Ok(name) = entry.file_name().into_string()
        {
            found.push((name, entry.path()));
        }
    }
    found.sort();
    Ok(found)
}

impl FsStore {
    /// A store rooted at `root`, whose caches may hold `budget` bytes.
    pub fn new(root: PathBuf, budget: u64) -> FsStore {
        FsStore { root, budget }
    }

    /// The ledger; a missing or corrupt one starts afresh (a use is only
    /// a hint for eviction).
    fn load_uses(&self) -> Uses {
        read_optional(&paths::uses_file(&self.root))
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save_uses(&self, uses: &Uses) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec(uses)?;
        write_atomic(&paths::uses_file(&self.root), &bytes)?;
        Ok(())
    }

    fn touch(&self, key: String) -> Result<(), StoreError> {
        let mut uses = self.load_uses();
        uses.touch(key);
        self.save_uses(&uses)
    }

    /// The cache entries on disk, with their sizes and last use. World
    /// files are listed once the world cache exists.
    fn entries(&self, uses: &Uses) -> Result<Vec<CacheEntry>, StoreError> {
        let mut entries = Vec::new();
        for (engine_name, engine_dir) in subdirs(&paths::cache_dir(&self.root))? {
            let Ok(engine) = engine_name.parse::<u32>().map(EngineVersion) else {
                continue;
            };
            for (id_name, id_dir) in subdirs(&engine_dir)? {
                let Ok(system) = id_name.parse::<SystemId>() else {
                    continue;
                };
                let file = id_dir.join("timelines.json");
                if let Ok(meta) = fs::metadata(&file) {
                    let key = paths::timelines_use_key(engine, system);
                    entries.push(CacheEntry {
                        key: CacheKey::Timelines { system, engine },
                        bytes: meta.len(),
                        last_use: uses.last(&key),
                    });
                }
            }
        }
        Ok(entries)
    }

    /// Deletes the cache entries that the policy picks for the engine
    /// version `current` and this store's budget, and the directories
    /// they leave empty. Returns the keys it removed, in policy order.
    pub fn evict(&self, current: EngineVersion) -> Result<Vec<CacheKey>, StoreError> {
        let mut uses = self.load_uses();
        let entries = self.entries(&uses)?;
        let gone = catalogue::evict(&entries, current, self.budget);
        for key in &gone {
            match key {
                CacheKey::Timelines { system, engine } => {
                    let file = paths::timelines_file(&self.root, *engine, *system);
                    match fs::remove_file(&file) {
                        Ok(()) => {}
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error.into()),
                    }
                    uses.uses
                        .remove(&paths::timelines_use_key(*engine, *system));
                    // Only succeeds on an empty directory: that is the point.
                    let dir = paths::entry_dir(&self.root, *engine, *system);
                    let _ = fs::remove_dir(&dir);
                    if let Some(engine_dir) = dir.parent() {
                        let _ = fs::remove_dir(engine_dir);
                    }
                }
                // World files are not listed yet, so none is ever picked.
                CacheKey::World(_) => {}
            }
        }
        if !gone.is_empty() {
            self.save_uses(&uses)?;
        }
        Ok(gone)
    }
}

impl SystemRepository for FsStore {
    type Error = StoreError;

    fn put(&self, system: &StoredSystem) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec_pretty(system)?;
        write_atomic(&paths::inputs_file(&self.root, system.id), &bytes)?;
        Ok(())
    }

    fn get(&self, id: SystemId) -> Result<Option<StoredSystem>, StoreError> {
        let Some(bytes) = read_optional(&paths::inputs_file(&self.root, id))? else {
            return Ok(None);
        };
        let system: StoredSystem = serde_json::from_slice(&bytes)?;
        system.check_schema()?;
        Ok(Some(system))
    }

    fn list(&self) -> Result<Vec<SystemId>, StoreError> {
        let mut ids: Vec<SystemId> = subdirs(&paths::systems_dir(&self.root))?
            .into_iter()
            .filter_map(|(name, _)| name.parse().ok())
            .collect();
        ids.sort();
        Ok(ids)
    }

    fn remove(&self, id: SystemId) -> Result<bool, StoreError> {
        let dir = paths::systems_dir(&self.root).join(id.to_string());
        match fs::remove_dir_all(&dir) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}

impl TimelineCache for FsStore {
    type Error = StoreError;

    fn get(
        &self,
        system: SystemId,
        engine: EngineVersion,
    ) -> Result<Option<Vec<Timeline>>, StoreError> {
        let file = paths::timelines_file(&self.root, engine, system);
        let timelines = read_optional(&file)
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice::<Vec<Timeline>>(&bytes).ok());
        if timelines.is_some() {
            self.touch(paths::timelines_use_key(engine, system))?;
        }
        Ok(timelines)
    }

    fn put(
        &self,
        system: SystemId,
        engine: EngineVersion,
        timelines: &[Timeline],
    ) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec(timelines)?;
        write_atomic(&paths::timelines_file(&self.root, engine, system), &bytes)?;
        self.touch(paths::timelines_use_key(engine, system))
    }
}
