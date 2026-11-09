//! A world made by an older generator (format version 10, 96 x 48 pixels)
//! is refused with a message pointing to `mise run clean-systems`: the
//! generator keeps no code to re-derive it.

use mapgen::export::raw_data::load_layers;
use std::path::PathBuf;

#[test]
fn a_world_of_an_older_format_is_refused_with_the_way_out() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/format10/bodies/e/sizes/96x48");
    let error = load_layers(&dir).err().expect("refused");
    assert!(error.contains("clean-systems"), "{error}");
}
