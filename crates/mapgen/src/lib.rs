//! Procedural planet generation.
//!
//! [`request::WorldRequest`] describes what to generate; resolving it gives
//! complete [`world::WorldParams`]. [`generation::generate`] runs every pass
//! (noise, erosion, climate, rivers, biomes) and [`export::export_world`]
//! writes the maps and raw data. The diagnostic maps are written during the
//! generation, by an [`export::NoiseMapWriter`] passed as its debug sink.
pub mod atmosphere;
pub mod biome;
pub mod carve;
pub mod climate;
pub mod clouds;
pub mod craters;
pub mod delta;
pub mod detail;
pub mod epoch;
pub mod erosion;
pub mod export;
pub mod generation;
pub mod geo;
pub mod geology;
pub mod hex;
pub mod hydrology;
pub mod noise;
pub mod preset;
pub mod request;
pub mod rivers;
pub mod world;
