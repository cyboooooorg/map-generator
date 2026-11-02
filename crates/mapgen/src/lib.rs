//! Procedural planet generation.
//!
//! [`request::WorldRequest`] describes what to generate; resolving it gives
//! complete [`world::WorldParams`]. [`generation::generate`] runs every pass
//! (noise, erosion, climate, rivers, biomes) and [`export::export_world`]
//! writes the maps, raw data and diagnostic maps.
pub mod biome;
pub mod climate;
pub mod detail;
pub mod erosion;
pub mod export;
pub mod generation;
pub mod geo;
pub mod hex;
pub mod hydrology;
pub mod noise;
pub mod preset;
pub mod request;
pub mod rivers;
pub mod world;
