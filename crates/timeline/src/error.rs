//! The errors of the contract: codes with parameters, never prose.

use std::fmt;

/// A violation of the timeline contract.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ContractError {
    /// A designation that does not parse or is out of range.
    InvalidDesignation(String),
    /// A cause or event code that is not a valid code.
    InvalidCode(String),
    /// A number that is NaN or infinite, found under `code`.
    NotFinite { code: String },
    /// A reference to a cause node that does not exist.
    UnknownNode { node: u32 },
    /// A star without any source.
    NoSource,
    /// More sources than a keyframe can hold.
    TooManySources(usize),
    /// An orbit whose eccentricity is 1 or more.
    EccentricityNotBelowOne,
    /// Interior layers or samples not ordered by radius.
    LayersNotOrdered,
    /// Keyframes not ordered by date.
    KeyframesNotOrdered,
    /// A timeline without keyframe.
    NoKeyframe,
    /// Keyframes of different kinds of body in one timeline.
    MixedBodyKinds,
    /// A designation of one kind of body on keyframes of another.
    BodyKindMismatch,
    /// A planet state whose age is not its keyframe's date.
    AgeMismatch,
    /// A present date outside the span of the keyframes.
    PresentOutsideKeyframes,
    /// Stellar sources not in non-increasing order of mass.
    SourcesNotOrdered,
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractError::InvalidDesignation(text) => write!(f, "invalid_designation({text})"),
            ContractError::InvalidCode(text) => write!(f, "invalid_code({text})"),
            ContractError::NotFinite { code } => write!(f, "not_finite(code={code})"),
            ContractError::UnknownNode { node } => write!(f, "unknown_node(node={node})"),
            ContractError::NoSource => write!(f, "no_source"),
            ContractError::TooManySources(n) => write!(f, "too_many_sources({n})"),
            ContractError::EccentricityNotBelowOne => write!(f, "eccentricity_not_below_one"),
            ContractError::LayersNotOrdered => write!(f, "layers_not_ordered"),
            ContractError::KeyframesNotOrdered => write!(f, "keyframes_not_ordered"),
            ContractError::NoKeyframe => write!(f, "no_keyframe"),
            ContractError::MixedBodyKinds => write!(f, "mixed_body_kinds"),
            ContractError::BodyKindMismatch => write!(f, "body_kind_mismatch"),
            ContractError::AgeMismatch => write!(f, "age_mismatch"),
            ContractError::PresentOutsideKeyframes => write!(f, "present_outside_keyframes"),
            ContractError::SourcesNotOrdered => write!(f, "sources_not_ordered"),
        }
    }
}

impl std::error::Error for ContractError {}
