//! Error types for LLM-to-struct parsing.

use std::fmt;

/// Result type alias for parsing operations.
pub type Result<T> = std::result::Result<T, ParseError>;

/// Errors that can occur during parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// No valid candidates were found after trying all parsing strategies.
    #[error("No valid candidates found in response")]
    NoCandidates,

    /// All parsing strategies failed.
    #[error("All parsing strategies failed")]
    AllStrategiesFailed {
        /// Details of each failed strategy attempt.
        attempts: Vec<StrategyError>,
    },

    /// Deserialization failed for all candidates.
    #[error("Deserialization failed: {0}")]
    DeserializeFailed(#[from] DeserializeError),

    /// JSON parsing error from serde_json.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Configuration error.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Details of a failed parsing strategy attempt.
#[derive(Debug, Clone)]
pub struct StrategyError {
    /// Name of the strategy that failed.
    pub strategy: &'static str,
    /// Error message describing why it failed.
    pub error: String,
}

impl StrategyError {
    /// Creates a new strategy error.
    #[inline]
    pub fn new(strategy: &'static str, error: impl Into<String>) -> Self {
        Self {
            strategy,
            error: error.into(),
        }
    }
}

impl fmt::Display for StrategyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.strategy, self.error)
    }
}

/// Errors that occur during deserialization.
#[derive(Debug, thiserror::Error)]
pub enum DeserializeError {
    /// Type mismatch between expected and found types.
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        /// Expected type name.
        expected: &'static str,
        /// Found type name.
        found: String,
    },

    /// Required field is missing from the input.
    #[error("Missing required field: {field}")]
    MissingField {
        /// Name of the missing field.
        field: String,
    },

    /// Invalid value encountered.
    #[error("Invalid value: {message}")]
    InvalidValue {
        /// Description of why the value is invalid.
        message: String,
    },

    /// Unknown variant for enum.
    #[error("Unknown variant '{variant}' for enum {enum_name}")]
    UnknownVariant {
        /// Enum type name.
        enum_name: String,
        /// The variant that was not recognized.
        variant: String,
    },

    /// Custom error message.
    #[error("{0}")]
    Custom(String),

    /// Depth limit exceeded during recursive deserialization.
    #[error("Depth limit exceeded: {depth} >= {max_depth}")]
    DepthLimitExceeded {
        /// Current depth.
        depth: usize,
        /// Maximum allowed depth.
        max_depth: usize,
    },

    /// Circular reference detected during deserialization.
    #[error("Circular reference detected for type: {type_name}")]
    CircularReference {
        /// Type name where the cycle was detected.
        type_name: String,
    },
}

impl DeserializeError {
    /// Creates a type mismatch error.
    #[inline]
    pub fn type_mismatch(expected: &'static str, found: impl Into<String>) -> Self {
        Self::TypeMismatch {
            expected,
            found: found.into(),
        }
    }

    /// Creates a missing field error.
    #[inline]
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    /// Creates an invalid value error.
    #[inline]
    pub fn invalid_value(message: impl Into<String>) -> Self {
        Self::InvalidValue {
            message: message.into(),
        }
    }
}

impl serde::de::Error for DeserializeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_error_display() {
        let err = StrategyError::new("test_strategy", "something went wrong");
        assert_eq!(err.to_string(), "test_strategy: something went wrong");
    }

    #[test]
    fn test_deserialize_error_type_mismatch() {
        let err = DeserializeError::type_mismatch("integer", "string");
        assert!(err.to_string().contains("integer"));
        assert!(err.to_string().contains("string"));
    }

    #[test]
    fn test_parse_error_from_json() {
        let json_err = serde_json::from_str::<u32>("not a number").unwrap_err();
        let parse_err: ParseError = json_err.into();
        assert!(matches!(parse_err, ParseError::JsonError(_)));
    }
}
