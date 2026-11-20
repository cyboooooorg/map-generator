//! No member opts out of the workspace lints, and no domain crate allows
//! the determinism and boundary lints away.

use std::fs;
use std::path::Path;

/// Crates held to the strict root `clippy.toml`.
pub const DOMAIN: [&str; 5] = ["science", "timeline", "system", "world", "catalogue"];

/// One line per escape under `root`: a member of `crates/*` or `tools/*`
/// whose manifest lacks `[lints] workspace = true`, or a domain crate
/// source that allows `clippy::disallowed_methods` or `disallowed_types`.
pub fn escapes(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for group in ["crates", "tools"] {
        let mut members: Vec<_> = fs::read_dir(root.join(group))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default();
        members.sort();
        for member in members
            .iter()
            .filter(|dir| dir.join("Cargo.toml").is_file())
        {
            let manifest = fs::read_to_string(member.join("Cargo.toml")).unwrap_or_default();
            let inherits = manifest
                .parse::<toml::Table>()
                .ok()
                .and_then(|table| table.get("lints")?.get("workspace")?.as_bool())
                == Some(true);
            if !inherits {
                found.push(format!(
                    "{}: lacks [lints] workspace = true",
                    member.display()
                ));
            }
        }
    }
    for name in DOMAIN {
        for entry in walkdir::WalkDir::new(root.join("crates").join(name))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        {
            let source = fs::read_to_string(entry.path()).unwrap_or_default();
            let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
            if compact.contains("clippy::disallowed_methods")
                || compact.contains("clippy::disallowed_types")
            {
                found.push(format!(
                    "{}: names a disallowed_* lint (allow or expect it away)",
                    entry.path().display()
                ));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(root: &Path, dir: &str, manifest: &str) {
        fs::create_dir_all(root.join(dir).join("src")).unwrap();
        fs::write(root.join(dir).join("Cargo.toml"), manifest).unwrap();
    }

    #[test]
    fn opting_out_of_the_workspace_lints_is_reported() {
        let root = tempfile::tempdir().unwrap();
        member(
            root.path(),
            "crates/world",
            "[package]\nname = \"world\"\n\n[lints]\nworkspace = true\n",
        );
        member(
            root.path(),
            "crates/system",
            "[package]\nname = \"system\"\n\n[lints.clippy]\ndisallowed_methods = \"allow\"\n",
        );
        member(root.path(), "tools/x", "[package]\nname = \"x\"\n");
        let found = escapes(root.path());
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].ends_with("crates/system: lacks [lints] workspace = true"));
        assert!(found[1].ends_with("tools/x: lacks [lints] workspace = true"));
    }

    #[test]
    fn allowing_a_disallowed_lint_in_a_domain_crate_is_reported() {
        let root = tempfile::tempdir().unwrap();
        member(
            root.path(),
            "crates/world",
            "[package]\nname = \"world\"\n\n[lints]\nworkspace = true\n",
        );
        member(
            root.path(),
            "crates/adapters",
            "[package]\nname = \"adapters\"\n\n[lints]\nworkspace = true\n",
        );
        fs::write(
            root.path().join("crates/world/src/lib.rs"),
            "#![allow( clippy::disallowed_methods )]\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/adapters/src/lib.rs"),
            "#![allow(clippy::disallowed_methods)]\n",
        )
        .unwrap();
        let found = escapes(root.path());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("crates/world/src/lib.rs: names a disallowed_* lint"));
    }
}
