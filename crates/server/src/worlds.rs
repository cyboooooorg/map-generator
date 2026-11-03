//! The world library: generated worlds on disk, one directory each.
use crate::api::WorldSummary;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Whether `id` is a world directory name (`<planet>-<seed>`). Anything else
/// is refused before touching the file system, which rules out path traversal.
pub fn is_valid_id(id: &str) -> bool {
    let Some((planet, seed)) = id.rsplit_once('-') else {
        return false;
    };
    !planet.is_empty()
        && planet
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !seed.is_empty()
        && seed.chars().all(|c| c.is_ascii_digit())
}

/// Prefix of the temporary directories generations export into. A dot is
/// never part of a world id, so the library never lists them.
const TEMP_PREFIX: &str = ".tmp-";

/// The directory a world is exported into before it is complete, deleted on
/// drop unless [`commit`](Self::commit) renamed it into place: an export
/// that fails, panics or is cancelled leaves nothing behind.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates `<dir>/.tmp-<id>`, emptied first if a previous attempt left it.
    pub fn create(dir: &Path, id: &str) -> Result<Self, String> {
        let path = dir.join(format!("{TEMP_PREFIX}{id}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Renames the directory to `target`, which must not exist.
    pub fn commit(self, target: &Path) -> Result<(), String> {
        std::fs::rename(&self.path, target).map_err(|e| format!("{}: {e}", target.display()))
        // On failure, drop removes the temporary directory.
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Already renamed after a successful commit: nothing to remove.
        if self.path.exists()
            && let Err(e) = std::fs::remove_dir_all(&self.path)
        {
            tracing::warn!("could not remove {}: {e}", self.path.display());
        }
    }
}

/// Removes the temporary directories a killed server left under `dir`.
pub fn sweep_temp_dirs(dir: &Path) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        if entry.file_name().to_string_lossy().starts_with(TEMP_PREFIX) {
            let path = entry.path();
            match std::fs::remove_dir_all(&path) {
                Ok(()) => tracing::info!("removed the unfinished export {}", path.display()),
                Err(e) => tracing::warn!("could not remove {}: {e}", path.display()),
            }
        }
    }
}

/// Every world under `dir` that has raw data, most recent first.
pub fn list(dir: &Path) -> Vec<WorldSummary> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut worlds: Vec<WorldSummary> = read
        .flatten()
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();
            is_valid_id(&id).then(|| summary(dir, &id)).flatten()
        })
        .collect();
    worlds.sort_by_key(|w| std::cmp::Reverse(w.modified_ms));
    worlds
}

/// A world's revision: the modification time of its `raw_data/world.json`
/// in milliseconds (the summary's `modified_ms`), from the file's metadata
/// only. `None` when the world has no raw data.
pub fn revision(dir: &Path, id: &str) -> Option<u64> {
    let path = dir.join(id).join("raw_data").join("world.json");
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(
        modified
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64),
    )
}

/// The fields of `raw_data/world.json` a summary needs: the layer and
/// biome tables are skipped without being built. Fields missing from older
/// worlds take their default.
#[derive(Deserialize)]
#[serde(default)]
struct Meta {
    planet_type: String,
    seed: u64,
    width: u32,
    height: u32,
    sea_level: f32,
    volcanic_intensity: f32,
    circumference_km: f32,
    axial_tilt_deg: f32,
    erosion: f32,
    gravity_modifier: f32,
    pressure_bar: f32,
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            planet_type: "unknown".into(),
            seed: 0,
            width: 0,
            height: 0,
            sea_level: 0.0,
            volcanic_intensity: 0.0,
            circumference_km: 0.0,
            axial_tilt_deg: 0.0,
            erosion: 0.0,
            gravity_modifier: 0.0,
            // Worlds made before the atmosphere had Earth's pressure.
            pressure_bar: 1.0,
        }
    }
}

/// Reads one world's summary from its `raw_data/world.json`.
pub fn summary(dir: &Path, id: &str) -> Option<WorldSummary> {
    let path = dir.join(id).join("raw_data").join("world.json");
    let meta: Meta = serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
    let modified_ms = revision(dir, id).unwrap_or(0);
    Some(WorldSummary {
        id: id.to_string(),
        planet: meta.planet_type,
        seed: meta.seed as u32,
        width: meta.width,
        height: meta.height,
        sea_level: meta.sea_level,
        volcanic_intensity: meta.volcanic_intensity,
        circumference_km: meta.circumference_km,
        axial_tilt_deg: meta.axial_tilt_deg,
        erosion: meta.erosion,
        gravity_modifier: meta.gravity_modifier,
        pressure_bar: meta.pressure_bar,
        modified_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_dirs_are_renamed_or_removed() {
        let root = std::env::temp_dir().join(format!("mapgen-tempdir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        // Dropped before its commit (error, cancel, panic): nothing is left.
        let temp = TempDir::create(&root, "terran-1").unwrap();
        std::fs::write(temp.path().join("world.png"), b"partial").unwrap();
        drop(temp);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);

        // Committed: the complete directory appears under its final name.
        let temp = TempDir::create(&root, "terran-2").unwrap();
        std::fs::write(temp.path().join("world.png"), b"done").unwrap();
        temp.commit(&root.join("terran-2")).unwrap();
        assert!(root.join("terran-2/world.png").exists());
        assert!(!root.join(".tmp-terran-2").exists());

        // Hidden from the library, removed by the sweep.
        std::fs::create_dir_all(root.join(".tmp-terran-3/raw_data")).unwrap();
        sweep_temp_dirs(&root);
        assert!(!root.join(".tmp-terran-3").exists());
        assert!(root.join("terran-2").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn summaries_read_old_and_new_metadata() {
        let root = std::env::temp_dir().join(format!("mapgen-summary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (id, json) in [
            (
                "terran-1",
                r#"{"planet_type": "terran", "seed": 1, "width": 64, "height": 32,
                    "pressure_bar": 2.5, "layers": {"elevation": {}}, "biomes": [1, 2]}"#,
            ),
            // Before the atmosphere: Earth's pressure.
            ("frozen-2", r#"{"planet_type": "frozen", "seed": 2}"#),
        ] {
            std::fs::create_dir_all(root.join(id).join("raw_data")).unwrap();
            std::fs::write(root.join(id).join("raw_data/world.json"), json).unwrap();
        }
        let terran = summary(&root, "terran-1").unwrap();
        assert_eq!((terran.planet.as_str(), terran.width), ("terran", 64));
        assert_eq!(terran.pressure_bar, 2.5);
        let frozen = summary(&root, "frozen-2").unwrap();
        assert_eq!((frozen.seed, frozen.pressure_bar), (2, 1.0));
        assert!(summary(&root, "barren-3").is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ids_follow_the_directory_pattern() {
        assert!(is_valid_id("terran-42"));
        assert!(is_valid_id("mars-like-4"));
        for bad in [
            "terran", "-42", "terran-", "../etc-1", "Terran-1", "a/b-1", "x-1a",
        ] {
            assert!(!is_valid_id(bad), "{bad}");
        }
    }
}
