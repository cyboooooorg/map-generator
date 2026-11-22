//! Where each file lives under the store root.

use std::path::{Path, PathBuf};

use catalogue::{SystemId, WorldKey};
use timeline::{Designation, EngineVersion};

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

/// The directory of the cached worlds of one system and engine version.
pub(super) fn worlds_dir(root: &Path, engine: EngineVersion, id: SystemId) -> PathBuf {
    entry_dir(root, engine, id).join("worlds")
}

/// The directory name of a body: its designation with `-` for spaces.
pub(super) fn body_slug(body: &Designation) -> String {
    body.to_string().replace(' ', "-")
}

/// The body of a directory name made by [`body_slug`].
pub(super) fn parse_body_slug(slug: &str) -> Option<Designation> {
    slug.replace('-', " ").parse().ok()
}

/// The cached world of a key.
pub(super) fn world_file(root: &Path, key: &WorldKey) -> PathBuf {
    worlds_dir(root, key.engine, key.system)
        .join(body_slug(&key.body))
        .join(format!(
            "{}-{}-w{}.world",
            key.keyframe, key.grid, key.world.0
        ))
}

/// The keyframe, grid and world engine version of a world file name
/// (`<keyframe>-<grid>-w<world>.world`), without its directory. Any
/// other form, such as the older `<keyframe>-<grid>.world`, is `None`.
pub(super) fn parse_world_name(name: &str) -> Option<(u32, u32, u32)> {
    let (keyframe, rest) = name.strip_suffix(".world")?.split_once('-')?;
    let (grid, world) = rest.split_once("-w")?;
    Some((
        keyframe.parse().ok()?,
        grid.parse().ok()?,
        world.parse().ok()?,
    ))
}

/// The key of a world file in `uses.json`: its path relative to `cache/`.
pub(super) fn world_use_key(key: &WorldKey) -> String {
    format!(
        "{}/{}/worlds/{}/{}-{}-w{}.world",
        key.engine.0,
        key.system,
        body_slug(&key.body),
        key.keyframe,
        key.grid,
        key.world.0
    )
}
