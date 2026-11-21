//! Where each file lives under the store root.

use std::path::{Path, PathBuf};

use catalogue::SystemId;
use timeline::EngineVersion;

/// The directory of the stored systems.
pub(super) fn systems_dir(root: &Path) -> PathBuf {
    root.join("systems")
}

/// The file holding the inputs of a system.
pub(super) fn inputs_file(root: &Path, id: SystemId) -> PathBuf {
    systems_dir(root).join(id.to_string()).join("inputs.json")
}

/// The root of every cache.
pub(super) fn cache_dir(root: &Path) -> PathBuf {
    root.join("cache")
}

/// The directory of the cache entries of one system and engine version.
pub(super) fn entry_dir(root: &Path, engine: EngineVersion, id: SystemId) -> PathBuf {
    cache_dir(root)
        .join(engine.0.to_string())
        .join(id.to_string())
}

/// The cached timelines of a system for an engine version.
pub(super) fn timelines_file(root: &Path, engine: EngineVersion, id: SystemId) -> PathBuf {
    entry_dir(root, engine, id).join("timelines.json")
}

/// The key of the timelines file in `uses.json`: its path relative to
/// `cache/`, with `/` separators.
pub(super) fn timelines_use_key(engine: EngineVersion, id: SystemId) -> String {
    format!("{}/{id}/timelines.json", engine.0)
}

/// The usage ledger.
pub(super) fn uses_file(root: &Path) -> PathBuf {
    cache_dir(root).join("uses.json")
}
