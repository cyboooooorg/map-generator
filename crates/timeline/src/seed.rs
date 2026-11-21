//! Seeds: the root of every random draw, and its derivation by label.

use serde::{Deserialize, Serialize};

/// A seed of the seed hierarchy. Children are derived by label with
/// [`Seed::derive`]; nothing draws outside this hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seed(u64);

impl Seed {
    /// A seed from its number.
    pub fn new(value: u64) -> Seed {
        Seed(value)
    }

    /// The seed's number.
    pub fn get(self) -> u64 {
        self.0
    }

    /// The child seed named `label`: `splitmix64(parent ^ fnv1a64(label))`.
    /// Stable across versions and platforms.
    pub fn derive(self, label: &str) -> Seed {
        Seed(splitmix64(self.0 ^ fnv1a64(label.as_bytes())))
    }
}

/// FNV-1a, 64 bits.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The splitmix64 output function applied to `state + gamma`.
fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a64_matches_the_published_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn splitmix64_matches_the_reference_generator_from_zero() {
        // First output of Vigna's splitmix64.c seeded with 0.
        assert_eq!(splitmix64(0), 0xe220_a839_7b1d_cdaf);
    }

    #[test]
    fn derive_mixes_the_parent_with_the_label_hash() {
        let parent = Seed::new(42);
        assert_eq!(parent.derive("b").get(), splitmix64(42 ^ fnv1a64(b"b")));
        assert_ne!(parent.derive("").get(), parent.derive("b").get());
        assert_ne!(parent.derive("é").get(), parent.derive("e").get());
        assert_ne!(Seed::new(43).derive("b"), parent.derive("b"));
    }

    #[test]
    fn a_seed_serializes_as_its_number() {
        assert_eq!(serde_json::to_string(&Seed::new(7)).unwrap(), "7");
    }
}
