//! Radiative transfer of a planetary atmosphere: the gases it holds, and
//! the laws the longwave column is built on.

pub mod clouds;
pub mod column;
pub(crate) mod fast_math;
pub mod gases;
pub(crate) mod ktable;
pub mod methane;
pub(crate) mod shortwave;
pub(crate) mod spectrum;
pub(crate) mod starlight;
#[cfg(test)]
mod test_points;
pub mod veils;
