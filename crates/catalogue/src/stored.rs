//! A system as stored: its inputs, the only truth.

use serde::{Deserialize, Serialize};
use system::{INPUTS_SCHEMA, Inputs};
use timeline::EngineVersion;

use crate::{CatalogueError, SystemId};

/// The stored record of a system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoredSystem {
    pub id: SystemId,
    pub inputs: Inputs,
    /// The engine version that last generated it.
    pub engine: EngineVersion,
    pub name: Option<String>,
}

impl StoredSystem {
    /// A record for `inputs`, unnamed, its id derived from them.
    pub fn new(inputs: Inputs, engine: EngineVersion) -> StoredSystem {
        StoredSystem {
            id: SystemId::of(&inputs),
            inputs,
            engine,
            name: None,
        }
    }

    /// Refuses inputs whose schema this build does not read.
    pub fn check_schema(&self) -> Result<(), CatalogueError> {
        if self.inputs.schema == INPUTS_SCHEMA {
            Ok(())
        } else {
            Err(CatalogueError::UnknownSchema {
                found: self.inputs.schema,
                expected: INPUTS_SCHEMA,
            })
        }
    }
}
