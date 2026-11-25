//! The error type of the system engine.

use science::stellar::{MAX_MASS, MIN_MASS, StellarError};
use science::units::{Dex, Gyr, SolarMass};
use std::fmt;
use timeline::ContractError;

use crate::ArchetypeId;
use crate::planet::InteriorError;

/// Why the system engine cannot compute a system. Codes with parameters,
/// in the style of [`ContractError`]. More variants arrive with the
/// planet model, hence `non_exhaustive`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SystemError {
    /// A violation of the timeline contract.
    Contract(ContractError),
    /// An asked input is NaN, which no range can place. `input` is the
    /// name of the command-line flag: `present`, `host-mass` or
    /// `metallicity`. Infinities are not NaN and give the range errors.
    NotANumber { input: &'static str },
    /// A host star outside the tables (0.08-8 M☉; above 8 see #615).
    HostMassOutOfRange { mass: SolarMass },
    /// A host metallicity outside the tables.
    MetallicityOutOfRange { metallicity: Dex },
    /// The present date is before the host reaches the main sequence.
    PresentBeforeZams { present: Gyr, zams: Gyr },
    /// The present date is after the host's life (`end` is the lesser of
    /// its life and 13.8 Gyr).
    PresentAfterLife { present: Gyr, end: Gyr },
    /// A stellar error that carries no present date (`BeforeZams`,
    /// `AfterTams`), kept as is by the `From<StellarError>` conversion.
    Stellar(StellarError),
    /// An asked archetype does not fit in the system.
    NoRoom { archetype: ArchetypeId },
    /// An asked archetype names a host that disagrees with the others.
    IncompatibleHost { archetype: ArchetypeId },
    /// An asked archetype is not known (moons included: they are T2).
    UnknownArchetype { archetype: ArchetypeId },
    /// The request is a single body, which the system draw does not draw.
    BodyRequestNotDrawn,
    /// The request asks more planets than a system can hold.
    TooManyPlanets { count: u64 },
    /// The planet chain could not compute a planet.
    Planet(InteriorError),
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => write!(f, "system.contract: {error}"),
            Self::NotANumber { input } => write!(f, "system.not-a-number: {input}"),
            Self::HostMassOutOfRange { mass } => write!(
                f,
                "system.host-mass-out-of-range: {} M☉ ({}-{}; above {} see #615)",
                mass.0, MIN_MASS.0, MAX_MASS.0, MAX_MASS.0
            ),
            Self::MetallicityOutOfRange { metallicity } => {
                write!(f, "system.metallicity-out-of-range: {} dex", metallicity.0)
            }
            Self::PresentBeforeZams { present, zams } => write!(
                f,
                "system.present-before-zams: present={} Gyr, zams={} Gyr",
                present.0, zams.0
            ),
            Self::PresentAfterLife { present, end } => write!(
                f,
                "system.present-after-life: present={} Gyr, end={} Gyr",
                present.0, end.0
            ),
            Self::Stellar(error) => write!(f, "system.stellar: {error}"),
            Self::NoRoom { archetype } => {
                write!(f, "system.no-room: {}", archetype.0.as_str())
            }
            Self::IncompatibleHost { archetype } => {
                write!(f, "system.incompatible-host: {}", archetype.0.as_str())
            }
            Self::UnknownArchetype { archetype } => {
                write!(f, "system.unknown-archetype: {}", archetype.0.as_str())
            }
            Self::BodyRequestNotDrawn => write!(f, "system.body-request-not-drawn"),
            Self::TooManyPlanets { count } => write!(f, "system.too-many-planets: {count}"),
            Self::Planet(error) => write!(f, "system.planet: {error}"),
        }
    }
}

impl std::error::Error for SystemError {}

impl From<ContractError> for SystemError {
    fn from(error: ContractError) -> Self {
        Self::Contract(error)
    }
}

impl From<StellarError> for SystemError {
    fn from(error: StellarError) -> Self {
        match error {
            StellarError::MassOutOfRange(mass) => Self::HostMassOutOfRange { mass },
            StellarError::MetallicityOutOfRange(metallicity) => {
                Self::MetallicityOutOfRange { metallicity }
            }
            other => Self::Stellar(other),
        }
    }
}

impl From<InteriorError> for SystemError {
    fn from(error: InteriorError) -> Self {
        Self::Planet(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_nan_input() {
        let message = SystemError::NotANumber { input: "present" }.to_string();
        assert_eq!(message, "system.not-a-number: present");
    }

    #[test]
    fn display_names_the_issue_above_eight() {
        let message = SystemError::HostMassOutOfRange {
            mass: SolarMass(9.0),
        }
        .to_string();
        assert!(message.contains("#615"), "{message}");
        assert!(message.starts_with("system.host-mass-out-of-range"));
        assert!(message.contains("(0.08-8; above 8 see #615)"), "{message}");
    }

    #[test]
    fn stellar_errors_convert() {
        assert_eq!(
            SystemError::from(StellarError::MassOutOfRange(SolarMass(9.0))),
            SystemError::HostMassOutOfRange {
                mass: SolarMass(9.0)
            }
        );
        assert_eq!(
            SystemError::from(StellarError::MetallicityOutOfRange(Dex(2.0))),
            SystemError::MetallicityOutOfRange {
                metallicity: Dex(2.0)
            }
        );
        let before = StellarError::BeforeZams { zams: Gyr(0.1) };
        assert_eq!(SystemError::from(before), SystemError::Stellar(before));
        assert_eq!(
            SystemError::from(ContractError::NoSource),
            SystemError::Contract(ContractError::NoSource)
        );
    }
}
