//! The state of a star at a date.

use crate::macros::{contract_enum, traced_fields};
use crate::{ContractError, InteriorProfile, Traced};
use science::units::{Dex, Kelvin, SolarLuminosity, SolarMass, SolarRadius};
use serde::{Deserialize, Serialize};

contract_enum! {
    /// The evolutionary phase of a star.
    StarPhase {
        MainSequence => "main-sequence",
    }
}

/// A star at a date.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StarState {
    pub phase: StarPhase,
    pub mass: Traced<SolarMass>,
    pub radius: Traced<SolarRadius>,
    pub luminosity: Traced<SolarLuminosity>,
    pub effective_temperature: Traced<Kelvin>,
    pub metallicity: Traced<Dex>,
    /// None until the stellar structure is studied.
    pub interior: Option<InteriorProfile>,
}

traced_fields!(StarState {
    mass,
    radius,
    luminosity,
    effective_temperature,
    metallicity,
    interior
});

impl StarState {
    /// The interior, when present, is well formed.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.interior
            .as_ref()
            .map_or(Ok(()), InteriorProfile::validate)
    }
}
