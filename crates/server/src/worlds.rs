//! The world library: generated worlds on disk, one directory each.
use crate::api::{GivenNames, MoonNames, WorldNames, WorldSummary};
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
/// Prefix a replaced world is renamed to while its replacement moves in
/// (see [`TempDir::commit_replacing`]). Never the prefix of a temporary
/// directory, whatever the world id.
const OLD_PREFIX: &str = ".old-";

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

    /// Renames the directory to `target`, replacing the world there if
    /// any: the old world is moved aside to `.old-<id>`, the new one
    /// renamed into place, then the old one removed. If the new one cannot
    /// be moved in, the old one is moved back, so a failed commit keeps the
    /// old world. A server killed between the two renames finds the old
    /// world aside at startup and restores it ([`sweep_temp_dirs`]).
    pub fn commit_replacing(self, target: &Path) -> Result<(), String> {
        let name = target
            .file_name()
            .ok_or_else(|| format!("{}: no directory name", target.display()))?
            .to_string_lossy();
        let aside = target.with_file_name(format!("{OLD_PREFIX}{name}"));
        let _ = std::fs::remove_dir_all(&aside);
        let replaced = match std::fs::rename(target, &aside) {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(format!("{}: {e}", target.display())),
        };
        if let Err(e) = std::fs::rename(&self.path, target) {
            if replaced && let Err(back) = std::fs::rename(&aside, target) {
                tracing::error!("could not restore {}: {back}", target.display());
            }
            return Err(format!("{}: {e}", target.display()));
        }
        if replaced && let Err(e) = std::fs::remove_dir_all(&aside) {
            tracing::warn!("could not remove {}: {e}", aside.display());
        }
        Ok(())
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

/// Removes the temporary directories a killed server left under `dir`, and
/// puts back a replaced world it had moved aside but not replaced yet.
pub fn sweep_temp_dirs(dir: &Path) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(id) = name.strip_prefix(OLD_PREFIX) {
            let path = entry.path();
            let target = dir.join(id);
            if is_valid_id(id) && !target.exists() {
                match std::fs::rename(&path, &target) {
                    Ok(()) => tracing::info!(
                        "restored {} after an unfinished replacement",
                        target.display()
                    ),
                    Err(e) => tracing::warn!("could not restore {}: {e}", target.display()),
                }
            } else if let Err(e) = std::fs::remove_dir_all(&path) {
                tracing::warn!("could not remove {}: {e}", path.display());
            }
            continue;
        }
        if name.starts_with(TEMP_PREFIX) {
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
    surface_age_ga: Option<f32>,
    format_version: u32,
    preset: MetaPreset,
    star: Option<MetaStar>,
    giants: Option<MetaGiants>,
    moons: Option<MetaMoons>,
    host: Option<MetaHost>,
}

/// The giants of `world.json`, as far as names need them.
#[derive(Deserialize)]
struct MetaGiants {
    giants: Vec<MetaOrbit>,
}

#[derive(Deserialize)]
struct MetaOrbit {
    orbit_au: f32,
}

/// The moons of `world.json`: names need their number only.
#[derive(Deserialize)]
struct MetaMoons {
    moons: Vec<serde::de::IgnoredAny>,
}

/// The giant whose moon the world is.
#[derive(Deserialize)]
struct MetaHost {
    distance_km: f32,
    siblings: Vec<MetaDistance>,
}

#[derive(Deserialize)]
struct MetaDistance {
    distance_km: f32,
}

/// The part of the stored preset a summary needs.
#[derive(Deserialize, Default)]
#[serde(default)]
struct MetaPreset {
    geology: MetaGeology,
}

/// Earth's when absent: still wet.
#[derive(Deserialize, Default)]
#[serde(default)]
struct MetaGeology {
    wet_until: f32,
}

/// First format version with `elevation_base`, which epochs are drawn on.
const EPOCHS_VERSION: u32 = 7;

impl Meta {
    /// The epochs the world offers, in Ma (see [`WorldSummary::epochs_ma`]).
    fn epochs_ma(&self) -> Option<[i32; 2]> {
        if self.format_version < EPOCHS_VERSION {
            return None;
        }
        let (future, past) = mapgen::epoch::range_ga(
            self.pressure_bar,
            self.surface_age_ga?,
            self.preset.geology.wet_until,
        )?;
        // Whole Ma, within the range.
        Some([
            (future * 1000.0).ceil() as i32,
            (past * 1000.0).floor() as i32,
        ])
    }
}

/// The star of `world.json`, as far as the summary needs it.
#[derive(Deserialize)]
struct MetaStar {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default = "one_au")]
    orbit_au: f32,
}

