//! Every `clippy.toml` of the workspace keeps the determinism entries.
//!
//! clippy reads the nearest `clippy.toml` without merging, so the relaxed
//! copies of `crates/adapters`, `crates/cli` and `tools/*` would silently
//! drop an entry added to the root one.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Methods every configuration forbids: random sources outside the seed
/// hierarchy and the wall clock.
pub const SHARED_METHODS: [&str; 7] = [
    "rand::rng",
    "rand::thread_rng",
    "rand::random",
    "rand::make_rng",
    "getrandom::fill",
    "getrandom::getrandom",
    "std::time::SystemTime::now",
];

/// Types every configuration forbids: maps whose iteration order depends
/// on a random hasher.
pub const SHARED_TYPES: [&str; 2] = ["std::collections::HashMap", "std::collections::HashSet"];

/// Every `clippy.toml` under `root`, outside `target/`, `web/` and hidden
/// directories.
pub fn configs(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            entry.depth() == 0
                || !(name.starts_with('.')
                    || name == "target"
                    || name == "web"
                    || name == "node_modules")
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() == "clippy.toml")
        .map(|entry| entry.into_path())
        .collect();
    found.sort();
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

    #[test]
    fn a_relaxed_copy_without_hash_map_is_reported() {
        let root = tempfile::tempdir().unwrap();
        let full = format!(
            "disallowed-methods = [{}]\ndisallowed-types = [{}]\n",
            SHARED_METHODS
                .map(|p| format!("{{ path = \"{p}\" }}"))
                .join(", "),
            SHARED_TYPES.map(|p| format!("\"{p}\"")).join(", "),
        );
        fs::write(root.path().join("clippy.toml"), &full).unwrap();
        fs::create_dir_all(root.path().join("crates/cli")).unwrap();
        let relaxed = full.replace("\"std::collections::HashMap\", ", "");
        fs::write(root.path().join("crates/cli/clippy.toml"), relaxed).unwrap();
        let missing = missing_shared_entries(root.path());
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(
            missing[0].ends_with(
                "crates/cli/clippy.toml: disallowed-types lacks std::collections::HashMap"
            )
        );
    }
}
