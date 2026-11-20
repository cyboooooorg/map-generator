//! Architecture checks on the real workspace (tier 1).

use std::fs;
use std::path::{Path, PathBuf};

use guardrails::{clippy_config, constants, deps, readme};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn dependency_graph_points_inwards() {
    let meta = cargo_metadata::MetadataCommand::new()
        .current_dir(root())
        .no_deps()
        .exec()
        .unwrap();
    let violations = deps::violations(&deps::graph(&meta));
    assert!(
        violations.is_empty(),
        "dependencies against the architecture:\n{}",
        violations.join("\n")
    );
}

#[test]
fn science_constants_are_sourced() {
    let mut missing = Vec::new();
    let mut scanned = 0;
    for entry in walkdir::WalkDir::new(root().join("crates/science/src"))
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.path().extension().is_some_and(|ext| ext == "rs") {
            scanned += 1;
            let source = fs::read_to_string(entry.path()).unwrap();
            for name in constants::undocumented(&source) {
                missing.push(format!("{}: {name}", entry.path().display()));
            }
        }
    }
    assert!(scanned > 0, "no source file found under crates/science/src");
    assert!(
        missing.is_empty(),
        "constants without a doc comment starting with {:?}:\n{}",
        constants::PREFIXES,
        missing.join("\n")
    );
}

#[test]
fn readmes_stay_short() {
    let long = readme::too_long(&root());
    assert!(
        long.is_empty(),
        "READMEs over their limit:\n{}",
        long.join("\n")
    );
}

#[test]
fn clippy_configs_share_the_determinism_entries() {
    let configs = clippy_config::configs(&root());
    assert!(
        configs.len() >= 4,
        "expected the root and the relaxed copies: {configs:?}"
    );
    let missing = clippy_config::missing_shared_entries(&root());
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}
