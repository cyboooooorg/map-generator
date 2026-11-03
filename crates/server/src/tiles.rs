//! Worlds loaded for the deep zoom tiles and the hex grid, rendered on demand
//! from the world's raw data.
//!
//! A loaded world ([`DetailWorld`]) takes about 23 bytes per base pixel:
//! 0.2 GB at 4096 × 2048, 0.75 GB at 8192 × 4096. The most recently used
//! worlds stay in memory within a byte budget (`MAPGEN_TILE_CACHE_MB`); the
//! last one used is kept even when it alone exceeds it. A world is loaded
//! once even when many requests arrive together.
//!
//! A generation needs all the memory it can get: while one runs, the cache
//! is emptied and [`TileCache::world`] refuses to load worlds
//! ([`TileError::Busy`]); tile and hex grid requests then answer 503 with
//! `Retry-After`, and the viewer retries its tiles later.
use mapgen::detail::{DetailWorld, TileId};
use mapgen::export::raw_data::load_layers;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

/// Default memory budget of the loaded worlds.
pub const DEFAULT_BUDGET_BYTES: usize = 1 << 30;

/// A loaded world, keyed by id and revision (the modification time of its
/// raw data): regenerating a world changes its revision.
struct Entry {
    id: String,
    revision: u64,
    world: Arc<OnceCell<Arc<DetailWorld>>>,
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
}

/// Why a world could not be served.
#[derive(Debug)]
pub enum TileError {
    /// A generation is running: retry later.
    Busy,
    Failed(String),
}

/// Keeps the cache suspended while it lives (see [`TileCache::suspend`]).
pub struct Suspended<'a>(&'a TileCache);

impl Drop for Suspended<'_> {
    fn drop(&mut self) {
        self.0.lock().suspended -= 1;
    }
}

impl TileCache {
    /// A cache keeping loaded worlds within `budget` bytes.
    pub fn new(budget: usize) -> Self {
        Self {
            state: Mutex::default(),
            budget,
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
        let dir = dir.to_path_buf();
        let world = cell
            .get_or_try_init(|| async move {
                tokio::task::spawn_blocking(move || load(&dir))
                    .await
                    .map_err(|e| e.to_string())?
            })
            .await
            .cloned()
            .map_err(TileError::Failed)?;
        self.trim();
        Ok(world)
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
            let bytes = entry.world.get().map_or(0, |w| w.bytes());
            if total + bytes > self.budget && i + 1 < state.entries.len() {
                break;
            }
            total += bytes;
            keep = i;
        }
        state.entries.drain(..keep);
    }

    /// Empties the cache and stops loading worlds until the guard drops:
    /// a generation is about to start.
    pub fn suspend(&self) -> Suspended<'_> {
        let mut state = self.lock();
        state.suspended += 1;
        state.entries.clear();
        Suspended(self)
    }

    /// Forgets a world (deleted or regenerated).
    pub fn forget(&self, id: &str) {
        self.lock().entries.retain(|e| e.id != id);
    }

    /// Ids of the worlds held, least recently used first.
    #[cfg(test)]
    fn ids(&self) -> Vec<String> {
        self.lock().entries.iter().map(|e| e.id.clone()).collect()
    }
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
            let options = ExportOptions {
                raw_data: true,
                noise_maps: false,
            };
            export_world(
                &generate(&params),
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
            let _guard = cache.suspend();
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
}
