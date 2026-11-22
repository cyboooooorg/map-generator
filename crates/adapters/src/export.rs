//! Export of a world: its file and the PNG projection of one field.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use world::{FieldId, World, WorldError, project};

use crate::png::{Ramp, encode_png};
use crate::store::paths::body_slug;
use crate::store::write_atomic;
use crate::world_codec::encode;

/// The files an export wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exported {
    /// The world file, `<slug>-<keyframe>.world`.
    pub world: PathBuf,
    /// The projection, `<slug>-<keyframe>-<field code>.png`.
    pub png: PathBuf,
}

/// Why an export failed.
#[derive(Debug)]
pub enum ExportError {
    /// A file or directory could not be written.
    Io(io::Error),
    /// The field could not be projected.
    World(WorldError),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Io(error) => write!(f, "export.io({error})"),
            ExportError::World(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<io::Error> for ExportError {
    fn from(error: io::Error) -> ExportError {
        ExportError::Io(error)
    }
}

impl From<WorldError> for ExportError {
    fn from(error: WorldError) -> ExportError {
        ExportError::World(error)
    }
}

/// Writes `world` and the PNG of `field` (`width` pixels wide, coloured
/// through `ramp`) under `out`, creating it when missing.
///
/// The projection is made first: when it fails, nothing is created.
pub fn export(
    out: &Path,
    world: &World,
    field: &FieldId,
    width: u32,
    ramp: &Ramp,
) -> Result<Exported, ExportError> {
    let raster = project(world, field, width)?;
    let stem = format!("{}-{}", body_slug(world.body()), world.keyframe());
    let world_path = out.join(format!("{stem}.world"));
    let png_path = out.join(format!("{stem}-{}.png", field.0.as_str()));
    write_atomic(&world_path, &encode(world))?;
    write_atomic(&png_path, &encode_png(&raster, ramp))?;
    Ok(Exported {
        world: world_path,
        png: png_path,
    })
}
