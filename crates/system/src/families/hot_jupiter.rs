//! The hot-Jupiter family: one hot Jupiter and, in seven systems of ten,
//! a distant giant companion (study row 3). Slot 0 is the hot Jupiter,
//! slot 1 the companion.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the placement loop of a later task")
)]

use super::laws::{
    COMPANION_MASS, HOT_JUPITER_MASS, companion_axis, hot_jupiter_axis, jupiter_masses,
};
use super::table::HOT_JUPITER_COMPANION_CHANCE;
use super::{Counts, FamilyContext, Placed, Proposal, SlotKind};
use crate::random::uniform;
use rand_chacha::ChaCha8Rng;

/// One giant, two with [`HOT_JUPITER_COMPANION_CHANCE`].
pub(crate) fn counts(rng: &mut ChaCha8Rng) -> Counts {
    Counts {
        small: 0,
        giants: 1 + u32::from(uniform(rng) < HOT_JUPITER_COMPANION_CHANCE),
    }
}

/// Slot 0: mass log-uniform 0.25-13 M_J, period log-uniform from the
/// stellar surface to 10 d (G 1.1). Slot 1: mass log-uniform 1-13 M_J,
/// axis log-uniform 1-20 au scaled by the ice line (G 1.5).
pub(crate) fn propose(
    ctx: &FamilyContext,
    slot: u32,
    rng: &mut ChaCha8Rng,
    _placed: &[Placed],
) -> Proposal {
    let (mass, a) = if slot == 0 {
        let mass = jupiter_masses(rng, HOT_JUPITER_MASS);
        (mass, hot_jupiter_axis(rng, ctx.host, ctx.stellar_radius))
    } else {
        let mass = jupiter_masses(rng, COMPANION_MASS);
        (mass, companion_axis(rng, ctx.ice_line))
    };
    Proposal {
        mass,
        a,
        kind: SlotKind::Giant,
    }
}
