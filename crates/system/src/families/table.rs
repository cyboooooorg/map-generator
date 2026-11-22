//! The count ranges and chances of the families, in one table.
//!
//! Every range is `(low, high)`, both included, drawn uniformly. A
//! uniform draw flattens the source's shape (the `Definition:` of each
//! range); the range itself is the study's row. Raising or lowering a
//! range touches this file only. Study:
//! `docs/superpowers/studies/2026-11-21-architecture-families`.

/// Definition: planets of a compact system, uniform. Study row 1; source
/// He 2019 (3.86 planets per cluster) and EPOS I (7 and more), the
/// flattening is ours.
pub(crate) const COMPACT_PLANETS: (u32, u32) = (4, 7);

/// Definition: inner rocky planets of a solar-like system, uniform. Study
/// row 2; Raymond 2006 simulations, the flattening is ours.
pub(crate) const SOLAR_LIKE_ROCKY: (u32, u32) = (2, 4);

/// Definition: giants of a solar-like system, uniform. Study row 2; the
/// Solar System has four, no source counts them.
pub(crate) const SOLAR_LIKE_GIANTS: (u32, u32) = (1, 4);

/// Definition: giants of an eccentric-giants system, uniform. Study row
/// 4, from the spec.
pub(crate) const ECCENTRIC_GIANTS: (u32, u32) = (1, 2);

/// Definition: small planets inside an eccentric giant's orbit, uniform.
/// Study row 4; Zhu & Wu 2018 find fewer of them (1.4 against 2.5), the
/// range is ours.
pub(crate) const ECCENTRIC_INNER_SMALL: (u32, u32) = (0, 2);

/// Definition: planets of a sparse system, uniform, none allowed. Study
/// row 5, from the spec.
pub(crate) const SPARSE_PLANETS: (u32, u32) = (0, 2);

/// Source: Dawson & Johnson 2018 p32, combining Knutson 2014 and Bryan
/// 2016: 70 % of hot Jupiters have an outer companion (study G 1.5).
pub(crate) const HOT_JUPITER_COMPANION_CHANCE: f64 = 0.70;

/// Definition: chance that a solar-like system draws its giants'
/// eccentricities from the upper half of the law, per system (user
/// decision of 2026-11-21: one system in ten).
pub(crate) const SOLAR_LIKE_STIRRED_CHANCE: f64 = 0.1;

/// Definition: chance that a planet of a sparse system is a giant, the
/// other being small (study row 5).
pub(crate) const SPARSE_GIANT_CHANCE: f64 = 0.5;

/// Definition: tries a family slot has to find a valid orbit before it
/// stays empty.
pub(crate) const SLOT_TRIES: u32 = 20;
