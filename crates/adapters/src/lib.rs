//! Adapters plugged into the ports of the domain crates: filesystem
//! storage, PNG and SVG exports, the English text catalogue, relief files.
//!
//! The only library crate allowed to touch the outside world.

mod store;

pub use store::{FsStore, StoreError};
