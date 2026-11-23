//! Adapters plugged into the ports of the domain crates: filesystem
//! storage, PNG and SVG exports, the English text catalogue, relief files.
//!
//! The only library crate allowed to touch the outside world.

mod export;
mod png;
mod store;
mod world_codec;

pub use export::{ExportError, Exported, export};
pub use png::{Ramp, encode_png, write_png};
pub use store::{FsStore, StoreError};
pub use world_codec::{CodecError, decode, encode};
