use crate::world::World;
use std::fs::File;
use std::io::{BufWriter, Write};

/// Writes the full world (metadata and every tile) as compact JSON.
///
/// Compact rather than pretty-printed: a 1920 × 1080 world has two million
/// tiles, and indentation alone roughly doubles the file size.
pub fn export_json(world: &World, path: &str) -> std::io::Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    serde_json::to_writer(&mut out, world)?;
    out.flush()
}
