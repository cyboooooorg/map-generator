//! Radiative transfer of a planetary atmosphere: the gases it holds, and
//! the laws the longwave column is built on.

pub mod clouds;
pub mod gases;

#[cfg_attr(not(test), expect(dead_code, reason = "read by the column, Task 8"))]
pub(crate) mod fast_math;
#[expect(
    dead_code,
    reason = "the fast path (`Table::lines`, `spectrum::fast_level`) is read by the column, Task 8"
)]
pub(crate) mod ktable;
#[cfg_attr(not(test), expect(dead_code, reason = "read by the column, Task 8"))]
pub(crate) mod methane;
#[expect(
    dead_code,
    reason = "the fast path (`fast_level`, `smooth_above`) is read by the column, Task 8"
)]
pub(crate) mod spectrum;
