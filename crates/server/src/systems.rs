//! The system library: drawn systems on disk, one directory each (see
//! `docs/system-spec.md`, *Storage*):
//!
//! ```text
//! systems/<system-id>/
//! ├── system.json      the system as drawn
//! ├── names.json       names given by the user, by body id (optional)
//! └── bodies/<body-id>/   a generated body: the files a world has
//! ```
//!
//! The library reads only what its summaries need from `system.json`; the
//! system itself is `mapgen::system`'s.
use crate::api::SystemSummary;
use crate::staging;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The file of a system, in its directory.
pub const SYSTEM_FILE: &str = "system.json";
/// The names given to a system's bodies, in its directory.
pub const NAMES_FILE: &str = "names.json";
/// The directory of a system's generated bodies.
pub const BODIES_DIR: &str = "bodies";
/// Longest name a body can be given, in characters.
pub const MAX_NAME_CHARS: usize = 60;
/// The body id of the star.
pub const STAR: &str = "star";

/// Whether `id` is a system directory name: lowercase letters, digits and
/// dashes ending in `-<seed>` (`mgs-48213-42`). Anything else is refused
/// before touching the file system, which rules out path traversal.
pub fn is_valid_system_id(id: &str) -> bool {
    let Some((name, seed)) = id.rsplit_once('-') else {
        return false;
    };
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !seed.is_empty()
        && seed.chars().all(|c| c.is_ascii_digit())
}

/// Whether `id` is a body id: `star`, a planet's letters (`b` … `z`, then
/// `ba`…), or a moon's `<planet>-<n>` with `n` from 1.
pub fn is_valid_body_id(id: &str) -> bool {
    let planet =
        |p: &str| (1..=2).contains(&p.len()) && p.bytes().all(|b| (b'b'..=b'z').contains(&b));
    if id == STAR {
        return true;
    }
    match id.split_once('-') {
        None => planet(id),
        Some((p, n)) => {
            planet(p)
                && !n.is_empty()
                && !n.starts_with('0')
                && n.chars().all(|c| c.is_ascii_digit())
        }
    }
}

