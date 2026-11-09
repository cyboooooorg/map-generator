//! The export of a system (`GET /api/systems/{id}/export`): one `.zip`
//! that holds what the user needs to keep it or draw it again, since the
//! app makes no backup of its own (`docs/analysis/15-storage.md`,
//! decision 6).
//!
//! ```text
//! <system-id>.zip
//! └── <system-id>/
//!     ├── system.json                    the [`SystemExport`] document
//!     └── bodies/<body>/                 each generated body
//!         ├── thumb.png
//!         └── sizes/<W>x<H>/{world,legend}.png   each resolution on disk
//! ```
//!
//! Tiles, raw data, SVGs and noise maps are left out: they are rebuilt
//! from the seed. A body never generated (or whose files are gone) is
//! still listed in `system.json`, with its seed and its request.
use crate::index::Index;
use crate::systems;
use mapgen::system::{System, SystemRequest};
use mapgen::world::WorldParams;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// The version of the `system.json` document of an export. Bumped when a
/// field is removed or changes its meaning; adding a field does not.
pub const EXPORT_SCHEMA: u32 = 1;

/// `system.json` of an export: everything needed to draw the system again
/// from its seeds, and to know what each body was made with.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemExport {
    /// [`EXPORT_SCHEMA`].
    pub schema: u32,
    /// When the export was made, UTC, ISO 8601 (`2026-11-09T14:03:21Z`).
    pub exported_at: String,
    /// The generator that wrote this export.
    pub generator: ExportGenerator,
    /// The system's id (its directory name), also the archive's name.
    pub id: String,
    pub seed: u32,
    /// The request the system was drawn from, as stored: a seed left to
    /// chance is not in it, nor what a request around a planet left open
    /// (see `create`).
    pub request: SystemRequest,
    /// The body of the `POST /api/systems` that draws this system again,
    /// with the same id and the same seeds: `request` with its seed filled
    /// in and, around a planet, the parameters the planet asked for was
    /// resolved to (its star, orbit and world) pinned, see [`create`].
    pub create: SystemRequest,
    /// The star's designation (`MGS 48213`) and spectral type.
    pub star: ExportStar,
    /// Every body, planets and moons, in the system's order.
    pub bodies: Vec<ExportBody>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportGenerator {
    /// The `mapgen-server` crate version that wrote the export.
    pub version: String,
    /// `system.json` layout of the stored system (`SYSTEM_FORMAT_VERSION`).
    pub system_format_version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportStar {
    pub designation: String,
    #[serde(rename = "type")]
    pub spectral: String,
    /// The name the user gave the system, if any.
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportBody {
    pub id: String,
    pub designation: String,
    /// The name the user gave the body, if any.
    pub name: Option<String>,
    /// `planet` or `moon`.
    pub kind: String,
    /// Its preset: a planet type, `rocky-moon` or `icy-moon`.
    #[serde(rename = "type")]
    pub body_type: String,
    /// The planet a moon orbits.
    pub parent: Option<String>,
    pub seed: u32,
    /// The complete request that generates the body (`WorldParams`, as
    /// `system.json` stores it).
    pub request: WorldParams,
    /// Whether the archive holds images of the body.
    pub generated: bool,
    /// What generated the body; `None` when it is not generated.
    pub made_with: Option<MadeWith>,
    /// The images of the archive, relative to the system's directory.
    pub files: Vec<String>,
}

/// The generator version a body was made with, as the index records it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MadeWith {
    /// The crate version; `None` for a body found in the tree, made before
    /// the index recorded it.
    pub version: Option<String>,
    /// `FORMAT_VERSION` of the body's `world.json`.
    pub format_version: Option<u32>,
}

/// Why an export failed.
#[derive(Debug)]
pub enum ExportError {
    /// `system.json` could not be read (missing, damaged or too old).
    System(String),
    Other(String),
}

/// The request with its seed filled in: what draws the same system again.
pub fn with_seed(request: &SystemRequest, seed: u32) -> SystemRequest {
    let mut request = request.clone();
    match &mut request {
        SystemRequest::System(whole) => whole.seed = Some(seed),
        SystemRequest::Around(world) => world.seed = Some(seed),
    }
    request
}

/// The request that draws `system` again, with the same id and seeds: its
/// request with the seed filled in. Around a planet, the fields the request
/// left open (its star, orbit, world parameters) were resolved by a random
/// generator, not by the seed: they are pinned to the values the planet
/// asked for was resolved to, as its body stores them
/// ([`mapgen::request::WorldRequest::pinned`]).
pub fn create(system: &System) -> SystemRequest {
    let focus = system
        .focus
        .as_ref()
        .and_then(|id| system.bodies.iter().find(|b| b.id == *id));
    match (&system.request, focus) {
        (SystemRequest::Around(world), Some(body)) => with_seed(
            &SystemRequest::Around(world.pinned(&body.params, &body.params.planet.name)),
            system.seed,
        ),
        (request, _) => with_seed(request, system.seed),
    }
}

/// Builds the archive of system `id` under `root`. Blocking: reads the
/// system's files.
pub fn build_archive(
    root: &Path,
    id: &str,
    index: &Index,
    now_ms: u64,
) -> Result<Vec<u8>, ExportError> {
    let dir = systems::system_dir(root, id);
    let system = mapgen::system::read_system(&dir).map_err(ExportError::System)?;
    let names = index
        .names(id)
        .map_err(|e| ExportError::Other(e.to_string()))?;
    let mut images: Vec<(String, std::path::PathBuf)> = Vec::new();
    let mut bodies = Vec::with_capacity(system.bodies.len());
    for body in &system.bodies {
        let files = body_images(&dir, &body.id);
        let made_with = if files.is_empty() {
            None
        } else {
            index
                .generator_of(id, &body.id)
                .map_err(|e| ExportError::Other(e.to_string()))?
                .map(|g| MadeWith {
                    version: g.version,
                    format_version: g.format,
                })
        };
        bodies.push(ExportBody {
            id: body.id.clone(),
            designation: body.designation.clone(),
            name: names.get(&body.id).cloned(),
            kind: format!("{:?}", body.kind).to_lowercase(),
            body_type: body.body_type.clone(),
            parent: body.parent.clone(),
            seed: body.params.seed,
            request: body.params.clone(),
            generated: !files.is_empty(),
            made_with,
            files: files.iter().map(|(name, _)| name.clone()).collect(),
        });
        images.extend(files);
    }
    let document = document(&system, id, names, bodies, now_ms);
    let json =
        serde_json::to_vec_pretty(&document).map_err(|e| ExportError::Other(e.to_string()))?;
    zip_up(id, &json, &images).map_err(|e| ExportError::Other(e.to_string()))
}

fn document(
    system: &System,
    id: &str,
    mut names: BTreeMap<String, String>,
    bodies: Vec<ExportBody>,
    now_ms: u64,
) -> SystemExport {
    SystemExport {
        schema: EXPORT_SCHEMA,
        exported_at: iso8601_utc(now_ms),
        generator: ExportGenerator {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            system_format_version: system.format_version,
        },
        id: id.to_owned(),
        seed: system.seed,
        request: system.request.clone(),
        create: create(system),
        star: ExportStar {
            designation: system.star.designation.clone(),
            spectral: system.star.spectral.clone(),
            name: names.remove(systems::STAR),
        },
        bodies,
    }
}

/// The PNGs of a body that exist: its thumbnail, then `world.png` and
/// `legend.png` of each resolution, smallest first. Each is its path in
/// the system's directory (also its path in the archive) and its file.
/// A thumbnail without a resolution beside it does not make a generated
/// body: none then.
fn body_images(system_dir: &Path, body: &str) -> Vec<(String, std::path::PathBuf)> {
    let body_dir = system_dir.join(systems::BODIES_DIR).join(body);
    let relative = format!("{}/{body}", systems::BODIES_DIR);
    let mut found = Vec::new();
    let mut add = |name: String, path: std::path::PathBuf| {
        if path.is_file() {
            found.push((name, path));
        }
    };
    for (w, h) in mapgen::sizes::sizes_on_disk(&body_dir) {
        let size = mapgen::sizes::size_name(w, h);
        let size_dir = mapgen::sizes::size_dir(&body_dir, w, h);
        for file in ["world.png", "legend.png"] {
            add(
                format!("{relative}/{}/{size}/{file}", mapgen::sizes::SIZES_DIR),
                size_dir.join(file),
            );
        }
    }
    if !found.is_empty() {
        let thumb = body_dir.join("thumb.png");
        if thumb.is_file() {
            found.insert(0, (format!("{relative}/thumb.png"), thumb));
        }
    }
    found
}

fn zip_up(
    id: &str,
    json: &[u8],
    images: &[(String, std::path::PathBuf)],
) -> Result<Vec<u8>, zip::result::ZipError> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    // PNGs are compressed already: stored as they are.
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(format!("{id}/{}", systems::SYSTEM_FILE), deflated)?;
    zip.write_all(json)?;
    for (name, path) in images {
        match std::fs::read(path) {
            Ok(bytes) => {
                zip.start_file(format!("{id}/{name}"), stored)?;
                zip.write_all(&bytes)?;
            }
            // Gone since it was listed (a resolution deleted meanwhile).
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(zip.finish()?.into_inner())
}

/// `ms` since the Unix epoch as `YYYY-MM-DDThh:mm:ssZ`.
fn iso8601_utc(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_iso_8601_in_utc() {
        assert_eq!(iso8601_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_utc(951_782_400_000), "2000-02-29T00:00:00Z");
        assert_eq!(iso8601_utc(1_794_233_001_000), "2026-11-09T14:03:21Z");
    }

    #[test]
    fn the_seed_is_filled_into_either_kind_of_request() {
        let whole: SystemRequest = serde_json::from_str(r#"{"system": {"star": "K1V"}}"#).unwrap();
        let around: SystemRequest =
            serde_json::from_str(r#"{"around": {"planet": "barren"}}"#).unwrap();
        for request in [whole, around] {
            let json = serde_json::to_value(with_seed(&request, 77)).unwrap();
            let inner = json.as_object().unwrap().values().next().unwrap();
            assert_eq!(inner["seed"], 77, "{json}");
        }
    }
}
