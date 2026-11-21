//! The binary layout of a cached world, little-endian:
//!
//! ```text
//! b"MGWORLD1"  u32 engine  u16 len + designation text  u32 keyframe
//! u32 grid n   u32 field count
//! per field, in code order:
//!   u16 len + code text, u8 type (0 F32, 1 U16, 2 U8),
//!   u8 categorical (0/1), u8 storage (0 exact, 1 U16 then f32 min, f32 max)
//! then the values of each field, one per cell
//! ```
//!
//! An `F32` field with `Storage::U16` is written as `u16`
//! (`round((v - min) / (max - min) * 65535)`, clamped): the file keeps
//! what the field is allowed to lose, and no more.

use std::fmt;

use timeline::{Code, Designation, EngineVersion};
use world::{Field, FieldId, Grid, Storage, Values, World};

/// The file magic. Change it (`MGWORLD2`, ...) whenever the layout above
/// changes: an old file then reads as another file (a cache miss), never
/// as a damaged one.
const MAGIC: &[u8; 8] = b"MGWORLD1";

/// The largest cell count a file may declare: the cell index of a world
/// is a `u32` in the layout. Every grid `Grid::new` builds below it
/// round-trips, including those above the generator's own cap.
const MAX_FILE_CELLS: u64 = u32::MAX as u64;

/// Why a world file could not be decoded.
#[derive(Debug)]
pub enum CodecError {
    /// The file does not start with the world magic.
    Magic,
    /// The file ends before the layout does.
    Truncated,
    /// The bytes do not describe a valid world.
    Invalid(String),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecError::Magic => write!(f, "world.codec.magic"),
            CodecError::Truncated => write!(f, "world.codec.truncated"),
            CodecError::Invalid(why) => write!(f, "world.codec.invalid({why})"),
        }
    }
}

impl std::error::Error for CodecError {}

