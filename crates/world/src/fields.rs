//! The world container: one grid, one body, one keyframe, and the fields
//! the generator laid on the grid, ordered by code.

use std::collections::BTreeMap;

use timeline::{Code, Designation, EngineVersion};

use crate::error::WorldError;
use crate::grid::Grid;

/// The code of a field, such as `relief.height`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldId(pub Code);

/// One value per cell, in the field's own type.
#[derive(Clone, Debug, PartialEq)]
pub enum Values {
    /// Continuous values.
    F32(Vec<f32>),
    /// Integers up to 65 535 (categories, indices).
    U16(Vec<u16>),
    /// Integers up to 255 (categories, flags).
    U8(Vec<u8>),
}

impl Values {
    /// The number of values.
    pub fn len(&self) -> usize {
        match self {
            Values::F32(v) => v.len(),
            Values::U16(v) => v.len(),
            Values::U8(v) => v.len(),
        }
    }

    /// Whether there is no value.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The value of cell `k`, as `f32`.
    ///
    /// # Panics
    ///
    /// When `k` is out of range.
    pub fn get_f32(&self, k: usize) -> f32 {
        match self {
            Values::F32(v) => v[k],
            Values::U16(v) => f32::from(v[k]),
            Values::U8(v) => f32::from(v[k]),
        }
    }
}

/// How an `F32` field is written to disk.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Storage {
    /// Every bit kept.
    Exact,
    /// `[min, max]` mapped linearly onto `0..=65535`.
    U16 {
        /// The value stored as 0.
        min: f32,
        /// The value stored as 65 535.
        max: f32,
    },
}

/// One layer of the world.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// The values, one per cell.
    pub values: Values,
    /// Whether the values are categories (never interpolated).
    pub categorical: bool,
    /// How the field is written to disk.
    pub storage: Storage,
}

/// A body's state laid on a grid.
#[derive(Clone, Debug, PartialEq)]
pub struct World {
    grid: Grid,
    body: Designation,
    keyframe: u32,
    engine: EngineVersion,
    world_engine: EngineVersion,
    fields: BTreeMap<FieldId, Field>,
}

impl World {
    /// An empty world for `body` at `keyframe`, made from a timeline of
    /// the system engine `engine` by the map generator `world_engine`.
    pub fn new(
        grid: Grid,
        body: Designation,
        keyframe: u32,
        engine: EngineVersion,
        world_engine: EngineVersion,
    ) -> World {
        World {
            grid,
            body,
            keyframe,
            engine,
            world_engine,
            fields: BTreeMap::new(),
        }
    }

    /// The grid.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// The body.
    pub fn body(&self) -> &Designation {
        &self.body
    }

    /// The keyframe index.
    pub fn keyframe(&self) -> u32 {
        self.keyframe
    }

    /// The system engine version of the timeline the world was built from.
    pub fn engine(&self) -> EngineVersion {
        self.engine
    }

    /// The version of the map generator that made the world.
    pub fn world_engine(&self) -> EngineVersion {
        self.world_engine
    }

    /// Adds a field.
    ///
    /// # Errors
    ///
    /// `WrongLength` when it has not one value per cell, `Duplicate`
    /// when the code is taken, `Categorical` when a categorical field is
    /// `F32`, `Storage` when `U16` storage has no finite range, the
    /// field is not `F32`, or it holds a non-finite value.
    pub fn insert(&mut self, id: FieldId, field: Field) -> Result<(), WorldError> {
        let expected = self.grid.cell_count();
        let found = field.values.len();
        if found != expected {
            return Err(WorldError::WrongLength {
                field: id.0,
                expected,
                found,
            });
        }
        if self.fields.contains_key(&id) {
            return Err(WorldError::Duplicate(id.0));
        }
        if field.categorical && matches!(field.values, Values::F32(_)) {
            return Err(WorldError::Categorical(id.0));
        }
        if let Storage::U16 { min, max } = field.storage {
            // NaN would quantise to 0 and infinity to an end of the range.
            let ranged = matches!(&field.values, Values::F32(v) if v.iter().all(|x| x.is_finite()))
                && min.is_finite()
                && max.is_finite()
                && min < max;
            if !ranged {
                return Err(WorldError::Storage(id.0));
            }
        }
        self.fields.insert(id, field);
        Ok(())
    }

    /// The field with this code, if any.
    pub fn field(&self, id: &FieldId) -> Option<&Field> {
        self.fields.get(id)
    }

    /// The fields, in code order.
    pub fn fields(&self) -> impl Iterator<Item = (&FieldId, &Field)> {
        self.fields.iter()
    }
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod tests;
