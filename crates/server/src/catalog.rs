//! Archetypes the server accepts: the built-in ones plus every `.toml`
//! file under the archetypes directory that is not a built-in.
//!
//! Requests name an archetype by its catalog `spec`, never by a raw path, so
//! a client cannot make the server read arbitrary files. Archetypes are
//! parsed once, at startup: a changed or added file is picked up on restart.
use crate::api::ArchetypeInfo;
use mapgen::archetype::{Archetype, built_in_names};
use mapgen::giants::GiantsMode;
use mapgen::moons::MoonsMode;
use mapgen::request::WorldRequest;
use rand::SeedableRng;
use rand::rngs::StdRng;
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
                // A name names one archetype: the form's labels and a
                // world's `archetype` go by it.
                Ok(a) if entries.iter().any(|(info, _)| info.name == a.name) => {
                    tracing::warn!("skipping archetype {spec}: its name {} is taken", a.name);
                }
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
        giant: renders_as_giant(a),
        moon: a.moon.is_some(),
        moon_host: a.moon.as_ref().map(|m| m.host.clone()),
        locales: a.locales.clone(),
    }
}

/// Whether the planet of `a`, asked alone at seed 0 (its ranges drawn
/// once, its star and orbit by a generator seeded 0), renders as a giant:
/// what the model derives, not a mass threshold. An archetype whose
/// ranges straddle the giants is read at that one draw.
fn renders_as_giant(a: &Archetype) -> bool {
    let request = WorldRequest {
        seed: Some(0),
        moons: Some(MoonsMode::None),
        giants: Some(GiantsMode::None),
        width: Some(64),
        height: Some(32),
        ..WorldRequest::default()
    };
    request
        .resolve_with_archetype(Some(a.clone()), &mut StdRng::seed_from_u64(0))
        .is_ok_and(|r| r.params.giant_mass_earth.is_some())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory under the system's temporary one, emptied.
    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mapgen-catalog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_giants_are_the_ones_the_model_makes() {
        let catalog = Catalog::load(&temp_dir("giants"));
        let giants: Vec<String> = catalog
            .list()
            .into_iter()
            .filter(|a| a.giant)
            .map(|a| a.name)
            .collect();
        assert_eq!(
            giants,
            [
                "jupiter-like",
                "saturn-like",
                "uranus-like",
                "neptune-like",
                "mini-neptune",
                "hot-jupiter"
            ]
        );
    }

    #[test]
    fn a_file_with_a_name_taken_is_skipped() {
        let dir = temp_dir("taken");
        std::fs::write(
            dir.join("mine.toml"),
            "name = \"earth-like\"\n[inputs]\nmass_earth = 1.0\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("heavy-rock.toml"),
            "name = \"heavy-rock\"\n[inputs]\nmass_earth = [6.0, 8.0]\nenvelope_fraction = 0.0\n",
        )
        .unwrap();
        let catalog = Catalog::load(&dir);
        let list = catalog.list();
        assert_eq!(list.iter().filter(|a| a.name == "earth-like").count(), 1);
        assert!(catalog.get("mine.toml").is_none());
        // Heavy, but bare: a rocky super-Earth, not a giant.
        let rock = list.iter().find(|a| a.name == "heavy-rock").unwrap();
        assert!(!rock.giant);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
