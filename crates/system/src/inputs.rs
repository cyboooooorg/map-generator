//! The inputs: the only stored truth. Timelines, worlds and exports are
//! caches derived from them and invalidated by
//! [`ENGINE_VERSION`](crate::ENGINE_VERSION).
//!
//! mapgen3 may refine the request and mapgen1 the archetypes in their
//! pull requests.

use std::collections::BTreeMap;

use science::units::{Dex, Gyr, SolarMass};
use serde::{Deserialize, Serialize};
use timeline::Seed;

use crate::ArchetypeId;

/// The schema of [`Inputs`]: bumped when a stored input changes shape.
pub const INPUTS_SCHEMA: u32 = 1;

/// What is asked for: a whole system or a single body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Request {
    System(SystemRequest),
    Body(BodyRequest),
}

/// A whole system. `None` leaves the value to the draw.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SystemRequest {
    pub host_mass: Option<SolarMass>,
    pub metallicity: Option<Dex>,
    /// A count per archetype.
    pub archetypes: BTreeMap<ArchetypeId, u32>,
}

/// A single body. `None` leaves the value to the draw.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyRequest {
    pub host_mass: Option<SolarMass>,
    pub metallicity: Option<Dex>,
    pub archetype: Option<ArchetypeId>,
}

/// The stored truth of a generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inputs {
    pub schema: u32,
    pub seed: Seed,
    /// The present date. `None`: the draw takes the age within the
    /// star's lifetime.
    pub present: Option<Gyr>,
    pub request: Request,
}

#[cfg(test)]
mod tests {
    use super::*;
    use timeline::Code;

    #[test]
    fn inputs_round_trip_through_json() {
        let inputs = Inputs {
            schema: INPUTS_SCHEMA,
            seed: Seed::new(7),
            present: Some(Gyr(4.57)),
            request: Request::Body(BodyRequest {
                host_mass: None,
                metallicity: None,
                archetype: Some(ArchetypeId(Code::of("earth-like"))),
            }),
        };
        let json = serde_json::to_string(&inputs).unwrap();
        assert_eq!(serde_json::from_str::<Inputs>(&json).unwrap(), inputs);
    }

    #[test]
    fn system_request_round_trips_with_counts() {
        let mut archetypes = BTreeMap::new();
        archetypes.insert(ArchetypeId(Code::of("earth-like")), 2);
        let inputs = Inputs {
            schema: INPUTS_SCHEMA,
            seed: Seed::new(1),
            present: None,
            request: Request::System(SystemRequest {
                host_mass: Some(SolarMass(1.0)),
                metallicity: Some(Dex(0.0)),
                archetypes,
            }),
        };
        let json = serde_json::to_string(&inputs).unwrap();
        assert_eq!(serde_json::from_str::<Inputs>(&json).unwrap(), inputs);
    }
}
