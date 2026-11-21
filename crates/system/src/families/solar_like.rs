//! The solar-like family: rocky planets inside the ice line, giants
//! beyond it (study row 2). Slots: the rocky planets first, inside out,
//! then the giants.

use super::laws::{
    SOLAR_LIKE_GIANT_MASS, fulton_axis, innermost_period, next_giant_axis, next_rocky_axis,
    rocky_mass,
};
use super::table::{SOLAR_LIKE_GIANTS, SOLAR_LIKE_ROCKY};
use super::{Counts, FamilyContext, Placed, Proposal, SlotKind, outermost, uniform_count};
use crate::random::log_uniform;
use rand_chacha::ChaCha8Rng;
use science::orbit;
use science::units::EarthMass;

/// [`SOLAR_LIKE_ROCKY`] rocky planets, then [`SOLAR_LIKE_GIANTS`] giants,
/// each uniform.
pub(crate) fn counts(rng: &mut ChaCha8Rng) -> Counts {
    let small = uniform_count(rng, SOLAR_LIKE_ROCKY);
    let giants = uniform_count(rng, SOLAR_LIKE_GIANTS);
    Counts { small, giants }
}

/// A rocky slot: mass log-uniform 0.4-2.6 M⊕ (Raymond 2006, S 5); the
/// first at a period from EPOS I's innermost law inside the ice line,
/// each next one beyond the outermost drawn rocky planet by Fang &
/// Margot's spacing, kept inside the ice line. A giant slot: mass
/// log-uniform 30-900 M⊕ (Fulton 2021, G 3.2); the first on Fulton's
/// law beyond the ice line, each next one beyond the outermost drawn
/// giant by Rosenthal's ratio (G 3.3).
pub(crate) fn propose(
    ctx: &FamilyContext,
    slot: u32,
    rng: &mut ChaCha8Rng,
    placed: &[Placed],
) -> Proposal {
    if slot < ctx.counts.small {
        let mass = rocky_mass(rng);
        let a = match outermost(placed, SlotKind::Small) {
            None => {
                let edge = orbit::period(ctx.ice_line, ctx.host);
                let period = innermost_period(rng, ctx.surface_period(), edge);
                orbit::semi_major_axis(period, ctx.host)
            }
            Some(inner) => next_rocky_axis(
                rng,
                (inner.mass(), inner.axis()),
                mass,
                ctx.host,
                ctx.ice_line,
            ),
        };
        Proposal {
            mass,
            a,
            kind: SlotKind::Small,
        }
    } else {
        let (lo, hi) = SOLAR_LIKE_GIANT_MASS;
        let mass = EarthMass(log_uniform(rng, lo.0, hi.0));
        let a = match outermost(placed, SlotKind::Giant) {
            None => fulton_axis(rng, ctx.ice_line),
            Some(inner) => next_giant_axis(rng, inner.axis()),
        };
        Proposal {
            mass,
            a,
            kind: SlotKind::Giant,
        }
    }
}
