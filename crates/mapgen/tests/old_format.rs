//! A world made by an older generator (format version 10, 96 x 48 pixels)
//! is refused with a message pointing to the main menu's Regenerate button: the
//! generator keeps no code to re-derive it. The same holds for a system of
//! an older format.

use mapgen::export::raw_data::load_layers;
use std::path::PathBuf;

#[test]
fn a_world_of_an_older_format_is_refused_with_the_way_out() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/format10/bodies/e/sizes/96x48");
    let error = load_layers(&dir).err().expect("refused");
    assert!(error.contains("Regenerate"), "{error}");
}

/// A system stored before lot 2b (format 7: no `bond_albedo`, `snowball` or
/// `secular_w` in its bodies) is refused on its version, without trying to
/// parse the bodies, and with the way out.
#[test]
fn a_system_of_format_7_is_outdated_without_panicking() {
    let dir = std::env::temp_dir().join(format!("mapgen-old-system-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("system.json"),
        r#"{"format_version": 7, "seed": 1, "bodies": [{"id": "a", "model": {"climate": {}}}]}"#,
    )
    .unwrap();
    let error = mapgen::system::read_system(&dir).expect_err("refused");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(error.contains("format version 7"), "{error}");
    assert!(error.contains("Regenerate"), "{error}");
}
