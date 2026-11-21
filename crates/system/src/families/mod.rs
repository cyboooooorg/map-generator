//! The families of planetary systems the draw picks among.
//!
//! The family is engine-internal data, not traced: the trace has no node
//! type for an engine enum. [`Drawn`](crate::Drawn) carries it.

use crate::random::uniform;
use crate::{ArchetypeId, Trial, TrialOutcome};
use rand_chacha::ChaCha8Rng;
use science::constants::{ASTRONOMICAL_UNIT, SOLAR_RADIUS};
use science::orbit::{self, ICE_LINE_OF_THE_SUN};
use science::units::{AstronomicalUnit, Day, EarthMass, SolarMass};
use std::fmt;
use timeline::StarState;

mod compact;
mod eccentric;
mod hot_jupiter;
pub(crate) mod laws;
mod solar_like;
mod sparse;
pub(crate) mod table;
#[cfg(test)]
pub(crate) mod tests;

/// Definition: the longest period of a compact system's innermost
/// planet (and of the family's support), 640 d, the top of EPOS I's
/// outer extent of 35-640 d (study row 1). The study gives that range as
/// an outcome of the fit, not as a cap: the draw uses it as one.
const COMPACT_MAX_PERIOD: Day = Day(640.0);

/// Source: Dawson & Johnson 2018, hot Jupiters up to 10 d (study G 1.1).
const HOT_JUPITER_MAX_PERIOD: Day = Day(10.0);

/// Source: Fulton 2021, the broken power law's support reaches 30 AU for
/// the Sun (study G 3.1); scaled by the ice line (a Definition).
const FULTON_OUTER_SUPPORT: AstronomicalUnit = AstronomicalUnit(30.0);

/// The architecture of a drawn system. Drawn evenly (a user decision of
/// 2026-11-21) on `seed.derive("family")`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    /// Tightly packed small planets.
    Compact,
    /// Rocky planets inside the ice line, giants beyond it.
    SolarLike,
    /// A hot Jupiter and little else.
    HotJupiter,
    /// Giants on eccentric orbits.
    EccentricGiants,
    /// Few planets, possibly none.
    Sparse,
}

impl Family {
    /// The family's code: `compact`, `solar-like`, `hot-jupiter`,
    /// `eccentric-giants`, `sparse`.
    pub fn code(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::SolarLike => "solar-like",
            Self::HotJupiter => "hot-jupiter",
            Self::EccentricGiants => "eccentric-giants",
            Self::Sparse => "sparse",
        }
    }
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Picks a family evenly among those `possible` accepts. `Sparse` must be
/// possible (it may hold no planet), so the loop ends.
pub(crate) fn pick_family(rng: &mut ChaCha8Rng, possible: impl Fn(Family) -> bool) -> Family {
    const ALL: [Family; 5] = [
        Family::Compact,
        Family::SolarLike,
        Family::HotJupiter,
        Family::EccentricGiants,
        Family::Sparse,
    ];
    let open: Vec<Family> = ALL.into_iter().filter(|f| possible(*f)).collect();
    if open.is_empty() {
        return Family::Sparse;
    }
    open[((uniform(rng) * open.len() as f64) as usize).min(open.len() - 1)]
}

/// False only when the whole period support of `family` lies inside the
/// star: its longest period is shorter than that of an orbit at the
/// stellar surface. True for every host of 0.08 to 8 M☉; a test pins it.
pub(crate) fn possible(family: Family, star: &StarState) -> bool {
    let mass = *star.mass.value();
    let surface = orbit::period(stellar_radius(star), mass);
    let longest = match family {
        Family::Compact => COMPACT_MAX_PERIOD,
        Family::HotJupiter => HOT_JUPITER_MAX_PERIOD,
        Family::SolarLike | Family::EccentricGiants | Family::Sparse => orbit::period(
            FULTON_OUTER_SUPPORT * (orbit::ice_line(mass).0 / ICE_LINE_OF_THE_SUN.0),
            mass,
        ),
    };
    longest.0 > surface.0
}

/// The star's radius in astronomical units.
fn stellar_radius(star: &StarState) -> AstronomicalUnit {
    AstronomicalUnit(star.radius.value().0 * SOLAR_RADIUS.0 / ASTRONOMICAL_UNIT.0)
}

/// How many family slots a system has: `small` small-planet slots and
/// `giants` giant slots. Each family orders them (its module says how).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Counts {
    pub small: u32,
    pub giants: u32,
}

/// What a family slot's proposal is: a small planet or a giant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotKind {
    Small,
    Giant,
}

/// A family slot's proposed mass and semi-major axis, before classifying.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Proposal {
    pub mass: EarthMass,
    pub a: AstronomicalUnit,
    pub kind: SlotKind,
}

/// A trial the placement loop kept, with its classified outcome, its
/// label (for its orbit stream) and the archetype asked for it, if any.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    pub trial: Trial,
    pub outcome: TrialOutcome,
    pub label: String,
    pub asked: Option<ArchetypeId>,
}

impl Placed {
    fn mass(&self) -> EarthMass {
        *self.trial.mass.value()
    }

    fn axis(&self) -> AstronomicalUnit {
        *self.trial.orbit.semi_major_axis.value()
    }

