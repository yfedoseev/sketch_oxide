//! Error types for sketch operations

use std::fmt;

/// Errors that can occur during sketch operations
///
/// This enum is `#[non_exhaustive]`: downstream code must include a wildcard
/// arm when matching, which lets new variants be added without a breaking
/// (semver-major) change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SketchError {
    /// Invalid parameter provided to sketch constructor or operation
    InvalidParameter {
        /// Parameter name
        param: String,
        /// Invalid value provided
        value: String,
        /// Constraint that was violated
        constraint: String,
    },

    /// Error during serialization
    SerializationError(String),

    /// Error during deserialization
    DeserializationError(String),

    /// Attempted to merge incompatible sketches
    IncompatibleSketches {
        /// Reason for incompatibility
        reason: String,
    },

    /// Error during reconciliation operations
    ReconciliationError {
        /// Reason for reconciliation failure
        reason: String,
    },

    /// The requested operation is not supported by this sketch
    ///
    /// Used by immutable/build-once structures (e.g. Binary Fuse filters) that
    /// cannot support in-place streaming `update`, and by types whose format is
    /// not yet implemented.
    Unsupported {
        /// The name of the unsupported operation
        op: &'static str,
    },

    /// The sketch is in an invalid state for the requested operation
    InvalidState {
        /// Description of the invalid state
        reason: String,
    },
}

impl fmt::Display for SketchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SketchError::InvalidParameter {
                param,
                value,
                constraint,
            } => {
                write!(
                    f,
                    "Invalid parameter '{}': value '{}' {}",
                    param, value, constraint
                )
            }
            SketchError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            SketchError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            SketchError::IncompatibleSketches { reason } => {
                write!(f, "Incompatible sketches: {}", reason)
            }
            SketchError::ReconciliationError { reason } => {
                write!(f, "Reconciliation error: {}", reason)
            }
            SketchError::Unsupported { op } => {
                write!(f, "Unsupported operation: {}", op)
            }
            SketchError::InvalidState { reason } => {
                write!(f, "Invalid state: {}", reason)
            }
        }
    }
}

impl std::error::Error for SketchError {}

/// Result type alias for sketch operations
pub type Result<T> = std::result::Result<T, SketchError>;
