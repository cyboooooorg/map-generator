//! The kinds of object a planet can orbit, and the notation that names
//! them.
//!
//! Only main-sequence dwarfs are modelled today ([`HostKind::Dwarf`]); the
//! other kinds are recognised by the parser so that a request for one is
//! refused with the issue that tracks it, rather than as a typo.

use crate::star::{CLASSES, TYPE_RANGE};
use serde::{Deserialize, Serialize};

/// What a planet's host is, by the physics that sets its light and its
/// surroundings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    /// A main-sequence star (luminosity class V), the only kind modelled.
    Dwarf,
    /// A star leaving the main sequence (luminosity class IV).
    Subgiant,
    /// A giant or supergiant (luminosity classes I to III).
    Giant,
    /// The degenerate core of a dead star of low or medium mass (D).
    WhiteDwarf,
    /// A pulsar or other neutron star (PSR).
    NeutronStar,
    /// A neutron star with an extreme magnetic field (MAG).
    Magnetar,
    /// A substellar object, spectral classes L, T and Y.
    BrownDwarf,
    /// A stellar-mass black hole (BH).
    BlackHole,
    /// A massive star stripped of its hydrogen (WR, WN, WC, WO).
    WolfRayet,
    /// A hypothetical star powered by a black hole inside it (QS).
    QuasiStar,
}

impl HostKind {
    /// Every kind, in the order of the enum.
    pub const ALL: [HostKind; 10] = [
        HostKind::Dwarf,
        HostKind::Subgiant,
        HostKind::Giant,
        HostKind::WhiteDwarf,
        HostKind::NeutronStar,
        HostKind::Magnetar,
        HostKind::BrownDwarf,
        HostKind::BlackHole,
        HostKind::WolfRayet,
        HostKind::QuasiStar,
    ];

    /// Whether the generator models this kind of host.
    pub fn available(self) -> bool {
        self == HostKind::Dwarf
    }

    /// The backlog issue that tracks modelling this kind, `None` for the
    /// kind already modelled.
    pub fn issue(self) -> Option<u32> {
        match self {
            HostKind::Dwarf => None,
            HostKind::Subgiant | HostKind::Giant => Some(576),
            HostKind::WhiteDwarf => Some(577),
            HostKind::NeutronStar => Some(578),
            HostKind::Magnetar => Some(579),
            HostKind::BrownDwarf => Some(580),
            HostKind::BlackHole => Some(581),
            HostKind::WolfRayet => Some(582),
            HostKind::QuasiStar => Some(583),
        }
    }

    /// The plural noun of the kind, for messages ("white dwarfs are not
    /// modelled yet").
    pub fn name(self) -> &'static str {
        match self {
            HostKind::Dwarf => "dwarfs",
            HostKind::Subgiant => "subgiants",
            HostKind::Giant => "giants",
            HostKind::WhiteDwarf => "white dwarfs",
            HostKind::NeutronStar => "neutron stars",
            HostKind::Magnetar => "magnetars",
            HostKind::BrownDwarf => "brown dwarfs",
            HostKind::BlackHole => "black holes",
            HostKind::WolfRayet => "Wolf-Rayet stars",
            HostKind::QuasiStar => "quasi-stars",
        }
    }

    /// Whether the photosphere has a convective envelope, so that it can
    /// carry a dynamo, spots and granulation. A degenerate or hydrogen-free
    /// surface has none.
    pub fn has_convective_photosphere(self) -> bool {
        matches!(
            self,
            HostKind::Dwarf | HostKind::Subgiant | HostKind::Giant | HostKind::BrownDwarf
        )
    }
}

/// A parsed spectral type: the kind of host and its position along that
/// kind's own sequence (for dwarfs, `class × 10 + subtype`, O = 0 … M = 6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostType {
    pub kind: HostKind,
    pub code: f32,
}

const FIRST_ERROR: &str = "starts with one of O B A F G K M";
const SUBTYPE_ERROR: &str = "the subtype is a number from 0 to 9.5";