    fn periapsis(&self) -> AstronomicalUnit {
        self.axis() * (1.0 - self.trial.orbit.eccentricity.value().get())
    }

    /// The kind of a drawn planet, by its mass against the giants' floor
    /// (every proposal of a kind keeps to its side); `None` when asked.
    fn drawn_kind(&self) -> Option<SlotKind> {
        if self.asked.is_some() {
            None
        } else if self.mass().0 < laws::SMALL_MASS_MAX.0 {
            Some(SlotKind::Small)
        } else {
            Some(SlotKind::Giant)
        }
    }
}

/// The drawn planet of `kind` furthest out, the anchor of a chain.
fn outermost(placed: &[Placed], kind: SlotKind) -> Option<&Placed> {
    placed
        .iter()
        .filter(|p| p.drawn_kind() == Some(kind))
        .max_by(|a, b| a.axis().0.total_cmp(&b.axis().0))
}

/// The drawn planet of `kind` furthest in.
fn innermost(placed: &[Placed], kind: SlotKind) -> Option<&Placed> {
    placed
        .iter()
        .filter(|p| p.drawn_kind() == Some(kind))
        .min_by(|a, b| a.axis().0.total_cmp(&b.axis().0))
}

/// A count uniform in `(low, high)`, both included.
fn uniform_count(rng: &mut ChaCha8Rng, (low, high): (u32, u32)) -> u32 {
    let span = f64::from(high - low + 1);
    low + ((uniform(rng) * span) as u32).min(high - low)
}

/// The family's slot counts, drawn on the `family` stream.
pub(crate) fn counts(family: Family, rng: &mut ChaCha8Rng) -> Counts {
    match family {
        Family::Compact => compact::counts(rng),
        Family::SolarLike => solar_like::counts(rng),
        Family::HotJupiter => hot_jupiter::counts(rng),
        Family::EccentricGiants => eccentric::counts(rng),
        Family::Sparse => sparse::counts(rng),
    }
}

/// The proposal of family slot `slot` (0-based, family slots only), on
/// that slot's stream, given every planet `placed` so far (asked ones
/// are never a chain's anchor).
pub(crate) fn propose(
    ctx: &FamilyContext,
    slot: u32,
    rng: &mut ChaCha8Rng,
    placed: &[Placed],
) -> Proposal {
    match ctx.family {
        Family::Compact => compact::propose(ctx, slot, rng, placed),
        Family::SolarLike => solar_like::propose(ctx, slot, rng, placed),
        Family::HotJupiter => hot_jupiter::propose(ctx, slot, rng, placed),
        Family::EccentricGiants => eccentric::propose(ctx, slot, rng, placed),
        Family::Sparse => sparse::propose(ctx, slot, rng, placed),
    }
}

/// The family, its host and the system-level draws every slot reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FamilyContext {
    pub family: Family,
    pub host: SolarMass,
    pub ice_line: AstronomicalUnit,
    pub stellar_radius: AstronomicalUnit,
    pub counts: Counts,
    /// The compact system's mass (Neil & Rogers, B9), drawn for every
    /// family.
    pub system_mass: EarthMass,
    /// The compact system's Rayleigh σ_e, uniform in
    /// [`laws::COMPACT_SIGMA_E`], drawn for every family.
    pub sigma_e: f64,
    /// A solar-like system whose giants take the upper Beta half.
    pub stirred: bool,
    /// A hot Jupiter with its outer companion.
    pub companion: bool,
}

impl FamilyContext {
    /// Draws, on the `family` stream after the family pick: the counts,
    /// then the system mass, σ_e and the stirred draw, in that order for
    /// every family.
    pub(crate) fn draw(family: Family, star: &StarState, rng: &mut ChaCha8Rng) -> Self {
        let counts = counts(family, rng);
        let system_mass = laws::small_planet_mass(rng);
        let (lo, hi) = laws::COMPACT_SIGMA_E;
        let sigma_e = lo + (hi - lo) * uniform(rng);
        let stirred = uniform(rng) < table::SOLAR_LIKE_STIRRED_CHANCE;
        let host = *star.mass.value();
        Self {
            family,
            host,
            ice_line: orbit::ice_line(host),
            stellar_radius: stellar_radius(star),
            counts,
            system_mass,
            sigma_e,
            stirred: family == Family::SolarLike && stirred,
            companion: family == Family::HotJupiter && counts.giants == 2,
        }
    }

    /// The period of an orbit grazing the stellar surface.
    fn surface_period(&self) -> Day {
        orbit::period(self.stellar_radius, self.host)
    }
}

#[cfg(test)]
mod tests_codes {
    use super::*;

    #[test]
    fn codes_are_the_published_ones() {
        let codes: Vec<_> = [
            Family::Compact,
            Family::SolarLike,
            Family::HotJupiter,
            Family::EccentricGiants,
            Family::Sparse,
        ]
        .iter()
        .map(|family| family.to_string())
        .collect();
        assert_eq!(
            codes,
            [
                "compact",
                "solar-like",
                "hot-jupiter",
                "eccentric-giants",
                "sparse"
            ]
        );
    }
}
