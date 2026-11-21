//! Determinism across runs: the digests of the shared reference systems
//! are pinned, a regression net rather than a source.

use regression::{digest, reference_systems};

/// The pinned digests of the reference systems. macOS (local) and Linux
/// (CI) gave the same values when pinned (PR #629); if their libm ever
/// differs by an ulp and flips a threshold of the draw, split this table
/// per `target_os`.
const EXPECTED: [u64; 8] = [
    5015423907104303343,
    16509297106138697393,
    1849007125372145455,
    15989544312016328557,
    6776797817325080942,
    8845664612222415756,
    12078261093261751040,
    11469689764732076138,
];

/// The digests of the reference systems match the table. A change means
/// every drawn system changes: decide it on purpose, then paste the values
/// the failure prints into the table.
#[test]
fn golden_digests() {
    let digests: Vec<u64> = reference_systems().iter().map(digest).collect();
    assert_eq!(digests, EXPECTED, "golden digests: {digests:?}");
}
