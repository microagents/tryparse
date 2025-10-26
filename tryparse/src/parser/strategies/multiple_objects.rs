//! Strategy for collecting multiple top-level JSON objects into an array.
//!
//! This strategy handles inputs like:
//! - `{"key": "value1"} {"key": "value2"}` → `[{"key": "value1"}, {"key": "value2"}]`
//! - `prefix {"a": 1} text {"b": 2} suffix` → `[{"a": 1}, {"b": 2}]`
//!
//! BAML behavior: When expecting Vec<T>, collect all JSON objects from the input.

use serde_json::Value;

use super::ParsingStrategy;
use crate::{
    error::Result,
    value::{FlexValue, Source},
};

#[derive(Debug, Default)]
pub struct MultipleObjectsStrategy;

impl MultipleObjectsStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Finds all balanced JSON objects and arrays in the input.
    fn find_all_json_values(&self, input: &str) -> Vec<String> {
        let mut values = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Skip whitespace
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }

            if i >= chars.len() {
                break;
            }

            // Check for opening brace or bracket
            if chars[i] == '{' || chars[i] == '[' {
                let open = chars[i];
                let close = if open == '{' { '}' } else { ']' };

                // Find matching close
                if let Some(end_idx) = self.find_matching_close(&chars, i, open, close) {
                    let json_str: String = chars[i..=end_idx].iter().collect();

                    // Verify it's valid JSON
                    if serde_json::from_str::<Value>(&json_str).is_ok() {
                        values.push(json_str);
                    }

                    i = end_idx + 1;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        values
    }

    /// Finds the matching closing bracket/brace.
    fn find_matching_close(
        &self,
        chars: &[char],
        start: usize,
        open: char,
        close: char,
    ) -> Option<usize> {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (idx, &ch) in chars.iter().enumerate().skip(start) {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                _ if ch == open && !in_string => depth += 1,
                _ if ch == close && !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(idx);
                    }
                }
                _ => {}
            }
        }

        None
    }
}

impl ParsingStrategy for MultipleObjectsStrategy {
    fn name(&self) -> &'static str {
        "multiple_objects"
    }

    fn parse(&self, input: &str) -> Result<Vec<FlexValue>> {
        let json_values = self.find_all_json_values(input);

        // Only create a candidate if we found 2+ values
        // (1 value should be handled by other strategies)
        if json_values.len() < 2 {
            return Ok(Vec::new());
        }

        // Parse each JSON value
        let mut parsed_values = Vec::new();
        for json_str in &json_values {
            if let Ok(value) = serde_json::from_str::<Value>(json_str) {
                parsed_values.push(value);
            }
        }

        // If we successfully parsed multiple values, wrap them in an array
        if parsed_values.len() >= 2 {
            let array = Value::Array(parsed_values);
            // Use Source::MultiJsonArray to indicate this is a collected array
            // (Don't use Source::Direct as it would prevent other strategies from running)
            let flex = FlexValue::new(array, Source::MultiJsonArray);
            Ok(vec![flex])
        } else {
            Ok(Vec::new())
        }
    }

    fn priority(&self) -> u8 {
        // Highest priority - MUST run before DirectJsonStrategy to detect multiple objects
        // If we don't run first, DirectJsonStrategy will parse only the first object
        // and win the scoring battle (Direct=0 vs MultiJsonArray=40)
        // Priority 0 ensures we run before DirectJson (priority 1)
        0
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_find_two_objects() {
        let strategy = MultipleObjectsStrategy::new();
        let input = r#"{"key": "value1"} {"key": "value2"}"#;

        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].value,
            json!([{"key": "value1"}, {"key": "value2"}])
        );
    }

    #[test]
    fn test_objects_with_surrounding_text() {
        let strategy = MultipleObjectsStrategy::new();
        let input = r#"prefix {"a": 1} some text {"b": 2} suffix"#;

        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, json!([{"a": 1}, {"b": 2}]));
    }

    #[test]
    fn test_single_object_skipped() {
        let strategy = MultipleObjectsStrategy::new();
        let input = r#"{"key": "value"}"#;

        let result = strategy.parse(input).unwrap();

        // Should return empty (let other strategies handle single objects)
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_mixed_objects_and_arrays() {
        let strategy = MultipleObjectsStrategy::new();
        let input = r#"{"a": 1} [1, 2, 3] {"b": 2}"#;

        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, json!([{"a": 1}, [1, 2, 3], {"b": 2}]));
    }

    #[test]
    fn test_nested_objects() {
        let strategy = MultipleObjectsStrategy::new();
        let input = r#"{"outer": {"inner": 1}} {"outer": {"inner": 2}}"#;

        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].value,
            json!([{"outer": {"inner": 1}}, {"outer": {"inner": 2}}])
        );
    }

    #[test]
    fn test_objects_with_strings_containing_braces() {
        let strategy = MultipleObjectsStrategy::new();
        let input = r#"{"text": "has } brace"} {"text": "has { brace"}"#;

        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].value,
            json!([{"text": "has } brace"}, {"text": "has { brace"}])
        );
    }
}
