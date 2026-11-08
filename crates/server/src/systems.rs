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
//! system itself is `mapgen::system`'s. The server lists systems from its
//! index (see [`crate::index`]), which scans this tree.
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
    #[serde(default)]
    info: MetaStarInfo,
}

/// The star's light and habitable zone, as far as the library's orrery
/// needs them.
#[derive(Deserialize, Default)]
struct MetaStarInfo {
    #[serde(default)]
    light: Option<[f32; 3]>,
    #[serde(default)]
    hz_inner_au: f32,
    #[serde(default)]
    hz_outer_au: f32,
}

/// A body's orbit, as far as the library's orrery needs it.
#[derive(Deserialize, Default)]
struct MetaOrbit {
    #[serde(default)]
    semi_major_axis_au: f64,
    #[serde(default)]
    eccentricity: f64,
    #[serde(default)]
    node_deg: f64,
    #[serde(default)]
    periapsis_deg: f64,
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
    #[serde(default)]
    preview: MetaPreview,
    #[serde(default)]
    orbit: MetaOrbit,
}

/// The preview of a body, as far as the library's strip needs it.
#[derive(Deserialize, Default)]
struct MetaPreview {
    #[serde(default)]
    colours: Vec<[f32; 3]>,
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
pub fn modified_ms(path: &Path) -> Option<u64> {
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

/// The parameters a body of a system is generated with, from its
/// `system.json`: `None` when the system or the body is not there or the
/// file cannot be read.
pub fn body_params(root: &Path, id: &str, body: &str) -> Option<mapgen::world::WorldParams> {
    let system = mapgen::system::read_system(&system_dir(root, id)).ok()?;
    let body = system.bodies.iter().find(|b| b.id == body)?;
    Some(mapgen::system::world_params(body))
}

/// A system's summary from the bytes of its `system.json` (modified at
/// `modified_ms`); the names are the index's to fill in.
pub fn summarize(id: &str, json: &[u8], modified_ms: u64) -> Option<SystemSummary> {
    let meta: Meta = serde_json::from_slice(json).ok()?;
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
        star_light: meta.star.info.light.unwrap_or([1.0, 1.0, 1.0]),
        hz_inner_au: meta.star.info.hz_inner_au,
        hz_outer_au: meta.star.info.hz_outer_au,
        planets,
        moons: meta.bodies.iter().filter(|b| b.kind == "moon").count() as u32,
        bodies: meta
            .bodies
            .iter()
            .map(|b| crate::api::BodySummary {
                id: b.id.clone(),
                designation: b.designation.clone(),
                kind: b.kind.clone(),
                body_type: b.preset.clone(),
                colour: b
                    .preview
                    .colours
                    .first()
                    .copied()
                    .unwrap_or([0.5, 0.5, 0.5]),
                generated: b.generated,
                orbit_au: b.orbit.semi_major_axis_au,
                eccentricity: b.orbit.eccentricity,
                periapsis_deg: (b.orbit.node_deg + b.orbit.periapsis_deg).rem_euclid(360.0),
            })
            .collect(),
        names: GivenNames::new(),
        modified_ms,
    })
}

/// The ids of the systems under `root`: directories with a valid name.
pub fn system_ids(root: &Path) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    read.flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|id| is_valid_system_id(id))
        .collect()
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

/// Checks the names to give to the bodies of a system (`current`: its
/// bodies, the star is always there) and returns them trimmed.
pub fn checked_names(
    current: &[crate::api::BodySummary],
    given: &GivenNames,
) -> Result<GivenNames, NamingError> {
    let mut names = GivenNames::new();
    for (body, name) in given {
        if body != STAR && !current.iter().any(|b| &b.id == body) {
            return Err(NamingError::NoSuchBody(body.clone()));
        }
        names.insert(body.clone(), checked_name(name)?);
    }
    Ok(names)
}

/// Writes `names.json` of system `id` (the tree stays self-sufficient
/// beside the index); no names at all removes the file.
pub fn write_names_file(root: &Path, id: &str, names: &GivenNames) -> Result<(), NamingError> {
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
        let json = serde_json::to_vec_pretty(names).map_err(|e| NamingError::Io(e.to_string()))?;
        std::fs::write(&temp, json).map_err(io)?;
        std::fs::rename(&temp, &path).map_err(io)?;
    }
    Ok(())
}

/// The fields of a body's `raw_data/world.json` its epochs need.
#[derive(Deserialize)]
#[serde(default)]
struct BodyMeta {
    format_version: u32,
    pressure_bar: f32,
    surface_age_ga: Option<f32>,
    preset: BodyPreset,
}

