//! Errors of the world container: codes and numbers only, never prose.

use std::error::Error;
use std::fmt;

use timeline::Code;

/// Why a field was refused by a [`World`](crate::World).
#[derive(Clone, Debug, PartialEq)]
pub enum WorldError {
    /// The field does not hold one value per cell.
    WrongLength {
        /// The field's code.
        field: Code,
        /// The grid's cell count.
        expected: usize,
        /// The number of values given.
        found: usize,
    },
    /// A field with this code is already in the world.
    Duplicate(Code),
    /// A categorical field must hold integers (`U8` or `U16`).
    Categorical(Code),
    /// A field the reader needs is not in the world.
    Missing(Code),
    /// The storage mode does not fit the field (`U16` needs an `F32`
    /// field and a finite, non-empty range).
    Storage(Code),
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldError::WrongLength {
                field,
                expected,
                found,
            } => write!(
                f,
                "world.wrong-length {} expected={expected} found={found}",
                field.as_str()
            ),
            WorldError::Duplicate(c) => write!(f, "world.duplicate {}", c.as_str()),
            WorldError::Categorical(c) => write!(f, "world.categorical {}", c.as_str()),
            WorldError::Missing(c) => write!(f, "world.missing {}", c.as_str()),
            WorldError::Storage(c) => write!(f, "world.storage {}", c.as_str()),
        }
    }
}

impl Error for WorldError {}
