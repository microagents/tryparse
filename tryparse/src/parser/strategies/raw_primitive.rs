//! Strategy for parsing raw primitive values (booleans, numbers, strings)
//!
//! This strategy handles inputs that are primitives but not wrapped in JSON, like:
//! - "true" → Bool(true)
//! - "12,111" → Number(12111)
//! - "The answer is true" → String("true") extracted
//!
//! This matches BAML's behavior where raw text can be parsed as primitives.

use serde_json::Value;

use super::ParsingStrategy;
use crate::{
    error::Result,
    value::{FlexValue, Source},
};

#[derive(Debug, Default)]
pub struct RawPrimitiveStrategy;

impl RawPrimitiveStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Try to parse input as a raw boolean
    fn try_as_bool(&self, input: &str) -> Option<FlexValue> {
        let trimmed = input.trim();

        // Direct boolean values
        match trimmed.to_lowercase().as_str() {
            "true" => Some(FlexValue::new(
                Value::Bool(true),
                Source::Heuristic {
                    pattern: "raw_bool".to_string(),
                },
            )),
            "false" => Some(FlexValue::new(
                Value::Bool(false),
                Source::Heuristic {
                    pattern: "raw_bool".to_string(),
                },
            )),
            _ => {
                // Try to extract boolean from text like "The answer is true"
                self.extract_bool_from_text(input)
            }
        }
    }

    /// Extract boolean from natural language text
    ///
    /// Handles patterns like:
    /// - "The answer is true"
    /// - "Answer: **True**"
    /// - "False. The explanation..."
    fn extract_bool_from_text(&self, input: &str) -> Option<FlexValue> {
        let lower = input.to_lowercase();

        // Count occurrences of true and false
        let true_count = lower.matches("true").count();
        let false_count = lower.matches("false").count();

        // Only succeed if one appears exactly once
        // BAML rejects ambiguous cases like "true or false"
        if true_count == 1 && false_count == 0 {
            Some(FlexValue::new(
                Value::Bool(true),
                Source::Heuristic {
                    pattern: "bool_from_text".to_string(),
                },
            ))
        } else if false_count == 1 && true_count == 0 {
            Some(FlexValue::new(
                Value::Bool(false),
                Source::Heuristic {
                    pattern: "bool_from_text".to_string(),
                },
            ))
        } else {
            None
        }
    }

    /// Try to parse input as a raw number
    fn try_as_number(&self, input: &str) -> Option<FlexValue> {
        let trimmed = input.trim().trim_end_matches(',');

        // Try integer first
        if let Ok(n) = trimmed.parse::<i64>() {
            if let Some(num) = serde_json::Number::from_f64(n as f64) {
                return Some(FlexValue::new(
                    Value::Number(num),
                    Source::Heuristic {
                        pattern: "raw_number".to_string(),
                    },
                ));
            }
        }

        // Try float
        if let Ok(f) = trimmed.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(f) {
                return Some(FlexValue::new(
                    Value::Number(num),
                    Source::Heuristic {
                        pattern: "raw_number".to_string(),
                    },
                ));
            }
        }

        // Try comma-separated numbers (handled by parser, but might have commas)
        let without_commas = trimmed.replace(",", "");
        if let Ok(f) = without_commas.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(f) {
                return Some(FlexValue::new(
                    Value::Number(num),
                    Source::Heuristic {
                        pattern: "raw_number_with_commas".to_string(),
                    },
                ));
            }
        }

        None
    }

    /// Try to treat input as a raw string
    fn as_string(&self, input: &str) -> FlexValue {
        // If all else fails, it's a string
        FlexValue::new(
            Value::String(input.to_string()),
            Source::Heuristic {
                pattern: "raw_string".to_string(),
            },
        )
    }
}

