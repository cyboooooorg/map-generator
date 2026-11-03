//! Worlds loaded for the deep zoom tiles and the hex grid, rendered on demand
//! from the world's raw data.
//!
//! A loaded world ([`DetailWorld`]) takes about 22 bytes per base pixel
//! ([`LOADED_BYTES_PER_PIXEL`]): 0.2 GB at 4096 × 2048, 0.75 GB at
//! 8192 × 4096. The most recently used worlds stay in memory within a byte
//! budget (`MAPGEN_TILE_CACHE_MB`); the last one used is kept even when it
//! alone exceeds it. The memory of evicted worlds is given back to the
//! system before the next one loads ([`memory::release`]).
//!
//! Loads are bounded so that the budget holds while a world is being read,
//! not only once it is in memory: one world loads at a time (a world is
//! loaded once even when many requests arrive together), and before it
//! loads, the least recently used worlds are evicted until its expected
//! size (from the width and height of its `world.json`) fits the budget
//! next to the ones kept.
//!
//! A generation needs all the memory it can get: when one starts, the cache
//! is emptied, the generation waits for a load in progress to finish, and
//! [`TileCache::world`] refuses to load worlds until it ends
//! ([`TileError::Busy`]); tile and hex grid requests then answer 503 with
//! `Retry-After`, and the viewer retries them later.
use crate::memory;
use mapgen::detail::{DetailWorld, TileId};
use mapgen::export::raw_data::load_layers;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::{OnceCell, Semaphore, SemaphorePermit};

/// Default memory budget of the loaded worlds.
pub const DEFAULT_BUDGET_BYTES: usize = 1 << 30;

/// Bytes a loaded world holds per base pixel (see [`DetailWorld::bytes`]):
/// eight 16-bit layers, the biome, the lake and river flags and the 32-bit
/// water depth. Worlds of older formats, with fewer layers, take less.
pub const LOADED_BYTES_PER_PIXEL: usize = 8 * 2 + 1 + 1 + 4;

/// A loaded world, keyed by id and revision (the modification time of its
/// raw data): regenerating a world changes its revision.
struct Entry {
    id: String,
    revision: u64,
    world: Arc<OnceCell<Arc<DetailWorld>>>,
}

impl Entry {
    /// Bytes held: none until the world is loaded.
    fn bytes(&self) -> usize {
        self.world.get().map_or(0, |w| w.bytes())
    }
}

#[derive(Default)]
struct State {
    /// Most recently used last.
    entries: Vec<Entry>,
    /// Generations running: worlds are neither loaded nor kept meanwhile.
    suspended: usize,
}

pub struct TileCache {
    state: Mutex<State>,
    budget: usize,
    /// One world loads at a time, and a generation holds this while it
    /// runs: the peaks of two loads, or of a load and a generation, never
    /// add up.
    loads: Semaphore,
}

/// Why a world could not be served.
#[derive(Debug)]
pub enum TileError {
    /// A generation is running: retry later.
    Busy,
    Failed(String),
}

/// Keeps the cache suspended while it lives (see [`TileCache::suspend`]).
pub struct Suspended<'a> {
    cache: &'a TileCache,
    /// Taken once the load in progress, if any, has finished.
    _loads: Option<SemaphorePermit<'a>>,
}

impl Drop for Suspended<'_> {
    fn drop(&mut self) {
        self.cache.lock().suspended -= 1;
    }
}