/// The directory of system `id` under `root`.
pub fn system_dir(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

/// The directory of body `body` of system `id` under `root`.
pub fn body_dir(root: &Path, id: &str, body: &str) -> PathBuf {
    root.join(id).join(BODIES_DIR).join(body)
}

/// The fields of `system.json` a summary needs.
#[derive(Deserialize)]
struct Meta {
    seed: u32,
    width: u32,
    height: u32,
    star: MetaStar,
    bodies: Vec<MetaBody>,
}

#[derive(Deserialize)]
struct MetaStar {
    #[serde(rename = "type")]
    kind: String,
    designation: String,
}

#[derive(Deserialize)]
struct MetaBody {
    id: String,
    kind: String,
    #[serde(rename = "type")]
    preset: String,
    designation: String,
    #[serde(default)]
    generated: bool,
}

/// The names given to a system's bodies, by body id (`names.json`).
pub type GivenNames = BTreeMap<String, String>;

/// The names given to system `id`: none when it has no `names.json`, or
/// one that cannot be read.
pub fn given_names(root: &Path, id: &str) -> GivenNames {
    std::fs::read(system_dir(root, id).join(NAMES_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Modification time of a file in milliseconds since the Unix epoch.
fn modified_ms(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(
        modified
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64),
    )
}

/// A body's revision: the modification time of its `raw_data/world.json`,
/// which keys its cached tiles. `None` while it is not generated.
pub fn body_revision(root: &Path, id: &str, body: &str) -> Option<u64> {
    modified_ms(&body_dir(root, id, body).join("raw_data").join("world.json"))
}

/// Reads one system's summary from its `system.json` and `names.json`.
pub fn summary(root: &Path, id: &str) -> Option<SystemSummary> {
    let file = system_dir(root, id).join(SYSTEM_FILE);
    let meta: Meta = serde_json::from_slice(&std::fs::read(&file).ok()?).ok()?;
    let mut planets: BTreeMap<String, u32> = BTreeMap::new();
    for body in meta.bodies.iter().filter(|b| b.kind == "planet") {
        *planets.entry(body.preset.clone()).or_default() += 1;
    }
    Some(SystemSummary {
        id: id.to_string(),
        seed: meta.seed,
        width: meta.width,
        height: meta.height,
        star: meta.star.kind,
        designation: meta.star.designation,
        planets,
        moons: meta.bodies.iter().filter(|b| b.kind == "moon").count() as u32,
        bodies: meta
            .bodies
            .iter()
            .map(|b| crate::api::BodySummary {
                id: b.id.clone(),
                designation: b.designation.clone(),
                generated: b.generated,
            })
            .collect(),
        names: given_names(root, id),
        modified_ms: modified_ms(&file).unwrap_or(0),
    })
}

/// Every system under `root`, most recently drawn first.
pub fn list(root: &Path) -> Vec<SystemSummary> {
    let Ok(read) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut systems: Vec<SystemSummary> = read
        .flatten()
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();
            is_valid_system_id(&id)
                .then(|| summary(root, &id))
                .flatten()
        })
        .collect();
    systems.sort_by_key(|s| std::cmp::Reverse(s.modified_ms));
    systems
}

/// Removes what a killed server left: unfinished systems under `root`,
/// and unfinished bodies in each system's `bodies/`.
pub fn sweep(root: &Path) {
    staging::sweep(root, is_valid_system_id);
    let Ok(read) = std::fs::read_dir(root) else {
        return;
    };
    for entry in read.flatten() {
        let id = entry.file_name().to_string_lossy().into_owned();
        if is_valid_system_id(&id) {
            staging::sweep(&entry.path().join(BODIES_DIR), is_valid_body_id);
        }
    }
}

/// Why names could not be given.
#[derive(Debug, PartialEq)]
pub enum NamingError {
    /// No such system.
    NotFound,
    /// A name is empty once trimmed, too long, or holds control characters.
    InvalidName(String),
    /// A body id the system does not have.
    NoSuchBody(String),
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

/// Gives the bodies of system `id` the names `given`, replacing those
/// they had, and returns its summary. No names at all removes the file.
pub fn give_names(root: &Path, id: &str, given: &GivenNames) -> Result<SystemSummary, NamingError> {
    let current = summary(root, id).ok_or(NamingError::NotFound)?;
    let mut names = GivenNames::new();
    for (body, name) in given {
        if body != STAR && !current.bodies.iter().any(|b| &b.id == body) {
            return Err(NamingError::NoSuchBody(body.clone()));
        }
        names.insert(body.clone(), checked_name(name)?);
    }
    let path = system_dir(root, id).join(NAMES_FILE);
    let io = |e: std::io::Error| NamingError::Io(e.to_string());
    if names.is_empty() {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(io(e)),
            _ => {}
        }
    } else {
        // Written aside, then renamed: a reader never sees half a file.
        let temp = system_dir(root, id).join(format!(".{NAMES_FILE}.tmp"));
        let json = serde_json::to_vec_pretty(&names).map_err(|e| NamingError::Io(e.to_string()))?;
        std::fs::write(&temp, json).map_err(io)?;
        std::fs::rename(&temp, &path).map_err(io)?;
    }
    summary(root, id).ok_or(NamingError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("mapgen-systems-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    const SYSTEM: &str = r#"{
        "format_version": 1, "seed": 42, "width": 1024, "height": 512,
        "star": {"type": "K1V", "designation": "MGS 48213"},
        "bodies": [
            {"id": "b", "kind": "planet", "type": "barren", "parent": null, "designation": "MGS 48213 b", "generated": false},
            {"id": "c", "kind": "planet", "type": "terran", "parent": null, "designation": "MGS 48213 c", "generated": true},
            {"id": "c-1", "kind": "moon", "type": "rocky-moon", "parent": "c", "designation": "MGS 48213 c I", "generated": true},
            {"id": "d", "kind": "planet", "type": "gas-giant", "parent": null, "designation": "MGS 48213 d"}
        ]
    }"#;

    #[test]
    fn ids_follow_the_storage_rules() {
        for good in ["mgs-48213-42", "a-1", "mgs-1-2-3"] {
            assert!(is_valid_system_id(good), "{good}");
        }
        for bad in [
            "mgs", "-42", "mgs-", "../etc-1", "MGS-1", "a/b-1", "x-1a", "--1",
        ] {
            assert!(!is_valid_system_id(bad), "{bad}");
        }
        for good in ["star", "b", "p", "bb", "c-1", "c-12"] {
            assert!(is_valid_body_id(good), "{good}");
        }
        for bad in [
            "", "a", "B", "c-0", "c-01", "c-", "-1", "c-1-1", "abc", "..", "c/1",
        ] {
            assert!(!is_valid_body_id(bad), "{bad}");
        }
    }

    #[test]
    fn summaries_count_planets_by_type_and_list_bodies() {
        let root = temp_root("summary");
        std::fs::create_dir_all(root.join("mgs-48213-42")).unwrap();
        std::fs::write(root.join("mgs-48213-42").join(SYSTEM_FILE), SYSTEM).unwrap();
        // Not a system: no system.json, or an invalid id.
        std::fs::create_dir_all(root.join("empty-1")).unwrap();
        std::fs::create_dir_all(root.join(".tmp-mgs-1-1")).unwrap();
        let listed = list(&root);
        assert_eq!(listed.len(), 1);
        let s = &listed[0];
        assert_eq!((s.seed, s.width, s.star.as_str()), (42, 1024, "K1V"));
        assert_eq!(s.designation, "MGS 48213");
        assert_eq!(s.planets.get("terran"), Some(&1));
        assert_eq!(s.planets.len(), 3);
        assert_eq!(s.moons, 1);
        let generated: Vec<&str> = s
            .bodies
            .iter()
            .filter(|b| b.generated)
            .map(|b| b.id.as_str())
            .collect();
        assert_eq!(generated, ["c", "c-1"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn names_are_given_by_body_id() {
        let root = temp_root("names");
        std::fs::create_dir_all(root.join("mgs-48213-42")).unwrap();
        std::fs::write(root.join("mgs-48213-42").join(SYSTEM_FILE), SYSTEM).unwrap();
        let given: GivenNames = [
            ("c".into(), " Gaia ".into()),
            ("star".into(), "Helia".into()),
        ]
        .into();
        let s = give_names(&root, "mgs-48213-42", &given).unwrap();
        assert_eq!(s.names.get("c").map(String::as_str), Some("Gaia"));
        assert_eq!(s.names.get("star").map(String::as_str), Some("Helia"));
        let unknown: GivenNames = [("z".into(), "Nyx".into())].into();
        assert_eq!(
            give_names(&root, "mgs-48213-42", &unknown),
            Err(NamingError::NoSuchBody("z".into()))
        );
        let blank: GivenNames = [("c".into(), "  ".into())].into();
        assert!(matches!(
            give_names(&root, "mgs-48213-42", &blank),
            Err(NamingError::InvalidName(_))
        ));
        assert_eq!(
            give_names(&root, "mgs-1-1", &GivenNames::new()),
            Err(NamingError::NotFound)
        );
        // No names: the file goes.
        give_names(&root, "mgs-48213-42", &GivenNames::new()).unwrap();
        assert!(!root.join("mgs-48213-42").join(NAMES_FILE).exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_sweep_clears_unfinished_systems_and_bodies() {
        let root = temp_root("sweep");
        std::fs::create_dir_all(root.join(".tmp-mgs-1-1")).unwrap();
        std::fs::create_dir_all(root.join("mgs-2-2").join(BODIES_DIR).join(".tmp-c")).unwrap();
        std::fs::create_dir_all(root.join("mgs-2-2").join(BODIES_DIR).join("b")).unwrap();
        sweep(&root);
        assert!(!root.join(".tmp-mgs-1-1").exists());
        assert!(
            !root
                .join("mgs-2-2")
                .join(BODIES_DIR)
                .join(".tmp-c")
                .exists()
        );
        assert!(root.join("mgs-2-2").join(BODIES_DIR).join("b").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