impl ParsingStrategy for RawPrimitiveStrategy {
    fn name(&self) -> &'static str {
        "raw_primitive"
    }

    fn parse(&self, input: &str) -> Result<Vec<FlexValue>> {
        let mut candidates = Vec::new();
        let trimmed = input.trim();

        // Skip if input looks like complete JSON or structured data
        // These should be handled by DirectJsonStrategy, JsonFixerStrategy, etc.
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return Ok(vec![]);
        }

        // ALSO skip if input CONTAINS JSON-like patterns (not just starts with them)
        // This ensures HeuristicExtractor gets a chance to extract embedded JSON
        if trimmed.contains('{') || trimmed.contains('[') {
            return Ok(vec![]);
        }

        // For inputs starting with quote, check if it's a complete JSON string
        // Complete JSON string: starts and ends with quote (with no unescaped quote in middle)
        // Incomplete string: `"hello` should be handled as raw string
        if trimmed.starts_with('"') {
            // Check if this looks like a complete JSON string by trying to parse it
            if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                // It's valid JSON, let DirectJsonStrategy handle it
                return Ok(vec![]);
            }
            // Otherwise, it's an incomplete string - we should handle it as raw text
        }

        // Only try if input is relatively short (< 1000 chars)
        // Longer inputs are likely to be prose or complex structured data
        if input.len() > 1000 {
            return Ok(vec![]);
        }

        // Try boolean first (most specific)
        if let Some(bool_value) = self.try_as_bool(input) {
            candidates.push(bool_value);
        }

        // Try number
        if let Some(number_value) = self.try_as_number(input) {
            candidates.push(number_value);
        }

        // Always include string interpretation as fallback
        // This ensures we always have at least one candidate
        candidates.push(self.as_string(input));

        Ok(candidates)
    }

    fn priority(&self) -> u8 {
        // Run after JSON parsing (1) and markdown (2), before JSON fixing (10)
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_raw_bool_true() {
        let strategy = RawPrimitiveStrategy::new();
        let result = strategy.parse("true").unwrap();

        assert!(!result.is_empty());
        // Should have bool and string candidates
        assert!(result.iter().any(|v| matches!(v.value, Value::Bool(true))));
    }

    #[test]
    fn test_parse_raw_bool_false() {
        let strategy = RawPrimitiveStrategy::new();
        let result = strategy.parse("False").unwrap(); // Case-insensitive

        assert!(!result.is_empty());
        assert!(result.iter().any(|v| matches!(v.value, Value::Bool(false))));
    }

    #[test]
    fn test_parse_bool_in_text() {
        let strategy = RawPrimitiveStrategy::new();
        let result = strategy.parse("The answer is true").unwrap();

        assert!(!result.is_empty());
        assert!(result.iter().any(|v| matches!(v.value, Value::Bool(true))));
    }

    #[test]
    fn test_parse_number() {
        let strategy = RawPrimitiveStrategy::new();
        let result = strategy.parse("12111").unwrap();

        assert!(!result.is_empty());
        assert!(result.iter().any(|v| matches!(v.value, Value::Number(_))));
    }

    #[test]
    fn test_parse_number_with_commas() {
        let strategy = RawPrimitiveStrategy::new();
        let result = strategy.parse("12,111").unwrap();

        assert!(!result.is_empty());
        // Should find a number
        assert!(result.iter().any(|v| matches!(v.value, Value::Number(_))));
    }

    #[test]
    fn test_parse_as_string_fallback() {
        let strategy = RawPrimitiveStrategy::new();
        let result = strategy.parse("some random text").unwrap();

        assert!(!result.is_empty());
        // Should at least have a string candidate
        assert!(result.iter().any(|v| matches!(v.value, Value::String(_))));
    }

    #[test]
    fn test_long_input_skipped() {
        let strategy = RawPrimitiveStrategy::new();
        let long_input = "a".repeat(2000);
        let result = strategy.parse(&long_input).unwrap();

        // Should skip long inputs
        assert!(result.is_empty());
    }

    #[test]
    fn test_ambiguous_bool_not_extracted() {
        let strategy = RawPrimitiveStrategy::new();
        let result = strategy.parse("The answer is true or false").unwrap();

        // Should NOT extract boolean because it's ambiguous
        assert!(!result.iter().any(|v| matches!(v.value, Value::Bool(_))));
    }
}
