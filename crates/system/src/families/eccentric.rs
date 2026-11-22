//! The eccentric-giants family: one or two giants on eccentric orbits
//! and up to two small planets inside them (study row 4). Slots: the
//! giants first, so that an inner planet knows the periapsis it must
//! stay inside, then the small planets.

use super::laws::{
    ECCENTRIC_GIANT_MASS, fulton_axis, innermost_period, jupiter_masses, next_giant_axis,
    small_planet_mass,
};
use super::table::{ECCENTRIC_GIANTS, ECCENTRIC_INNER_SMALL};
use super::{
    Counts, FamilyContext, Placed, Proposal, SlotKind, innermost, outermost, uniform_count,
};
use rand_chacha::ChaCha8Rng;
use science::orbit;

/// [`ECCENTRIC_GIANTS`] giants, then [`ECCENTRIC_INNER_SMALL`] small
/// planets, each uniform.
pub(crate) fn counts(rng: &mut ChaCha8Rng) -> Counts {
    let giants = uniform_count(rng, ECCENTRIC_GIANTS);
    let small = uniform_count(rng, ECCENTRIC_INNER_SMALL);
    Counts { small, giants }
}

/// A giant slot: mass log-uniform 0.3-13 M_J (G 2.4); the first on
/// Fulton's law beyond the ice line (study row 4), the second beyond it
/// by Rosenthal's ratio (G 3.3). A small slot: mass from Neil & Rogers
/// (B9), period from EPOS I's innermost law, from the stellar surface to
/// the periapsis of the innermost drawn giant (the ice line when none was
/// placed). The eccentric Δ floor against the giant (G 2.4) needs the
/// small planet's own eccentricity, drawn after classifying: the
/// placement loop checks it with the spacing of every pair.
pub(crate) fn propose(
    ctx: &FamilyContext,
    slot: u32,
    rng: &mut ChaCha8Rng,
    placed: &[Placed],
) -> Proposal {
    if slot < ctx.counts.giants {
        let mass = jupiter_masses(rng, ECCENTRIC_GIANT_MASS);
        let a = match outermost(placed, SlotKind::Giant) {
            None => fulton_axis(rng, ctx.ice_line),
            Some(inner) => next_giant_axis(rng, inner.axis()),
        };
        Proposal {
            mass,
            a,
            kind: SlotKind::Giant,
        }
    } else {
        let mass = small_planet_mass(rng);
        let edge = innermost(placed, SlotKind::Giant).map_or(ctx.ice_line, Placed::periapsis);
        let period = innermost_period(rng, ctx.surface_period(), orbit::period(edge, ctx.host));
        Proposal {
            mass,
            a: orbit::semi_major_axis(period, ctx.host),
            kind: SlotKind::Small,
        }
    }
}
