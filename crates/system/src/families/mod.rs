//! The families of planetary systems the draw picks among.
//!
//! The family is engine-internal data, not traced: the trace has no node
//! type for an engine enum. [`Drawn`](crate::Drawn) carries it.

use crate::random::uniform;
use rand_chacha::ChaCha8Rng;
use science::constants::{ASTRONOMICAL_UNIT, SOLAR_RADIUS};
use science::orbit::{self, ICE_LINE_OF_THE_SUN};
use science::units::{AstronomicalUnit, Day};
use std::fmt;
use timeline::StarState;

pub(crate) mod table;
#[cfg(test)]
mod tests;

/// Source: EPOS I, a compact system reaches 640 d at most (study row 1,
/// outer extent 35-640 d).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the family draw of a later task")
)]
const COMPACT_MAX_PERIOD: Day = Day(640.0);

/// Source: Dawson & Johnson 2018, hot Jupiters up to 10 d (study G 1.1).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the family draw of a later task")
)]
const HOT_JUPITER_MAX_PERIOD: Day = Day(10.0);

/// Source: Fulton 2021, the broken power law's support reaches 30 AU for
/// the Sun (study G 3.1); scaled by the ice line (a Definition).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the family draw of a later task")
)]
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
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the family draw of a later task")
)]
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
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "used by the family draw of a later task")
)]
pub(crate) fn possible(family: Family, star: &StarState) -> bool {
    let mass = *star.mass.value();
    let radius = star.radius.value().0 * SOLAR_RADIUS.0 / ASTRONOMICAL_UNIT.0;
    let surface = orbit::period(AstronomicalUnit(radius), mass);
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
