//! The sparse family: none to two planets, each small or a giant with
//! odds 1/2 (study row 5). Slots: the small planets, then the giants.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the placement loop of a later task")
)]

use super::laws::{ECCENTRIC_GIANT_MASS, jupiter_masses, small_planet_mass, sparse_axis};
use super::table::{SPARSE_GIANT_CHANCE, SPARSE_PLANETS};
use super::{Counts, FamilyContext, Placed, Proposal, SlotKind, uniform_count};
use crate::random::uniform;
use rand_chacha::ChaCha8Rng;

/// [`SPARSE_PLANETS`] planets, uniform, each a giant with
/// [`SPARSE_GIANT_CHANCE`].
pub(crate) fn counts(rng: &mut ChaCha8Rng) -> Counts {
    let planets = uniform_count(rng, SPARSE_PLANETS);
    let giants = (0..planets)
        .map(|_| u32::from(uniform(rng) < SPARSE_GIANT_CHANCE))
        .sum();
    Counts {
        small: planets - giants,
        giants,
    }
}

/// A small slot: mass from Neil & Rogers (B9). A giant slot: mass as the
/// eccentric giants', log-uniform 0.3-13 M_J. Either at a period
/// log-uniform from the stellar surface to 30 au scaled by the ice line.
pub(crate) fn propose(
    ctx: &FamilyContext,
    slot: u32,
    rng: &mut ChaCha8Rng,
    _placed: &[Placed],
) -> Proposal {
    let (mass, kind) = if slot < ctx.counts.small {
        (small_planet_mass(rng), SlotKind::Small)
    } else {
        (jupiter_masses(rng, ECCENTRIC_GIANT_MASS), SlotKind::Giant)
    };
    Proposal {
        mass,
        a: sparse_axis(rng, ctx.stellar_radius, ctx.ice_line),
        kind,
    }
}
