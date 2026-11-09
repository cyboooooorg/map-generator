//! Drift previews kept for the slider (see [`mapgen::drift`]).
//!
//! A preview is a PNG of a few tens of KB, keyed by the body, its revision,
//! the epoch and the width. The most recently used stay within a byte
//! budget, so that a slider moving back and forth over the same stops is
//! served from memory.
use std::sync::{Arc, Mutex};

/// Memory kept for previews, bytes.
const BUDGET_BYTES: usize = 32 << 20;

/// What identifies a preview.
#[derive(Clone, PartialEq, Eq)]
pub struct PreviewKey {
    /// The body's resolution (`systems/<id>/<body>/<W>x<H>`) and its
    /// revision: regenerating it changes its preview.
    pub body: String,
    pub revision: u64,
    pub drift_ma: i32,
    pub width: u32,
}

/// Least recently used previews within [`BUDGET_BYTES`].
#[derive(Default)]
pub struct PreviewCache {
    /// Most recently used last.
    entries: Mutex<Vec<(PreviewKey, Arc<Vec<u8>>)>>,
}

impl PreviewCache {
    pub fn get(&self, key: &PreviewKey) -> Option<Arc<Vec<u8>>> {
        let mut entries = self.entries.lock().expect("preview cache lock");
        let at = entries.iter().position(|(k, _)| k == key)?;
        let entry = entries.remove(at);
        let png = entry.1.clone();
        entries.push(entry);
        Some(png)
    }

    pub fn insert(&self, key: PreviewKey, png: Arc<Vec<u8>>) {
        let mut entries = self.entries.lock().expect("preview cache lock");
        // Other revisions of the body are stale.
        entries.retain(|(k, _)| k != &key && (k.body != key.body || k.revision == key.revision));
        entries.push((key, png));
        let mut total: usize = entries.iter().map(|(_, p)| p.len()).sum();
        while total > BUDGET_BYTES && entries.len() > 1 {
            total -= entries.remove(0).1.len();
        }
    }

    /// Forgets the previews of `body` (a resolution deleted).
    pub fn forget(&self, body: &str) {
        self.entries
            .lock()
            .expect("preview cache lock")
            .retain(|(k, _)| k.body != body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(body: &str, revision: u64, drift_ma: i32) -> PreviewKey {
        PreviewKey {
            body: body.into(),
            revision,
            drift_ma,
            width: 256,
        }
    }

    #[test]
    fn keeps_recent_previews_within_the_budget() {
        let cache = PreviewCache::default();
        let big = || Arc::new(vec![0u8; BUDGET_BYTES / 2 - 1]);
        cache.insert(key("a", 1, 10), big());
        cache.insert(key("a", 1, 20), big());
        // Using the first makes the second the oldest.
        assert!(cache.get(&key("a", 1, 10)).is_some());
        cache.insert(key("a", 1, 30), big());
        assert!(cache.get(&key("a", 1, 20)).is_none());
        assert!(cache.get(&key("a", 1, 10)).is_some());
        assert!(cache.get(&key("a", 1, 30)).is_some());
    }

    #[test]
    fn a_new_revision_drops_the_old_one() {
        let cache = PreviewCache::default();
        cache.insert(key("a", 1, 10), Arc::new(vec![1]));
        cache.insert(key("b", 1, 10), Arc::new(vec![2]));
        cache.insert(key("a", 2, 10), Arc::new(vec![3]));
        assert!(cache.get(&key("a", 1, 10)).is_none());
        assert!(cache.get(&key("a", 2, 10)).is_some());
        assert!(cache.get(&key("b", 1, 10)).is_some());
    }

    #[test]
    fn a_body_forgotten_drops_its_previews_only() {
        let cache = PreviewCache::default();
        cache.insert(key("a/1x1", 1, 10), Arc::new(vec![1]));
        cache.insert(key("a/2x1", 1, 10), Arc::new(vec![2]));
        cache.forget("a/1x1");
        assert!(cache.get(&key("a/1x1", 1, 10)).is_none());
        assert!(cache.get(&key("a/2x1", 1, 10)).is_some());
    }
}
