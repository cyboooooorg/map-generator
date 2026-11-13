//! Archetypes the server accepts: the built-in ones plus every `.toml`
//! file under the archetypes directory that is not a built-in.
//!
//! Requests name an archetype by its catalog `spec`, never by a raw path, so
//! a client cannot make the server read arbitrary files. Archetypes are
//! parsed once, at startup: a changed or added file is picked up on restart.
use crate::api::ArchetypeInfo;
use mapgen::archetype::{Archetype, built_in_names};
use std::path::{Path, PathBuf};

pub struct Catalog {
    entries: Vec<(ArchetypeInfo, Archetype)>,
}

impl Catalog {
    /// Loads the built-in archetypes and the archetype files under `dir`.
    /// Invalid files are skipped with a warning.
    pub fn load(dir: &Path) -> Self {
        let mut entries: Vec<(ArchetypeInfo, Archetype)> = Archetype::built_in()
            .into_iter()
            .map(|a| {
                let spec = a.name.clone();
                (info(&a, &spec, true), a)
            })
            .collect();

        let mut files = Vec::new();
        collect_toml(dir, &mut files);
        files.sort();
        for path in files {
            let Ok(relative) = path.strip_prefix(dir) else {
                continue;
            };
            let spec = relative.to_string_lossy().replace('\\', "/");
            // archetypes/<name>.toml are the built-in sources: already listed.
            if built_in_names()
                .iter()
                .any(|name| spec == format!("{name}.toml"))
            {
                continue;
            }
            match Archetype::load(&path.to_string_lossy()) {
                Ok(a) => entries.push((info(&a, &spec, false), a)),
                Err(e) => tracing::warn!("skipping archetype {spec}: {e}"),
            }
        }
        Self { entries }
    }

    /// The archetype a request's `planet` names (its catalog `spec`).
    pub fn get(&self, spec: &str) -> Option<&Archetype> {
        self.entries
            .iter()
            .find(|(info, _)| info.spec == spec)
            .map(|(_, a)| a)
    }

    pub fn list(&self) -> Vec<ArchetypeInfo> {
        self.entries.iter().map(|(info, _)| info.clone()).collect()
    }
}

fn info(a: &Archetype, spec: &str, built_in: bool) -> ArchetypeInfo {
    ArchetypeInfo {
        spec: spec.to_string(),
        name: a.name.clone(),
        description: a.description.clone(),
        built_in,
        inputs: a.inputs.clone(),
        orbit: a.orbit.clone(),
        locales: a.locales.clone(),
    }
}

fn collect_toml(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_toml(&path, out);
        } else if path.extension().is_some_and(|e| e == "toml") {
            out.push(path);
        }
    }
}
