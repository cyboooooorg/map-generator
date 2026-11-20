//! READMEs stay short: detail goes into `docs/`, never into a README.

use std::fs;
use std::path::Path;

/// Line limit of the root `README.md`.
pub const ROOT_LIMIT: usize = 300;
/// Line limit of each crate's `README.md`.
pub const CRATE_LIMIT: usize = 150;

/// One line per README missing or over its limit: the root one, and one
/// per member directory of `crates/*` and `tools/*` under `root`.
pub fn too_long(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut check = |path: &Path, limit: usize| match fs::read_to_string(path) {
        Ok(text) => {
            let lines = text.lines().count();
            if lines > limit {
                found.push(format!("{}: {lines} lines, limit {limit}", path.display()));
            }
        }
        Err(_) => found.push(format!("{}: missing", path.display())),
    };
    check(&root.join("README.md"), ROOT_LIMIT);
    for group in ["crates", "tools"] {
        let mut members: Vec<_> = fs::read_dir(root.join(group))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default();
        members.retain(|dir| dir.join("Cargo.toml").is_file());
        members.sort();
        for dir in members {
            check(&dir.join("README.md"), CRATE_LIMIT);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(count: usize) -> String {
        "line\n".repeat(count)
    }

    #[test]
    fn readmes_at_their_limit_pass_and_one_line_more_fails() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path();
        for dir in ["crates/ok", "crates/long", "tools/bare"] {
            fs::create_dir_all(path.join(dir)).unwrap();
            fs::write(path.join(dir).join("Cargo.toml"), "").unwrap();
        }
        fs::write(path.join("README.md"), lines(301)).unwrap();
        fs::write(path.join("crates/ok/README.md"), lines(150)).unwrap();
        fs::write(path.join("crates/long/README.md"), lines(151)).unwrap();
        let found = too_long(path);
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[0].ends_with("README.md: 301 lines, limit 300"));
        assert!(found[1].contains("crates/long/README.md: 151 lines, limit 150"));
        assert!(found[2].contains("tools/bare/README.md: missing"));
        fs::write(path.join("tools/bare/README.md"), lines(3)).unwrap();

        fs::write(path.join("README.md"), lines(300)).unwrap();
        fs::write(path.join("crates/long/README.md"), lines(150)).unwrap();
        assert_eq!(too_long(path), Vec::<String>::new());
    }
}

#[cfg(test)]
mod missing_root {
    #[test]
    fn a_missing_root_readme_is_reported() {
        let root = tempfile::tempdir().unwrap();
        let found = super::too_long(root.path());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].ends_with("README.md: missing"));
    }
}
