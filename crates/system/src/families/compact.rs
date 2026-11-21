//! The compact family: four to seven small planets packed outward from
//! an innermost one (study row 1).

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the placement loop of a later task")
)]

use super::laws::{compact_ratio, innermost_period};
use super::table::COMPACT_PLANETS;
use super::uniform_count;
use super::{COMPACT_MAX_PERIOD, Counts, FamilyContext, Placed, Proposal, SlotKind, outermost};
use rand_chacha::ChaCha8Rng;
use science::orbit;

/// Every slot is small: [`COMPACT_PLANETS`], uniform.
pub(crate) fn counts(rng: &mut ChaCha8Rng) -> Counts {
    Counts {
        small: uniform_count(rng, COMPACT_PLANETS),
        giants: 0,
    }
}

/// The first planet (no drawn one placed yet) at a period from EPOS I's
/// innermost law, from the stellar surface to 640 d; each next one
/// beyond the outermost drawn planet by a compact period ratio (EPOS I
/// spacing or near resonance, S 1.5). The mass is the system's: the
/// placement loop scatters it.
pub(crate) fn propose(
    ctx: &FamilyContext,
    _slot: u32,
    rng: &mut ChaCha8Rng,
    placed: &[Placed],
) -> Proposal {
    let period = match outermost(placed, SlotKind::Small) {
        None => innermost_period(rng, ctx.surface_period(), COMPACT_MAX_PERIOD),
        Some(anchor) => orbit::period(anchor.axis(), ctx.host) * compact_ratio(rng).0,
    };
    Proposal {
        mass: ctx.system_mass,
        a: orbit::semi_major_axis(period, ctx.host),
        kind: SlotKind::Small,
    }
}
