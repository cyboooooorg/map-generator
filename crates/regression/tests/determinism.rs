//! Determinism across runs: the digests of the shared reference systems
//! are pinned, a regression net rather than a source.

use regression::{digest, reference_systems};

/// The pinned digests of the reference systems, per platform: the libm of
/// each may differ by an ulp, enough to flip a threshold of the draw.
/// `None` until the platform's values are pinned.
#[cfg(target_os = "macos")]
const EXPECTED: Option<[u64; 8]> = Some([
    5015423907104303343,
    16509297106138697393,
    1849007125372145455,
    15989544312016328557,
    6776797817325080942,
    8845664612222415756,
    12078261093261751040,
    11469689764732076138,
]);

/// The Linux table is filled from the CI log of `golden_digests` (its
/// output is shown on success, see `.config/nextest.toml`).
#[cfg(target_os = "linux")]
const EXPECTED: Option<[u64; 8]> = None;

/// No table for other platforms.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const EXPECTED: Option<[u64; 8]> = None;

/// The digests of the reference systems match this platform's table. A
/// change means every drawn system changes: decide it on purpose, then
/// paste the printed values into the table of each platform. On a
/// platform whose table is not filled yet (Linux, until it is pinned from
/// CI), the test prints the digests and returns early with "linux digests
/// not pinned yet".
#[test]
fn golden_digests() {
    let digests: Vec<u64> = reference_systems().iter().map(digest).collect();
    let os = std::env::consts::OS;
    println!("golden digests ({os}): {digests:?}");
    let Some(expected) = EXPECTED else {
        println!("{os} digests not pinned yet: paste the values above into its EXPECTED table");
        return;
    };
    assert_eq!(digests, expected);
}