/// Reads a spectral type of any kind of host (see the grammar in the
/// design of host kinds). Grammar only: whether the kind is modelled is for
/// the caller to check with [`HostKind::available`]. Case-insensitive.
pub fn parse_host(text: &str) -> Result<HostType, String> {
    let text = text.trim();
    let fail = |why: &str| format!("spectral type {text:?}: {why}");
    let upper = text.to_ascii_uppercase();
    let host = |kind, code| Ok(HostType { kind, code });

    match upper.as_str() {
        "PSR" => return host(HostKind::NeutronStar, 0.0),
        "MAG" => return host(HostKind::Magnetar, 0.0),
        "BH" => return host(HostKind::BlackHole, 0.0),
        "WR" => return host(HostKind::WolfRayet, 0.0),
        "QS" => return host(HostKind::QuasiStar, 0.0),
        _ => {}
    }
    if let Some(n) = ["WN", "WC", "WO"]
        .iter()
        .find_map(|p| upper.strip_prefix(p))
    {
        let n = n
            .parse::<f32>()
            .ok()
            .filter(|n| (0.0..10.0).contains(n))
            .ok_or_else(|| fail("a Wolf-Rayet subtype is a number from 0 to 9.5"))?;
        return host(HostKind::WolfRayet, n);
    }
    if let Some(rest) = upper.strip_prefix('D') {
        let mut chars = rest.chars();
        if !chars.next().is_some_and(|c| "ABOQZC".contains(c)) {
            return Err(fail("a white dwarf is D followed by one of A B O Q Z C"));
        }
        let n = chars.as_str();
        let n = if n.is_empty() {
            0.0
        } else {
            n.parse::<f32>()
                .ok()
                .filter(|n| *n >= 0.0)
                .ok_or_else(|| fail("a white dwarf's temperature index is a number"))?
        };
        return host(HostKind::WhiteDwarf, n);
    }

    let mut chars = upper.chars();
    let letter = chars.next();
    let rest = chars.as_str();
    // The subtype is the leading number; what follows is the luminosity
    // class.
    let number_len = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    let (number, suffix) = rest.split_at(number_len);
    let subtype = |default: f32| -> Result<f32, String> {
        if number.is_empty() {
            Ok(default)
        } else {
            number
                .parse::<f32>()
                .ok()
                .filter(|s| (0.0..10.0).contains(s))
                .ok_or_else(|| fail(SUBTYPE_ERROR))
        }
    };

    if let Some(base) = letter.and_then(|c| ["LTY"].iter().find_map(|s| s.find(c))) {
        if !suffix.is_empty() {
            return Err(fail("a brown dwarf is L, T or Y and a subtype"));
        }
        return host(
            HostKind::BrownDwarf,
            70.0 + 10.0 * base as f32 + subtype(5.0)?,
        );
    }

    let class = letter
        .and_then(|c| CLASSES.iter().position(|&k| k == c))
        .ok_or_else(|| fail(FIRST_ERROR))?;
    let kind = match suffix {
        "" | "V" => HostKind::Dwarf,
        "IV" => HostKind::Subgiant,
        "I" | "IA" | "IB" | "IAB" | "II" | "III" => HostKind::Giant,
        _ => {
            return Err(fail(
                "the luminosity class is V, IV, III, II, I, Ia, Ib or Iab",
            ));
        }
    };
    let code = class as f32 * 10.0 + subtype(5.0)?;
    if kind == HostKind::Dwarf && !(TYPE_RANGE.0..=TYPE_RANGE.1).contains(&code) {
        return Err(fail("main-sequence dwarfs go from O3 to M9.5"));
    }
    host(kind, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dwarf_notations_keep_their_codes() {
        for (t, c) in [
            ("G2V", 42.0),
            ("g2", 42.0),
            ("m4.5", 64.5),
            ("K", 55.0),
            ("M", 65.0),
            ("B", 15.0),
        ] {
            assert_eq!(
                parse_host(t).unwrap(),
                HostType {
                    kind: HostKind::Dwarf,
                    code: c
                },
                "{t}"
            );
        }
    }

    #[test]
    fn other_kinds_are_recognised() {
        use HostKind::*;
        for (t, k) in [
            ("G8IV", Subgiant),
            ("K3III", Giant),
            ("M2Iab", Giant),
            ("B1Ia", Giant),
            ("DA", WhiteDwarf),
            ("DA2.4", WhiteDwarf),
            ("DZ", WhiteDwarf),
            ("L5", BrownDwarf),
            ("T7", BrownDwarf),
            ("Y0", BrownDwarf),
            ("PSR", NeutronStar),
            ("MAG", Magnetar),
            ("BH", BlackHole),
            ("WR", WolfRayet),
            ("WN7", WolfRayet),
            ("QS", QuasiStar),
        ] {
            assert_eq!(parse_host(t).unwrap().kind, k, "{t}");
        }
    }

    #[test]
    fn malformed_types_are_refused() {
        for t in ["X2", "G12", "O1", "D", "L12", "G2VI", ""] {
            assert!(parse_host(t).is_err(), "{t}");
        }
    }

    #[test]
    fn only_dwarfs_are_available() {
        for k in HostKind::ALL {
            assert_eq!(k.available(), k == HostKind::Dwarf);
            assert_eq!(k.issue().is_none(), k == HostKind::Dwarf);
        }
    }
}
