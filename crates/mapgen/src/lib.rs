//! Procedural planet generation.
//!
//! [`request::WorldRequest`] describes what to generate; resolving it gives
//! complete [`world::WorldParams`]. [`generation::generate`] runs every pass
//! (noise, erosion, climate, rivers, biomes) and [`export::export_world`]
//! writes the maps and raw data. The diagnostic maps are written during the
//! generation, by an [`export::NoiseMapWriter`] passed as its debug sink.
pub mod activity;
pub mod atmosphere;
pub mod belts;
pub mod biome;
pub mod carve;
pub mod climate;
pub mod clouds;
pub mod coarse;
pub mod craters;
pub mod cube;
pub mod delta;
pub mod detail;
pub mod drift;
pub mod epoch;
pub mod erosion;
pub mod exomoon;
pub mod export;
pub mod generation;
pub mod geo;
pub mod geology;
pub mod giant_clouds;
pub mod giant_world;
pub mod giants;
pub mod hex;
pub mod hydrology;
pub mod moons;
pub mod naming;
pub mod noise;
pub mod plates;
pub mod preset;
pub mod request;
pub mod rings;
pub mod rivers;
pub mod sizes;
pub mod spin;
pub mod star;
pub mod system;
pub mod world;
