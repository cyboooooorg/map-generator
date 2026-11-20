//! READMEs stay short: detail goes into `docs/`, never into a README.

use std::fs;
use std::path::Path;

/// Line limit of the root `README.md`.
pub const ROOT_LIMIT: usize = 300;
/// Line limit of each crate's `README.md`.
pub const CRATE_LIMIT: usize = 150;

/// One line per README over its limit: the root one and every
/// `crates/*/README.md` under `root`.
pub fn too_long(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut check = |path: &Path, limit: usize| {
        if let Ok(text) = fs::read_to_string(path) {
            let lines = text.lines().count();
            if lines > limit {
                found.push(format!("{}: {lines} lines, limit {limit}", path.display()));
            }
        }
    };
    check(&root.join("README.md"), ROOT_LIMIT);
    let mut crates: Vec<_> = fs::read_dir(root.join("crates"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect()
        })
        .unwrap_or_default();
    crates.sort();
    for dir in crates {
        check(&dir.join("README.md"), CRATE_LIMIT);
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
        fs::create_dir_all(path.join("crates/ok")).unwrap();
        fs::create_dir_all(path.join("crates/long")).unwrap();
        fs::write(path.join("README.md"), lines(301)).unwrap();
        fs::write(path.join("crates/ok/README.md"), lines(150)).unwrap();
        fs::write(path.join("crates/long/README.md"), lines(151)).unwrap();
        let found = too_long(path);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].ends_with("README.md: 301 lines, limit 300"));
        assert!(found[1].contains("crates/long/README.md: 151 lines, limit 150"));

        fs::write(path.join("README.md"), lines(300)).unwrap();
        fs::write(path.join("crates/long/README.md"), lines(150)).unwrap();
        assert_eq!(too_long(path), Vec::<String>::new());
    }
}