fn put_text(out: &mut Vec<u8>, text: &str) {
    // Codes and designations are short; a longer text cannot occur.
    let len = u16::try_from(text.len()).expect("text longer than 65535 bytes");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

/// Definition: the quantization step count of `Storage::U16`, the
/// largest `u16`.
const STEPS: f32 = 65535.0;

/// The bytes of `world`.
///
/// # Panics
///
/// When a text is longer than 65 535 bytes, or the field count does not
/// fit a `u32`: neither can come out of the generator.
pub fn encode(world: &World) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&world.engine().0.to_le_bytes());
    put_text(&mut out, &world.body().to_string());
    out.extend_from_slice(&world.keyframe().to_le_bytes());
    out.extend_from_slice(&world.grid().n().to_le_bytes());
    let count = u32::try_from(world.fields().count()).expect("field count");
    out.extend_from_slice(&count.to_le_bytes());
    for (id, field) in world.fields() {
        put_text(&mut out, id.0.as_str());
        out.push(match field.values {
            Values::F32(_) => 0,
            Values::U16(_) => 1,
            Values::U8(_) => 2,
        });
        out.push(u8::from(field.categorical));
        match field.storage {
            Storage::Exact => out.push(0),
            Storage::U16 { min, max } => {
                out.push(1);
                out.extend_from_slice(&min.to_le_bytes());
                out.extend_from_slice(&max.to_le_bytes());
            }
        }
    }
    for (_, field) in world.fields() {
        match (&field.values, field.storage) {
            (Values::F32(values), Storage::U16 { min, max }) => {
                for v in values {
                    let t = ((v - min) / (max - min) * STEPS).round().clamp(0.0, STEPS);
                    // `t` is in 0..=65535 (NaN becomes 0).
                    out.extend_from_slice(&(t as u16).to_le_bytes());
                }
            }
            (Values::F32(values), Storage::Exact) => {
                for v in values {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
            (Values::U16(values), _) => {
                for v in values {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
            (Values::U8(values), _) => out.extend_from_slice(values),
        }
    }
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], CodecError> {
        if self.bytes.len() < n {
            return Err(CodecError::Truncated);
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn f32(&mut self) -> Result<f32, CodecError> {
        Ok(f32::from_le_bytes(self.array()?))
    }

    fn text(&mut self) -> Result<String, CodecError> {
        let len = usize::from(self.u16()?);
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| CodecError::Invalid("text is not UTF-8".to_string()))
    }
}

fn invalid(why: &str) -> CodecError {
    CodecError::Invalid(why.to_string())
}

/// What a field's header says, before its values.
struct Header {
    id: FieldId,
    kind: u8,
    categorical: bool,
    storage: Storage,
}

/// The world in `bytes`.
///
/// # Errors
///
/// `Magic` for another file, `Truncated` when it ends early, `Invalid`
/// for anything else that is not a world (trailing bytes included).
pub fn decode(bytes: &[u8]) -> Result<World, CodecError> {
    if bytes.len() < MAGIC.len() {
        return Err(if MAGIC.starts_with(bytes) {
            CodecError::Truncated
        } else {
            CodecError::Magic
        });
    }
    let mut r = Reader { bytes };
    if r.take(MAGIC.len())? != MAGIC {
        return Err(CodecError::Magic);
    }
    let engine = EngineVersion(r.u32()?);
    let body: Designation = r.text()?.parse().map_err(|_| invalid("designation"))?;
    let keyframe = r.u32()?;
    let n = r.u32()?;
    let cells = u64::from(n) * u64::from(n) * 6;
    if n == 0 || cells > MAX_FILE_CELLS {
        return Err(invalid("grid size"));
    }
    let count = r.u32()?;
    let mut headers = Vec::new();
    for _ in 0..count {
        let id = FieldId(Code::new(r.text()?).map_err(|_| invalid("field code"))?);
        let kind = r.u8()?;
        let categorical = match r.u8()? {
            0 => false,
            1 => true,
            _ => return Err(invalid("categorical flag")),
        };
        let storage = match r.u8()? {
            0 => Storage::Exact,
            1 => Storage::U16 {
                min: r.f32()?,
                max: r.f32()?,
            },
            _ => return Err(invalid("storage")),
        };
        headers.push(Header {
            id,
            kind,
            categorical,
            storage,
        });
    }
    // Nothing is allocated from the header before the file proves it
    // holds the values: the bytes left must cover every field.
    let cells = usize::try_from(cells).map_err(|_| invalid("grid size"))?;
    let mut needed = 0usize;
    for header in &headers {
        let size = match (header.kind, header.storage) {
            (0, Storage::Exact) => 4,
            (0, Storage::U16 { .. }) | (1, _) => 2,
            (2, _) => 1,
            _ => return Err(invalid("field type")),
        };
        needed = needed.saturating_add(cells.saturating_mul(size));
    }
    if r.bytes.len() < needed {
        return Err(CodecError::Truncated);
    }
    let grid = Grid::new(n);
    let mut world = World::new(grid, body, keyframe, engine);
    for header in headers {
        let values = match (header.kind, header.storage) {
            (0, Storage::U16 { min, max }) => {
                let raw = r.take(cells * 2)?;
                Values::F32(
                    raw.as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| {
                            let q = f32::from(u16::from_le_bytes(*c));
                            min + q / STEPS * (max - min)
                        })
                        .collect(),
                )
            }
            (0, Storage::Exact) => Values::F32(
                r.take(cells * 4)?
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| f32::from_le_bytes(*c))
                    .collect(),
            ),
            (1, _) => Values::U16(
                r.take(cells * 2)?
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c))
                    .collect(),
            ),
            (2, _) => Values::U8(r.take(cells)?.to_vec()),
            _ => return Err(invalid("field type")),
        };
        world
            .insert(
                header.id,
                Field {
                    values,
                    categorical: header.categorical,
                    storage: header.storage,
                },
            )
            .map_err(|error| CodecError::Invalid(error.to_string()))?;
    }
    if !r.bytes.is_empty() {
        return Err(invalid("trailing bytes"));
    }
    Ok(world)
}
