//! Determinism across runs: the digests of the shared reference systems
//! are pinned, a regression net rather than a source.

use regression::{digest, reference_requests, reference_systems};
use system::draw_with;
use system::planet::class_of;
use timeline::Trace;

/// The pinned digests of the reference systems, drawn with the planet
/// model's `class_of` (engine 4; before it, the stand-in classifier and
/// spins from the body seed). Pinned from a local macOS run: CI runs on
/// Linux only on pull requests. macOS and Linux gave the same values for
/// the stand-in digests (PR #629); if their libm differs by an ulp and
/// flips a threshold of the draw, split this table per `target_os`.
const EXPECTED: [u64; 8] = [
    7717621872393268112,
    13277582001339794335,
    4485924109708704685,
    15676482809518794994,
    10643121030845601339,
    6233997479138571296,
    7398689851432714755,
    3500269132337718553,
];

/// The digests of the reference systems match the table. A change means
/// every drawn system changes: decide it on purpose, then paste the values
/// the failure prints into the table.
#[test]
fn golden_digests() {
    let digests: Vec<u64> = reference_systems().iter().map(digest).collect();
    assert_eq!(digests, EXPECTED, "golden digests: {digests:?}");
}

/// The draw is a pure function of its inputs: drawing the first reference
/// request again gives the digest of the shared draw.
#[test]
fn the_draw_is_pure_within_a_run() {
    let request = &reference_requests()[0];
    let again = draw_with(request, &mut Trace::new(), class_of).unwrap();
    assert_eq!(digest(&again), digest(&reference_systems()[0]));
}
