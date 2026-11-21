//! The uniform number every draw of the engine builds on.

use rand_chacha::ChaCha8Rng;
use rand_core::Rng;

/// Definition: bits of the mantissa of an `f64`, so that 53 random bits
/// make a uniform number of the grid `k / 2^53`.
const MANTISSA_BITS: u32 = 53;

/// A uniform number in the open interval (0, 1).
pub(crate) fn uniform(rng: &mut ChaCha8Rng) -> f64 {
    let grid = 1.0 / (1u64 << MANTISSA_BITS) as f64;
    ((rng.next_u64() >> (u64::BITS - MANTISSA_BITS)) as f64 + 0.5) * grid
}
