//! Planet presets the server accepts: the built-in ones plus every `.toml`
//! file under the presets directory that is not a built-in, nor a moon
//! preset (`moons/`).
//!
//! Requests name a preset by its catalog `spec`, never by a raw path, so a
//! client cannot make the server read arbitrary files. Presets are parsed
//! once, at startup: a changed or added file is picked up on restart.
use crate::api::PresetInfo;
use mapgen::preset::{Preset, built_in_names};
use std::path::{Path, PathBuf};

pub struct Catalog {
    entries: Vec<(PresetInfo, Preset)>,
}

impl Catalog {
    /// Loads the built-in presets and the preset files under `dir`. Invalid
    /// files are skipped with a warning.
    pub fn load(dir: &Path) -> Self {
        let mut entries: Vec<(PresetInfo, Preset)> = Preset::built_in()
            .into_iter()
            .map(|p| {
                let spec = p.name.clone();
                (info(&p, &spec, true), p)
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
            // presets/moons/ holds the presets of moons, not planet types.
            if spec.starts_with("moons/") {
                continue;
            }
            // presets/<name>.toml are the built-in sources: already listed.
            if built_in_names()
                .iter()
                .any(|name| spec == format!("{name}.toml"))
            {
                continue;
            }
            match Preset::load(&path.to_string_lossy()) {
                Ok(p) => entries.push((info(&p, &spec, false), p)),
                Err(e) => tracing::warn!("skipping preset {spec}: {e}"),
            }
        }
        Self { entries }
    }

    pub fn list(&self) -> Vec<PresetInfo> {
        self.entries.iter().map(|(info, _)| info.clone()).collect()
    }

    /// The preset of a catalog spec.
    pub fn resolve(&self, spec: &str) -> Option<Preset> {
        self.entries
            .iter()
            .find(|(info, _)| info.spec == spec)
            .map(|(_, preset)| preset.clone())
    }
}

fn info(p: &Preset, spec: &str, built_in: bool) -> PresetInfo {
    PresetInfo {
        spec: spec.to_string(),
        name: p.name.clone(),
        description: p.description.clone(),
        built_in,
        defaults: p.defaults.clone(),
        star: p.star.clone(),
        giant: p.giant.clone(),
        locales: p.locales.clone(),
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