impl TileCache {
    /// A cache keeping loaded worlds within `budget` bytes.
    pub fn new(budget: usize) -> Self {
        Self {
            state: Mutex::default(),
            budget,
            loads: Semaphore::new(1),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().expect("tile cache lock")
    }

    /// The loaded world for `id` at `revision`, loading it if needed.
    pub async fn world(
        &self,
        dir: &Path,
        id: &str,
        revision: u64,
    ) -> Result<Arc<DetailWorld>, TileError> {
        let cell = {
            let mut state = self.lock();
            if state.suspended > 0 {
                return Err(TileError::Busy);
            }
            let entries = &mut state.entries;
            let cell = match entries
                .iter()
                .position(|e| e.id == id && e.revision == revision)
            {
                Some(i) => entries.remove(i).world,
                None => Arc::new(OnceCell::new()),
            };
            entries.retain(|e| e.id != id);
            entries.push(Entry {
                id: id.to_string(),
                revision,
                world: cell.clone(),
            });
            cell
        };
        if let Some(world) = cell.get() {
            return Ok(world.clone());
        }
        let world = cell
            .get_or_try_init(|| async {
                let _turn = self
                    .loads
                    .acquire()
                    .await
                    .map_err(|e| TileError::Failed(e.to_string()))?;
                // A generation may have started while this load waited.
                if self.lock().suspended > 0 {
                    return Err(TileError::Busy);
                }
                let dir = dir.to_path_buf();
                let expected = expected_bytes(&dir);
                let evicted = self.make_room(&cell, expected);
                tokio::task::spawn_blocking(move || {
                    if !evicted.is_empty() {
                        drop(evicted);
                        memory::release();
                    }
                    load(&dir)
                })
                .await
                .map_err(|e| TileError::Failed(e.to_string()))?
                .map_err(TileError::Failed)
            })
            .await?
            .clone();
        self.trim();
        Ok(world)
    }

    /// Evicts the least recently used loaded worlds, other than `loading`,
    /// until `expected` more bytes fit the budget. Returns them, to be
    /// dropped off the async workers.
    fn make_room(&self, loading: &Arc<OnceCell<Arc<DetailWorld>>>, expected: usize) -> Vec<Entry> {
        let mut state = self.lock();
        let mut total: usize = state.entries.iter().map(Entry::bytes).sum();
        state
            .entries
            .extract_if(.., |e| {
                if total + expected <= self.budget || Arc::ptr_eq(&e.world, loading) {
                    return false;
                }
                let bytes = e.bytes();
                total -= bytes;
                // Worlds still waiting for their turn hold nothing yet.
                bytes > 0
            })
            .collect()
    }

    /// Evicts the least recently used worlds beyond the budget, keeping the
    /// most recent one; forgets everything while suspended (a world loaded
    /// just as a generation started).
    fn trim(&self) {
        let mut state = self.lock();
        if state.suspended > 0 {
            state.entries.clear();
            return;
        }
        let mut total = 0;
        let mut keep = state.entries.len();
        for (i, entry) in state.entries.iter().enumerate().rev() {
            let bytes = entry.bytes();
            if total + bytes > self.budget && i + 1 < state.entries.len() {
                break;
            }
            total += bytes;
            keep = i;
        }
        state.entries.drain(..keep);
    }

    /// Empties the cache and stops loading worlds until the guard drops:
    /// a generation is about to start. Waits for a load in progress to
    /// finish (the world it loads is dropped), so the generation does not
    /// start on top of it.
    pub async fn suspend(&self) -> Suspended<'_> {
        let mut guard = {
            let mut state = self.lock();
            state.suspended += 1;
            state.entries.clear();
            Suspended {
                cache: self,
                _loads: None,
            }
        };
        // Never closed; without a permit the generation still runs.
        guard._loads = self.loads.acquire().await.ok();
        // A load that finished meanwhile may have been kept before trimming.
        self.lock().entries.clear();
        guard
    }

    /// Forgets a world (deleted or regenerated).
    /// Returns whether a loaded world was dropped.
    pub fn forget(&self, id: &str) -> bool {
        let mut state = self.lock();
        let before: usize = state.entries.iter().map(Entry::bytes).sum();
        state.entries.retain(|e| e.id != id);
        state.entries.iter().map(Entry::bytes).sum::<usize>() < before
    }

    /// Ids of the worlds held, least recently used first.
    #[cfg(test)]
    fn ids(&self) -> Vec<String> {
        self.lock().entries.iter().map(|e| e.id.clone()).collect()
    }
}

/// The memory a world will take once loaded, from the size in its
/// `raw_data/world.json`; 0 when unreadable (the load then fails anyway).
fn expected_bytes(dir: &Path) -> usize {
    #[derive(serde::Deserialize)]
    struct Size {
        width: usize,
        height: usize,
    }
    std::fs::read(dir.join("raw_data").join("world.json"))
        .ok()
        .and_then(|json| serde_json::from_slice::<Size>(&json).ok())
        .map_or(0, |s| s.width * s.height * LOADED_BYTES_PER_PIXEL)
}

fn load(dir: &Path) -> Result<Arc<DetailWorld>, String> {
    load_layers(dir).map(|w| Arc::new(DetailWorld::new(w)))
}

