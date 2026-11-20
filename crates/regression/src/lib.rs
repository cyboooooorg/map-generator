//! Tier-2 regression tests (see CLAUDE.md, Tests): shared reference
//! systems generated once per run on a coarse grid, then statistics,
//! contracts and determinism asserted on them.
//!
//! Test-only crate: it may depend on every engine, which a domain crate
//! may not, even as a dev-dependency.

/// Seeds of the shared reference systems; filled in with slice 1, PR 2.
pub const REFERENCE_SEEDS: &[u64] = &[];
