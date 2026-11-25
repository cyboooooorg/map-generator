//! Determinism across runs: the digests of the shared reference systems
//! are pinned, a regression net rather than a source.

use regression::{digest, reference_systems};

/// The pinned digests of the reference systems. macOS (local) and Linux
/// (CI) gave the same values when pinned (PR #629); if their libm ever
/// differs by an ulp and flips a threshold of the draw, split this table
/// per `target_os`.
const EXPECTED: [u64; 8] = [
    2686437292942865273,
    5220593974498918302,
    10634933927778523099,
    18330059145874931970,
    13023959179005833053,
    12877296337732375238,
    13321507188459537393,
    565560109508613837,
];

/// The digests of the reference systems match the table. A change means
/// every drawn system changes: decide it on purpose, then paste the values
/// the failure prints into the table.
#[test]
fn golden_digests() {
    let digests: Vec<u64> = reference_systems().iter().map(digest).collect();
    assert_eq!(digests, EXPECTED, "golden digests: {digests:?}");
}
