//! The error type of the system engine.

use science::stellar::StellarError;
use science::units::{Dex, Gyr, SolarMass};
use std::fmt;
use timeline::ContractError;

/// Why the system engine cannot compute a system. Codes with parameters,
/// in the style of [`ContractError`]. More variants arrive with the
/// planet model, hence `non_exhaustive`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SystemError {
    /// A violation of the timeline contract.
    Contract(ContractError),
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
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => write!(f, "system.contract: {error}"),
            Self::HostMassOutOfRange { mass } => write!(
                f,
                "system.host-mass-out-of-range: {} M☉ (0.08-8; above 8 see #615)",
                mass.0
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_issue_above_eight() {
        let message = SystemError::HostMassOutOfRange {
            mass: SolarMass(9.0),
        }
        .to_string();
        assert!(message.contains("#615"), "{message}");
        assert!(message.starts_with("system.host-mass-out-of-range"));
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
