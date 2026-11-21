//! The usage ledger: a logical tick instead of a clock, so that the
//! last use of a cache entry is deterministic.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `cache/uses.json`: the last tick handed out and the tick of the last
/// use of each cache file, by path relative to `cache/`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct Uses {
    pub tick: u64,
    pub uses: BTreeMap<String, u64>,
}

impl Uses {
    /// Records a use of `key` at the next tick.
    pub fn touch(&mut self, key: String) {
        self.tick += 1;
        self.uses.insert(key, self.tick);
    }

    /// The tick of the last use of `key`, 0 when never recorded.
    pub fn last(&self, key: &str) -> u64 {
        self.uses.get(key).copied().unwrap_or(0)
    }
}
