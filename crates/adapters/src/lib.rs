//! Adapters plugged into the ports of the domain crates: filesystem
//! storage, PNG and SVG exports, the English text catalogue, relief files.
//!
//! The only library crate allowed to touch the outside world.

mod store;
mod world_codec;

pub use store::{FsStore, StoreError};
pub use world_codec::{CodecError, decode, encode};
