//! Catalogue designations: `MGS 12345 A` for a star, `MGS 12345 b` for
//! its closest planet.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ContractError;

const CATALOGUE: &str = "MGS";
/// Letters available for a planet: `b` to `z`.
const LETTERS: u32 = 25;
/// Planets that fit in one or two letters.
const MAX_RANKS: u32 = LETTERS + LETTERS * LETTERS;

/// The survey number of a system, 10 000 to 99 999 (the legacy `MGS`
/// range).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemNumber(u32);

impl SystemNumber {
    /// The number, or `InvalidDesignation` outside 10 000 to 99 999.
    pub fn new(number: u32) -> Result<SystemNumber, ContractError> {
        if (10_000..=99_999).contains(&number) {
            Ok(SystemNumber(number))
        } else {
            Err(ContractError::InvalidDesignation(number.to_string()))
        }
    }

    /// The number.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Which body of a system: the star, or the planet at `rank` from it.
/// The derived order puts the star before the planets, then by rank.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum BodyPath {
    /// The central star.
    Star,
    /// The planet at `rank` (0 is the closest), 0 to 649.
    Planet { rank: u32 },
}

/// A body's catalogue designation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Designation {
    /// The system.
    pub system: SystemNumber,
    /// The body in it.
    pub body: BodyPath,
}

fn letter(k: u32) -> char {
    char::from(b'b' + k as u8)
}

/// The letters of the planet at `rank` from the star (0 is the closest):
/// `b`, `c`, ... `z`, then `bb`, `bc`... Ported from the legacy
/// `planet_letter`; valid for ranks below 650.
pub fn planet_letters(rank: u32) -> String {
    if rank < LETTERS {
        letter(rank).to_string()
    } else {
        format!("{}{}", letter(rank / LETTERS - 1), letter(rank % LETTERS))
    }
}

fn rank_of(letters: &str) -> Option<u32> {
    let index = |c: char| {
        c.is_ascii_lowercase()
            .then(|| u32::from(c as u8) - u32::from(b'b'))
    };
    let valid = |c: char| ('b'..='z').contains(&c);
    let mut chars = letters.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(a), None, _) if valid(a) => index(a),
        (Some(a), Some(b), None) if valid(a) && valid(b) => {
            Some((index(a)? + 1) * LETTERS + index(b)?)
        }
        _ => None,
    }
}

impl fmt::Display for Designation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.body {
            BodyPath::Star => write!(f, "{CATALOGUE} {} A", self.system.0),
            BodyPath::Planet { rank } => {
                write!(f, "{CATALOGUE} {} {}", self.system.0, planet_letters(rank))
            }
        }
    }
}

impl FromStr for Designation {
    type Err = ContractError;

    fn from_str(text: &str) -> Result<Designation, ContractError> {
        let bad = || ContractError::InvalidDesignation(text.to_string());
        let mut parts = text.split(' ');
        let (Some(CATALOGUE), Some(number), Some(body), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(bad());
        };
        if number.len() != 5 || !number.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
        let system = number
            .parse()
            .ok()
            .and_then(|n| SystemNumber::new(n).ok())
            .ok_or_else(bad)?;
        let body = if body == "A" {
            BodyPath::Star
        } else {
            let rank = rank_of(body).filter(|r| *r < MAX_RANKS).ok_or_else(bad)?;
            BodyPath::Planet { rank }
        };
        Ok(Designation { system, body })
    }
}

impl Serialize for Designation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Designation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Designation, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_follow_the_legacy_scheme() {
        assert_eq!(planet_letters(0), "b");
        assert_eq!(planet_letters(24), "z");
        assert_eq!(planet_letters(25), "bb");
    }

    #[test]
    fn designations_print_parse_and_sort() {
        for text in ["MGS 12345 A", "MGS 12345 b", "MGS 12345 bb", "MGS 99999 zz"] {
            let d: Designation = text.parse().unwrap();
            assert_eq!(d.to_string(), text);
        }
        for bad in ["MGS 123 b", "MGS 12345 a", "XYZ 12345 b", "MGS 12345 B"] {
            assert!(bad.parse::<Designation>().is_err(), "{bad}");
        }
        let star: Designation = "MGS 12345 A".parse().unwrap();
        let b: Designation = "MGS 12345 b".parse().unwrap();
        let bb: Designation = "MGS 12345 bb".parse().unwrap();
        assert!(star < b && b < bb);
    }

    #[test]
    fn every_rank_round_trips_and_the_range_is_enforced() {
        for rank in 0..650 {
            let text = format!("MGS 10000 {}", planet_letters(rank));
            let d: Designation = text.parse().unwrap();
            assert_eq!(d.body, BodyPath::Planet { rank });
        }
        for bad in [
            "MGS 10000 bbb",
            "MGS 100000 b",
            "MGS 09999 b",
            "MGS 10000 ",
            "MGS 10000 {b",
        ] {
            assert!(bad.parse::<Designation>().is_err(), "{bad}");
        }
    }

    #[test]
    fn a_designation_serializes_as_its_text() {
        let d: Designation = "MGS 12345 c".parse().unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "\"MGS 12345 c\"");
        assert_eq!(serde_json::from_str::<Designation>(&json).unwrap(), d);
        assert!(serde_json::from_str::<Designation>("\"nope\"").is_err());
    }
}
