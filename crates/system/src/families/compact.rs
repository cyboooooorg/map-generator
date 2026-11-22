//! The compact family: four to seven small planets packed outward from
//! an innermost one (study row 1).

use super::laws::{compact_ratio, innermost_period};
use super::table::COMPACT_PLANETS;
use super::uniform_count;
use super::{
    COMPACT_MAX_PERIOD, COMPACT_OUTER_EDGE, Counts, Family, FamilyContext, Placed, Proposal,
    SlotKind, outermost,
};
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
/// spacing or near resonance, S 1.5). The proposal can lie beyond
/// [`COMPACT_OUTER_EDGE`]: [`ends_chain`] then ends the chain. The mass
/// is the system's: the placement loop scatters it.
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

/// Whether `proposal` ends the chain: a compact family proposal whose
/// period is strictly beyond [`COMPACT_OUTER_EDGE`] (1071 d, Weiss et al.
/// 2018). The slot and every later family slot stay empty, as Weiss's
/// bootstrap removes such planets without redrawing them. Always false
/// for the other families.
pub(crate) fn ends_chain(ctx: &FamilyContext, proposal: &Proposal) -> bool {
    ctx.family == Family::Compact && orbit::period(proposal.a, ctx.host).0 > COMPACT_OUTER_EDGE.0
}
