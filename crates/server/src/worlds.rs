//! The world library: generated worlds on disk, one directory each.
use crate::api::WorldSummary;
use serde_json::Value;
use std::path::Path;
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

/// Reads one world's summary from its `raw_data/world.json`.
pub fn summary(dir: &Path, id: &str) -> Option<WorldSummary> {
    let path = dir.join(id).join("raw_data").join("world.json");
    let meta: Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    let modified_ms = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64);
    let f = |key: &str| meta[key].as_f64().unwrap_or(0.0) as f32;
    let u = |key: &str| meta[key].as_u64().unwrap_or(0) as u32;
    Some(WorldSummary {
        id: id.to_string(),
        planet: meta["planet_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        seed: u("seed"),
        width: u("width"),
        height: u("height"),
        sea_level: f("sea_level"),
        volcanic_intensity: f("volcanic_intensity"),
        circumference_km: f("circumference_km"),
        axial_tilt_deg: f("axial_tilt_deg"),
        erosion: f("erosion"),
        gravity_modifier: f("gravity_modifier"),
        modified_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
