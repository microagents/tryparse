//! Direct JSON parsing strategy.

use super::ParsingStrategy;
use crate::{
    error::Result,
    value::{FlexValue, Source},
};

/// Strategy that attempts to parse input directly as JSON.
///
/// This is the fastest strategy and should always be tried first.
/// It succeeds only if the entire input (after trimming whitespace)
/// is valid JSON.
///
/// # Examples
///
/// ```
/// use tryparse::parser::strategies::{ParsingStrategy, DirectJsonStrategy};
///
/// let strategy = DirectJsonStrategy;
/// let result = strategy.parse(r#"{"name": "Alice"}"#).unwrap();
/// assert_eq!(result.len(), 1);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct DirectJsonStrategy;

impl ParsingStrategy for DirectJsonStrategy {
    #[inline]
    fn name(&self) -> &'static str {
        "direct_json"
    }

    fn parse(&self, input: &str) -> Result<Vec<FlexValue>> {
        let trimmed = input.trim();

        // Fast path: check if it looks like JSON
        if !looks_like_json(trimmed) {
            return Ok(Vec::new());
        }

        match serde_json::from_str(trimmed) {
            Ok(value) => Ok(vec![FlexValue::new(value, Source::Direct)]),
            Err(_) => Ok(Vec::new()), // Not an error, just not applicable
        }
    }

    #[inline]
    fn priority(&self) -> u8 {
        1 // Highest priority (try first)
    }
}

/// Fast check to see if a string looks like JSON.
///
/// This is a heuristic to avoid expensive parsing attempts.
#[inline]
fn looks_like_json(s: &str) -> bool {
    let first = s.chars().next();
    matches!(first, Some('{') | Some('[') | Some('"'))
        || s.starts_with("true")
        || s.starts_with("false")
        || s.starts_with("null")
        || s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_parse_valid_json_object() {
        let strategy = DirectJsonStrategy;
        let result = strategy.parse(r#"{"name": "Alice", "age": 30}"#).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, json!({"name": "Alice", "age": 30}));
    }

    #[test]
    fn test_parse_valid_json_array() {
        let strategy = DirectJsonStrategy;
        let result = strategy.parse(r#"[1, 2, 3]"#).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, json!([1, 2, 3]));
    }

    #[test]
    fn test_parse_with_whitespace() {
        let strategy = DirectJsonStrategy;
        let result = strategy.parse("  \n  {\"test\": true}  \n  ").unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, json!({"test": true}));
    }

    #[test]
    fn test_parse_invalid_json() {
        let strategy = DirectJsonStrategy;
        let result = strategy.parse("{invalid json}").unwrap();

        assert_eq!(result.len(), 0); // Not applicable
    }

    #[test]
    fn test_parse_not_json() {
        let strategy = DirectJsonStrategy;
        let result = strategy.parse("This is just text").unwrap();

        assert_eq!(result.len(), 0); // Not applicable
    }

    #[test]
    fn test_looks_like_json() {
        assert!(looks_like_json("{"));
        assert!(looks_like_json("["));
        assert!(looks_like_json("\""));
        assert!(looks_like_json("true"));
        assert!(looks_like_json("false"));
        assert!(looks_like_json("null"));
        assert!(looks_like_json("123"));
        assert!(looks_like_json("-42"));

        assert!(!looks_like_json("text"));
        assert!(!looks_like_json(""));
    }

    #[test]
    fn test_parse_primitives() {
        let strategy = DirectJsonStrategy;

        let result = strategy.parse("true").unwrap();
        assert_eq!(result[0].value, json!(true));

        let result = strategy.parse("42").unwrap();
        assert_eq!(result[0].value, json!(42));

        let result = strategy.parse(r#""hello""#).unwrap();
        assert_eq!(result[0].value, json!("hello"));

        let result = strategy.parse("null").unwrap();
        assert_eq!(result[0].value, json!(null));
    }

    #[test]
    fn test_parse_escaped_quotes_in_string_value() {
        let strategy = DirectJsonStrategy;

        // Test that we can parse JSON with escaped quotes in string values
        let result = strategy.parse(r#"{"foo": "[\"bar\"]"}"#).unwrap();

        assert_eq!(
            result.len(),
            1,
            "Should successfully parse JSON with escaped quotes"
        );

        let obj = result[0].value.as_object().expect("Should be an object");
        let foo_value = obj.get("foo").expect("Should have 'foo' field");

        assert_eq!(
            foo_value,
            &json!(r#"["bar"]"#),
            "Should preserve escaped content as string"
        );
    }

    #[test]
    fn test_parse_nested_json_string() {
        let strategy = DirectJsonStrategy;

        let result = strategy
            .parse(r#"{"foo": "{\"foo\": [\"bar\"]}"}"#)
            .unwrap();

        assert!(!result.is_empty(), "Should parse nested JSON string");
    }
}
