//! The families of planetary systems the draw picks among.
//!
//! The family is engine-internal data, not traced: the trace has no node
//! type for an engine enum. [`Drawn`](crate::Drawn) carries it.

use std::fmt;

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

#[cfg(test)]
mod tests {
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