impl Default for BodyMeta {
    fn default() -> Self {
        Self {
            format_version: 1,
            pressure_bar: 1.0,
            surface_age_ga: None,
            preset: BodyPreset::default(),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BodyPreset {
    geology: BodyGeology,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct BodyGeology {
    wet_until: f32,
}

/// The epochs the body in `dir` can be shown at, in Ma before present
/// (negative in the future), `[future, past]`, or `None` inside when it
/// offers none (liquid water today, no craters); `None` outside when it
/// has no `raw_data/world.json`. See `mapgen::epoch`.
pub fn epochs_at(dir: &Path) -> Option<Option<[i32; 2]>> {
    let bytes = std::fs::read(dir.join("raw_data").join("world.json")).ok()?;
    let meta: BodyMeta = serde_json::from_slice(&bytes).ok()?;
    if meta.format_version < mapgen::export::raw_data::MIN_FORMAT_VERSION {
        return Some(None);
    }
    let range = meta.surface_age_ga.and_then(|age| {
        mapgen::epoch::range_ga(meta.pressure_bar, age, meta.preset.geology.wet_until)
    });
    // Whole Ma, within the range.
    Some(range.map(|(future, past)| {
        [
            (future * 1000.0).ceil() as i32,
            (past * 1000.0).floor() as i32,
        ]
    }))
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
        "star": {"type": "K1V", "designation": "MGS 48213",
                 "info": {"light": [1.0, 0.8, 0.6], "hz_inner_au": 0.6, "hz_outer_au": 1.1}},
        "bodies": [
            {"id": "b", "kind": "planet", "type": "barren", "parent": null, "designation": "MGS 48213 b", "generated": false},
            {"id": "c", "kind": "planet", "type": "terran", "parent": null, "designation": "MGS 48213 c", "generated": true,
             "orbit": {"semi_major_axis_au": 0.9, "eccentricity": 0.1, "node_deg": 300, "periapsis_deg": 90}},
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
        let s = summarize("mgs-48213-42", SYSTEM.as_bytes(), 7).unwrap();
        assert_eq!((s.seed, s.width, s.star.as_str()), (42, 1024, "K1V"));
        assert_eq!(s.designation, "MGS 48213");
        assert_eq!(s.planets.get("terran"), Some(&1));
        assert_eq!(s.planets.len(), 3);
        assert_eq!((s.moons, s.modified_ms), (1, 7));
        let generated: Vec<&str> = s
            .bodies
            .iter()
            .filter(|b| b.generated)
            .map(|b| b.id.as_str())
            .collect();
        assert_eq!(generated, ["c", "c-1"]);
        // What the library's orrery draws, and its defaults when missing.
        assert_eq!(s.star_light, [1.0, 0.8, 0.6]);
        assert_eq!((s.hz_inner_au, s.hz_outer_au), (0.6, 1.1));
        let orbits: Vec<f64> = s.bodies.iter().map(|b| b.orbit_au).collect();
        assert_eq!(orbits, [0.0, 0.9, 0.0, 0.0]);
        assert_eq!(
            (s.bodies[1].eccentricity, s.bodies[1].periapsis_deg),
            (0.1, 30.0)
        );
        assert!(summarize("x-1", b"not json", 0).is_none());
    }

    #[test]
    fn only_directories_with_a_valid_id_are_systems() {
        let root = temp_root("ids");
        std::fs::create_dir_all(root.join("mgs-48213-42")).unwrap();
        std::fs::create_dir_all(root.join(".tmp-mgs-1-1")).unwrap();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        assert_eq!(system_ids(&root), ["mgs-48213-42"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn names_are_checked_and_written_aside() {
        let root = temp_root("names");
        std::fs::create_dir_all(root.join("mgs-48213-42")).unwrap();
        let bodies = summarize("mgs-48213-42", SYSTEM.as_bytes(), 0)
            .unwrap()
            .bodies;
        let given: GivenNames = [
            ("c".into(), " Gaia ".into()),
            ("star".into(), "Helia".into()),
        ]
        .into();
        let names = checked_names(&bodies, &given).unwrap();
        assert_eq!(names.get("c").map(String::as_str), Some("Gaia"));
        let unknown: GivenNames = [("z".into(), "Nyx".into())].into();
        assert_eq!(
            checked_names(&bodies, &unknown),
            Err(NamingError::NoSuchBody("z".into()))
        );
        let blank: GivenNames = [("c".into(), "  ".into())].into();
        assert!(matches!(
            checked_names(&bodies, &blank),
            Err(NamingError::InvalidName(_))
        ));
        write_names_file(&root, "mgs-48213-42", &names).unwrap();
        assert_eq!(given_names(&root, "mgs-48213-42"), names);
        // No names: the file goes.
        write_names_file(&root, "mgs-48213-42", &GivenNames::new()).unwrap();
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
