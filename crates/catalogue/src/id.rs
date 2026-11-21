//! The identity of a stored system: a hash of its inputs.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use system::Inputs;
use timeline::fnv1a64;

/// What the catalogue refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogueError {
    /// The stored inputs have a schema this build does not read.
    UnknownSchema { found: u32, expected: u32 },
    /// Text that is not a system id.
    BadId(String),
    /// A stored record whose id is not the hash of its inputs.
    IdMismatch {
        /// The id the record carries.
        stored: SystemId,
        /// The id its inputs hash to.
        computed: SystemId,
    },
}

impl fmt::Display for CatalogueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogueError::UnknownSchema { found, expected } => {
                write!(f, "unknown_schema(found={found}, expected={expected})")
            }
            CatalogueError::BadId(text) => write!(f, "bad_id({text})"),
            CatalogueError::IdMismatch { stored, computed } => {
                write!(f, "id_mismatch(stored={stored}, computed={computed})")
            }
        }
    }
}

impl std::error::Error for CatalogueError {}

/// The id of a system: FNV-1a 64 of the JSON of its [`Inputs`]. Equal
/// inputs give the same id; the text form is 16 lowercase hex digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemId(u64);

impl SystemId {
    /// The id of `inputs`.
    pub fn of(inputs: &Inputs) -> SystemId {
        // Inputs are plain data: serialising them cannot fail.
        let json = serde_json::to_string(inputs).expect("inputs serialise to JSON");
        SystemId(fnv1a64(json.as_bytes()))
    }
}

impl fmt::Display for SystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl FromStr for SystemId {
    type Err = CatalogueError;

    fn from_str(text: &str) -> Result<SystemId, CatalogueError> {
        let valid = text.len() == 16
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !valid {
            return Err(CatalogueError::BadId(text.to_string()));
        }
        u64::from_str_radix(text, 16)
            .map(SystemId)
            .map_err(|_| CatalogueError::BadId(text.to_string()))
    }
}

impl Serialize for SystemId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for SystemId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<SystemId, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}
