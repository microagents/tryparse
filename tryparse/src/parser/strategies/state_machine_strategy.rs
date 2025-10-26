//! Parsing strategy that uses the state machine parser.
//!
//! This strategy provides more robust parsing of malformed JSON by using
//! a token-by-token state machine instead of regex-based approaches.

use super::ParsingStrategy;
use crate::{error::Result, parser::state_machine::StateMachineParser, value::FlexValue};

/// Strategy that uses the state machine parser for robust JSON parsing.
///
/// This strategy is particularly effective at:
/// - Handling unclosed collections (auto-closes)
/// - Parsing multiple top-level JSON objects
/// - Context-aware string handling
/// - Maintaining parse state through nested structures
#[derive(Debug, Clone)]
pub struct StateMachineStrategy {
    parser: StateMachineParser,
}

impl Default for StateMachineStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachineStrategy {
    /// Creates a new state machine strategy.
    pub fn new() -> Self {
        Self {
            parser: StateMachineParser::new(),
        }
    }
}

impl ParsingStrategy for StateMachineStrategy {
    fn name(&self) -> &'static str {
        "state_machine"
    }

    fn priority(&self) -> u8 {
        // Priority 15: After direct JSON (1) and JSON fixer (10),
        // but before heuristic (20)
        15
    }

    fn parse(&self, input: &str) -> Result<Vec<FlexValue>> {
        let mut parser = self.parser.clone();
        parser.parse(input)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_strategy_name() {
        let strategy = StateMachineStrategy::new();
        assert_eq!(strategy.name(), "state_machine");
    }

    #[test]
    fn test_strategy_priority() {
        let strategy = StateMachineStrategy::new();
        assert_eq!(strategy.priority(), 15);
    }

    #[test]
    fn test_parse_valid_json() {
        let strategy = StateMachineStrategy::new();
        let result = strategy.parse(r#"{"name": "Alice", "age": 30}"#);

        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].value, json!({"name": "Alice", "age": 30}));
    }

    #[test]
    fn test_parse_unclosed_object() {
        let strategy = StateMachineStrategy::new();
        let result = strategy.parse(r#"{"name": "Bob""#);

        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_parse_unclosed_array() {
        let strategy = StateMachineStrategy::new();
        let result = strategy.parse(r#"[1, 2, 3"#);

        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_parse_multiple_objects() {
        let strategy = StateMachineStrategy::new();
        let result = strategy.parse(r#"{"a": 1} {"b": 2}"#);

        assert!(result.is_ok());
        let candidates = result.unwrap();
        // Should parse at least the first object
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_parse_nested_structure() {
        let strategy = StateMachineStrategy::new();
        let result = strategy.parse(r#"{"user": {"name": "Charlie", "items": [1, 2, 3]}}"#);

        assert!(result.is_ok());
        let candidates = result.unwrap();
        assert!(!candidates.is_empty());

        let expected = json!({
            "user": {
                "name": "Charlie",
                "items": [1, 2, 3]
            }
        });
        assert_eq!(candidates[0].value, expected);
    }

    #[test]
    fn test_parse_empty_input() {
        let strategy = StateMachineStrategy::new();
        let result = strategy.parse("");

        assert!(result.is_err());
    }
}
