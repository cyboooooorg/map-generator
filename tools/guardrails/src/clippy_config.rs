//! Every `clippy.toml` of the workspace keeps the determinism entries.
//!
//! clippy reads the nearest `clippy.toml` without merging, so the relaxed
//! copies of `crates/adapters`, `crates/cli` and `tools/*` would silently
//! drop an entry added to the root one.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Methods every configuration forbids: random sources outside the seed
/// hierarchy, random hasher keys and the wall clock.
pub const SHARED_METHODS: [&str; 13] = [
    "rand::rng",
    "rand::thread_rng",
    "rand::random",
    "rand::make_rng",
    "rand::SeedableRng::from_os_rng",
    "rand::SeedableRng::from_entropy",
    "rand::SeedableRng::try_from_os_rng",
    "getrandom::fill",
    "getrandom::getrandom",
    "getrandom::u32",
    "getrandom::u64",
    "std::hash::RandomState::new",
    "std::time::SystemTime::now",
];

/// Types every configuration forbids: maps whose iteration order depends
/// on a random hasher, and ambient random generators.
pub const SHARED_TYPES: [&str; 5] = [
    "std::collections::HashMap",
    "std::collections::HashSet",
    "std::hash::RandomState",
    "rand::rngs::OsRng",
    "rand::rngs::ThreadRng",
];

/// Directories allowed a relaxed copy (relative to the workspace root,
/// `tools/*` matched by its first component): the crates that may touch
/// the outside world.
pub const RELAXED: [&str; 3] = ["crates/adapters", "crates/cli", "tools"];

/// Names clippy reads its configuration from.
const NAMES: [&str; 2] = ["clippy.toml", ".clippy.toml"];

/// Every clippy configuration under `root` (`clippy.toml` and
/// `.clippy.toml`), outside `target/`, `web/`, `node_modules/` and `.git/`.
pub fn configs(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            entry.depth() == 0
                || !(name == ".git" || name == "target" || name == "web" || name == "node_modules")
        })
        .filter_map(Result::ok)
        .filter(|entry| NAMES.iter().any(|name| entry.file_name() == *name))
        .map(|entry| entry.into_path())
        .collect();
    found.sort();
    found
}

/// One line per configuration that relaxes the root one where it may not:
/// a hidden `.clippy.toml` anywhere, or a `clippy.toml` outside the root
/// and the [`RELAXED`] directories (a domain crate would escape its bans).
pub fn unexpected_configs(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for path in configs(root) {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let dir = relative.parent().unwrap_or(Path::new(""));
        let hidden = relative
            .file_name()
            .is_some_and(|name| name == ".clippy.toml");
        let allowed = dir.as_os_str().is_empty()
            || RELAXED.iter().any(|relaxed| {
                let relaxed = Path::new(relaxed);
                dir == relaxed || (relaxed == Path::new("tools") && dir.parent() == Some(relaxed))
            });
        if hidden || !allowed {
            found.push(format!(
                "{}: only the root and crates/adapters, crates/cli, tools/* may have a clippy.toml",
                relative.display()
            ));
        }
    }
    found
}

/// One line per shared entry missing from a configuration under `root`.
pub fn missing_shared_entries(root: &Path) -> Vec<String> {
    let mut missing = Vec::new();
    for path in configs(root) {
        let text = fs::read_to_string(&path).unwrap_or_default();
        let (methods, types) = match text.parse::<toml::Table>() {
            Ok(table) => (
                paths(&table, "disallowed-methods"),
                paths(&table, "disallowed-types"),
            ),
            Err(error) => {
                missing.push(format!("{}: does not parse: {error}", path.display()));
                continue;
            }
        };
        for (wanted, present, key) in [
            (&SHARED_METHODS[..], &methods, "disallowed-methods"),
            (&SHARED_TYPES[..], &types, "disallowed-types"),
        ] {
            for entry in wanted {
                if !present.contains(*entry) {
                    missing.push(format!("{}: {key} lacks {entry}", path.display()));
                }
            }
        }
    }
    missing
}

fn paths(table: &toml::Table, key: &str) -> BTreeSet<String> {
    table
        .get(key)
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| match entry {
            toml::Value::String(path) => Some(path.clone()),
            toml::Value::Table(table) => table
                .get("path")
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> String {
        format!(
            "disallowed-methods = [{}]\ndisallowed-types = [{}]\n",
            SHARED_METHODS
                .map(|p| format!("{{ path = \"{p}\" }}"))
                .join(", "),
            SHARED_TYPES.map(|p| format!("\"{p}\"")).join(", "),
        )
    }

    /// Missing entries when the root holds every shared entry and
    /// `crates/cli/clippy.toml` holds `copy`.
    fn with_copy(copy: &str) -> Vec<String> {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("clippy.toml"), full()).unwrap();
        fs::create_dir_all(root.path().join("crates/cli")).unwrap();
        fs::write(root.path().join("crates/cli/clippy.toml"), copy).unwrap();
        missing_shared_entries(root.path())
    }

    #[test]
    fn copies_outside_the_relaxed_crates_are_reported() {
        let root = tempfile::tempdir().unwrap();
        for dir in [
            "",
            "crates/cli",
            "tools/opacity-table",
            "crates/world",
            "crates/system",
        ] {
            fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        for file in [
            "clippy.toml",
            "crates/cli/clippy.toml",
            "tools/opacity-table/clippy.toml",
            "crates/world/.clippy.toml",
            "crates/system/clippy.toml",
        ] {
            fs::write(root.path().join(file), full()).unwrap();
        }
        let found = unexpected_configs(root.path());
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].starts_with("crates/system/clippy.toml:"));
        assert!(found[1].starts_with("crates/world/.clippy.toml:"));
    }

    #[test]
    fn a_complete_copy_passes() {
        assert_eq!(with_copy(&full()), Vec::<String>::new());
    }

    #[test]
    fn a_relaxed_copy_without_a_random_source_is_reported() {
        let copy = full().replace("{ path = \"rand::thread_rng\" }, ", "");
        let missing = with_copy(&copy);
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(
            missing[0]
                .ends_with("crates/cli/clippy.toml: disallowed-methods lacks rand::thread_rng")
        );
    }

    #[test]
    fn an_unparsable_copy_is_reported() {
        let missing = with_copy("disallowed-methods = [");
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(missing[0].contains("crates/cli/clippy.toml: does not parse"));
    }

    #[test]
    fn a_relaxed_copy_without_hash_map_is_reported() {
        let missing = with_copy(&full().replace("\"std::collections::HashMap\", ", ""));
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(
            missing[0].ends_with(
                "crates/cli/clippy.toml: disallowed-types lacks std::collections::HashMap"
            )
        );
    }
}