fn one_au() -> f32 {
    1.0
}

impl Meta {
    /// The names of the world's system, with those given in `given`.
    fn names(&self, given: GivenNames) -> WorldNames {
        let star = self.star.as_ref();
        let names = mapgen::naming::names(&mapgen::naming::Layout {
            seed: self.seed as u32,
            // Worlds made before stars orbit the Sun at 1 AU.
            star: star.map_or("G2V", |s| s.kind.as_str()),
            orbit_au: star.map_or(1.0, |s| s.orbit_au),
            giant_orbits_au: self
                .giants
                .iter()
                .flat_map(|g| g.giants.iter().map(|o| o.orbit_au))
                .collect(),
            moons: self.moons.as_ref().map_or(0, |m| m.moons.len()),
            host: self.host.as_ref().map(|h| mapgen::naming::HostLayout {
                distance_km: h.distance_km,
                sibling_distances_km: h.siblings.iter().map(|s| s.distance_km).collect(),
            }),
        });
        let GivenNames { name, mut moons } = given;
        WorldNames {
            star: names.star,
            designation: names.world,
            name,
            giants: names.giants,
            moons: names
                .moons
                .into_iter()
                .enumerate()
                .map(|(k, designation)| MoonNames {
                    designation,
                    name: moons.remove(&(k + 1).to_string()),
                })
                .collect(),
            host: names.host,
            siblings: names.siblings,
        }
    }

    /// The number of the world's moons.
    fn moon_count(&self) -> usize {
        self.moons.as_ref().map_or(0, |m| m.moons.len())
    }
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
            // Worlds made before the surface age have none.
            surface_age_ga: None,
            format_version: 1,
            preset: MetaPreset::default(),
            // Nor a star: they orbit the Sun.
            star: None,
            giants: None,
            moons: None,
            host: None,
        }
    }
}

/// File of the names given to a world, next to `raw_data/`.
pub const NAMES_FILE: &str = "names.json";

/// Longest name a world or a moon can be given, in characters.
pub const MAX_NAME_CHARS: usize = 60;

