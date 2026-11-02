//! Detail tiles of the deep zoom, rendered on demand from the world's raw data.
//!
//! Loading a world (decoding its layers) takes a fraction of a second and a
//! few tens of MB, so the two most recently used worlds stay in memory. A
//! world is loaded once even when many tile requests arrive together.
use mapgen::detail::{DetailWorld, TileId};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

/// Worlds kept in memory.
const CAPACITY: usize = 2;

/// A loaded world, keyed by id and revision (the modification time of its
/// raw data): regenerating a world changes its revision.
struct Entry {
    id: String,
    revision: u64,
    world: Arc<OnceCell<Arc<DetailWorld>>>,
}

#[derive(Default)]
pub struct TileCache {
    /// Most recently used last.
    entries: Mutex<Vec<Entry>>,
}

impl TileCache {
    /// The loaded world for `id` at `revision`, loading it if needed.
    pub async fn world(
        &self,
        dir: &Path,
        id: &str,
        revision: u64,
    ) -> Result<Arc<DetailWorld>, String> {
        let cell = {
            let mut entries = self.entries.lock().expect("tile cache lock");
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
            if entries.len() > CAPACITY {
                entries.remove(0);
            }
            cell
        };
        let dir = dir.to_path_buf();
        cell.get_or_try_init(|| async move {
            tokio::task::spawn_blocking(move || {
                mapgen::export::raw_data::load_layers(&dir).map(|w| Arc::new(DetailWorld::new(w)))
            })
            .await
            .map_err(|e| e.to_string())?
        })
        .await
        .cloned()
    }

    /// Forgets every loaded world, to free memory.
    pub fn clear(&self) {
        self.entries.lock().expect("tile cache lock").clear();
    }

    /// Forgets a world (deleted or regenerated).
    pub fn forget(&self, id: &str) {
        self.entries
            .lock()
            .expect("tile cache lock")
            .retain(|e| e.id != id);
    }
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
