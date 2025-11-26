//! Error types for sketch operations

use std::fmt;

/// Errors that can occur during sketch operations
#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
    }
}

impl std::error::Error for SketchError {}

/// Result type alias for sketch operations
pub type Result<T> = std::result::Result<T, SketchError>;