/// The names given to world `id`: none when it has no `names.json`, or
/// one that cannot be read.
fn given_names(dir: &Path, id: &str) -> GivenNames {
    std::fs::read(dir.join(id).join(NAMES_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Why names could not be given.
#[derive(Debug, PartialEq)]
pub enum NamingError {
    /// No such world.
    NotFound,
    /// A name is empty once trimmed, too long, or holds control characters.
    InvalidName(String),
    /// A moon number the world does not have.
    NoSuchMoon(String),
    Io(String),
}

/// A name trimmed of its surrounding spaces, if it can be given.
fn checked_name(name: &str) -> Result<String, NamingError> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > MAX_NAME_CHARS
        || trimmed.chars().any(char::is_control)
    {
        return Err(NamingError::InvalidName(name.to_string()));
    }
    Ok(trimmed.to_string())
}

/// Gives world `id` and its moons the names `given`, replacing those it
/// had, and returns its summary. No names at all removes `names.json`.
pub fn give_names(dir: &Path, id: &str, given: GivenNames) -> Result<WorldSummary, NamingError> {
    let meta = read_meta(dir, id).ok_or(NamingError::NotFound)?;
    let name = given.name.as_deref().map(checked_name).transpose()?;
    let mut moons = std::collections::BTreeMap::new();
    for (number, moon_name) in &given.moons {
        let n: usize = number
            .parse()
            .map_err(|_| NamingError::NoSuchMoon(number.clone()))?;
        if n == 0 || n > meta.moon_count() || number != &n.to_string() {
            return Err(NamingError::NoSuchMoon(number.clone()));
        }
        moons.insert(number.clone(), checked_name(moon_name)?);
    }
    let names = GivenNames { name, moons };
    let path = dir.join(id).join(NAMES_FILE);
    if names == GivenNames::default() {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(NamingError::Io(e.to_string())),
        }
    } else {
        // Written aside, then renamed: a reader never sees half a file.
        let temp = dir.join(id).join(format!(".{NAMES_FILE}.tmp"));
        let json = serde_json::to_vec_pretty(&names).map_err(|e| NamingError::Io(e.to_string()))?;
        std::fs::write(&temp, json).map_err(|e| NamingError::Io(e.to_string()))?;
        std::fs::rename(&temp, &path).map_err(|e| NamingError::Io(e.to_string()))?;
    }
    summary(dir, id).ok_or(NamingError::NotFound)
}

/// The `raw_data/world.json` of world `id`, as far as summaries need it.
fn read_meta(dir: &Path, id: &str) -> Option<Meta> {
    let path = dir.join(id).join("raw_data").join("world.json");
    serde_json::from_slice(&std::fs::read(&path).ok()?).ok()
}

/// Reads one world's summary from its `raw_data/world.json`, and the
/// names given to it from its `names.json`.
pub fn summary(dir: &Path, id: &str) -> Option<WorldSummary> {
    let meta = read_meta(dir, id)?;
    let modified_ms = revision(dir, id).unwrap_or(0);
    let epochs_ma = meta.epochs_ma();
    let names = meta.names(given_names(dir, id));
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
        surface_age_ga: meta.surface_age_ga,
        epochs_ma,
        names,
        star: meta.star.map(|s| s.kind),
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
    fn replacing_commits_swap_the_world_or_keep_the_old_one() {
        let root = std::env::temp_dir().join(format!("mapgen-replace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("terran-1")).unwrap();
        std::fs::write(root.join("terran-1/world.png"), b"old").unwrap();
        let target = root.join("terran-1");

        // A replacement swaps the directories and removes the old one.
        let temp = TempDir::create(&root, "terran-1").unwrap();
        std::fs::write(temp.path().join("world.png"), b"new").unwrap();
        temp.commit_replacing(&target).unwrap();
        assert_eq!(std::fs::read(target.join("world.png")).unwrap(), b"new");
        assert!(!root.join(".old-terran-1").exists());
        assert!(!root.join(".tmp-terran-1").exists());

        // Nothing to replace: a plain commit.
        let temp = TempDir::create(&root, "terran-2").unwrap();
        temp.commit_replacing(&root.join("terran-2")).unwrap();
        assert!(root.join("terran-2").is_dir());

        // The new directory cannot move in (gone here): the old world stays.
        let temp = TempDir::create(&root, "terran-1").unwrap();
        std::fs::remove_dir_all(temp.path()).unwrap();
        assert!(temp.commit_replacing(&target).is_err());
        assert_eq!(std::fs::read(target.join("world.png")).unwrap(), b"new");
        assert!(!root.join(".old-terran-1").exists());

        // Killed between the renames: the sweep restores the old world, or
        // removes it when its replacement made it into place.
        std::fs::rename(&target, root.join(".old-terran-1")).unwrap();
        std::fs::create_dir_all(root.join(".old-terran-2")).unwrap();
        sweep_temp_dirs(&root);
        assert_eq!(std::fs::read(target.join("world.png")).unwrap(), b"new");
        assert!(!root.join(".old-terran-1").exists());
        assert!(!root.join(".old-terran-2").exists());
        assert!(root.join("terran-2").is_dir());
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
                    "pressure_bar": 2.5, "surface_age_ga": 3.8, "layers": {"elevation": {}}, "biomes": [1, 2]}"#,
            ),
            // Before the atmosphere: Earth's pressure.
            ("frozen-2", r#"{"planet_type": "frozen", "seed": 2}"#),
            // Airless, of format version 7: epochs back to its surface age.
            (
                "mercury-like-3",
                r#"{"planet_type": "mercury-like", "seed": 3, "format_version": 7,
                    "pressure_bar": 0.0, "surface_age_ga": 4.0,
                    "preset": {"geology": {"regime": "stagnant", "wet_until": 4.6}}}"#,
            ),
        ] {
            std::fs::create_dir_all(root.join(id).join("raw_data")).unwrap();
            std::fs::write(root.join(id).join("raw_data/world.json"), json).unwrap();
        }
        let terran = summary(&root, "terran-1").unwrap();
        assert_eq!((terran.planet.as_str(), terran.width), ("terran", 64));
        assert_eq!(terran.pressure_bar, 2.5);
        assert_eq!(terran.surface_age_ga, Some(3.8));
        let frozen = summary(&root, "frozen-2").unwrap();
        assert_eq!((frozen.seed, frozen.pressure_bar), (2, 1.0));
        assert_eq!(frozen.surface_age_ga, None);
        assert_eq!((terran.epochs_ma, frozen.epochs_ma), (None, None));
        let mercury = summary(&root, "mercury-like-3").unwrap();
        assert_eq!(mercury.epochs_ma, Some([-1000, 4000]));
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
