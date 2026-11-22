//! Radiative transfer of a planetary atmosphere: the gases it holds, and
//! the laws the longwave column is built on.

pub mod gases;

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "read by the spectrum, next task")
)]
pub(crate) mod fast_math;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "read by the spectrum, next task")
)]
pub(crate) mod ktable;