/// Renders a tile as PNG bytes.
pub fn render_png(world: &DetailWorld, tile: TileId) -> Result<Vec<u8>, String> {
    let image = world.render_tile(tile);
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapgen::export::{ExportOptions, export_world};
    use mapgen::generation::generate;
    use mapgen::request::WorldRequest;

    /// Exports small worlds `<planet>-<seed>` under a temporary directory.
    fn worlds(name: &str, ids: &[(&str, u32)]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("mapgen-tiles-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (planet, seed) in ids {
            let request = WorldRequest {
                planet: Some((*planet).into()),
                seed: Some(*seed),
                width: Some(64),
                height: Some(32),
                ..Default::default()
            };
            let params = request.resolve(&mut rand::rng()).unwrap().params;
            let options = ExportOptions { raw_data: true };
            export_world(
                generate(&params),
                &root.join(format!("{planet}-{seed}")),
                options,
            )
            .unwrap();
        }
        root
    }

    #[tokio::test]
    async fn keeps_worlds_within_the_budget() {
        let root = worlds("budget", &[("barren", 1), ("barren", 2), ("barren", 3)]);
        let one = load(&root.join("barren-1")).unwrap().bytes();
        // Room for two worlds.
        let cache = TileCache::new(2 * one + one / 2);
        for id in ["barren-1", "barren-2", "barren-3"] {
            cache.world(&root.join(id), id, 1).await.unwrap();
        }
        assert_eq!(cache.ids(), ["barren-2", "barren-3"]);
        // The last world is kept even beyond the budget.
        let tiny = TileCache::new(1);
        tiny.world(&root.join("barren-1"), "barren-1", 1)
            .await
            .unwrap();
        assert_eq!(tiny.ids(), ["barren-1"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn a_generation_empties_the_cache_and_refuses_loads() {
        let root = worlds("suspend", &[("barren", 4)]);
        let dir = root.join("barren-4");
        let cache = TileCache::new(DEFAULT_BUDGET_BYTES);
        cache.world(&dir, "barren-4", 1).await.unwrap();
        {
            let _guard = cache.suspend().await;
            assert!(cache.ids().is_empty());
            assert!(matches!(
                cache.world(&dir, "barren-4", 1).await,
                Err(TileError::Busy)
            ));
        }
        cache.world(&dir, "barren-4", 1).await.unwrap();
        assert_eq!(cache.ids(), ["barren-4"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn estimates_the_size_of_a_world_before_loading_it() {
        let root = worlds("estimate", &[("terran", 5)]);
        let dir = root.join("terran-5");
        let world = load(&dir).unwrap();
        // River bounds aside (negligible next to the layers on full-size
        // maps), the estimate is exact for the current format.
        let rivers = world.world().rivers.len() * size_of::<[f64; 4]>();
        assert_eq!(world.bytes() - rivers, expected_bytes(&dir));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn evicts_worlds_before_loading_one() {
        let root = worlds("room", &[("barren", 5), ("barren", 6), ("barren", 7)]);
        let one = load(&root.join("barren-5")).unwrap().bytes();
        let cache = TileCache::new(2 * one + one / 2);
        for id in ["barren-5", "barren-6"] {
            cache.world(&root.join(id), id, 1).await.unwrap();
        }
        // A third world needs room for itself: the oldest goes first.
        let loading = Arc::new(OnceCell::new());
        cache.make_room(&loading, expected_bytes(&root.join("barren-7")));
        assert_eq!(cache.ids(), ["barren-6"]);
        // A world larger than the budget empties it, and is kept alone.
        cache.make_room(&loading, 3 * one);
        assert!(cache.ids().is_empty());
        let tiny = TileCache::new(1);
        tiny.world(&root.join("barren-5"), "barren-5", 1)
            .await
            .unwrap();
        tiny.world(&root.join("barren-6"), "barren-6", 1)
            .await
            .unwrap();
        assert_eq!(tiny.ids(), ["barren-6"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn a_generation_waits_for_the_load_in_progress() {
        let cache = TileCache::new(DEFAULT_BUDGET_BYTES);
        let load = cache.loads.acquire().await.unwrap();
        let suspend = cache.suspend();
        tokio::pin!(suspend);
        let waited = tokio::time::timeout(std::time::Duration::from_millis(50), &mut suspend).await;
        assert!(waited.is_err(), "the generation started during a load");
        // Requests are refused at once meanwhile.
        assert!(matches!(
            cache.world(Path::new("nowhere"), "barren-1", 1).await,
            Err(TileError::Busy)
        ));
        drop(load);
        let guard = suspend.await;
        drop(guard);
        assert_eq!(cache.lock().suspended, 0);
    }
}
