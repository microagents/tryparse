//! YAML parsing strategy that converts YAML to JSON.

use crate::{
    error::Result,
    parser::strategies::ParsingStrategy,
    value::{FlexValue, Source},
};

/// Strategy that parses YAML content and converts it to JSON.
///
/// This strategy:
/// 1. Detects YAML-like content (key: value patterns)
/// 2. Attempts to parse as YAML using serde_yaml
/// 3. Converts the parsed YAML to JSON via serde_json::Value
///
/// # Examples
///
/// ```
/// use tryparse::parser::strategies::{ParsingStrategy, YamlStrategy};
///
/// let strategy = YamlStrategy::default();
/// let input = "name: Alice\nage: 30";
/// let candidates = strategy.parse(input).unwrap();
/// assert!(!candidates.is_empty());
/// ```
#[derive(Debug, Clone, Default)]
pub struct YamlStrategy;

impl YamlStrategy {
    /// Creates a new YAML strategy.
    pub fn new() -> Self {
        Self
    }

    /// Checks if the input looks like YAML.
    ///
    /// Returns true if the input contains YAML-like patterns:
    /// - key: value on separate lines
    /// - proper indentation
    /// - no JSON-like braces at the start
    fn looks_like_yaml(input: &str) -> bool {
        let trimmed = input.trim();

        // Don't treat JSON as YAML
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return false;
        }

        // Look for YAML patterns: key: value
        // Count lines that match "key: value" pattern
        let yaml_pattern_count = trimmed
            .lines()
            .filter(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return false;
                }
                // Check for "key: value" pattern
                line.contains(':') && !line.starts_with('-')
            })
            .count();

        // If we have at least 2 lines with key: value, likely YAML
        yaml_pattern_count >= 2
    }
}

impl ParsingStrategy for YamlStrategy {
    fn name(&self) -> &'static str {
        "yaml"
    }

    fn parse(&self, input: &str) -> Result<Vec<FlexValue>> {
        // Quick check if it looks like YAML
        if !Self::looks_like_yaml(input) {
            return Ok(Vec::new());
        }

        // Try to parse as YAML
        match serde_yaml::from_str::<serde_yaml::Value>(input) {
            Ok(yaml_value) => {
                // Convert YAML Value to JSON Value
                match serde_json::to_value(&yaml_value) {
                    Ok(json_value) => {
                        let flex_value = FlexValue::new(json_value, Source::Yaml);
                        Ok(vec![flex_value])
                    }
                    Err(_) => Ok(Vec::new()),
                }
            }
            Err(_) => {
                // Not valid YAML
                Ok(Vec::new())
            }
        }
    }

    fn priority(&self) -> u8 {
        // Run after direct JSON (1) but before heuristics (5)
        // YAML is fairly structured, so we can try it early
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_yaml() {
        // Valid YAML patterns
        assert!(YamlStrategy::looks_like_yaml("name: Alice\nage: 30"));
        assert!(YamlStrategy::looks_like_yaml(
            "user:\n  name: Bob\n  age: 25"
        ));
        assert!(YamlStrategy::looks_like_yaml(
            "# Comment\nname: Charlie\nage: 35"
        ));

        // Not YAML
        assert!(!YamlStrategy::looks_like_yaml("{\"name\": \"Alice\"}"));
        assert!(!YamlStrategy::looks_like_yaml("[1, 2, 3]"));
        assert!(!YamlStrategy::looks_like_yaml("Just plain text"));
        assert!(!YamlStrategy::looks_like_yaml("name: Alice")); // Only 1 line
    }

    #[test]
    fn test_parse_simple_yaml() {
        let strategy = YamlStrategy::new();
        let input = "name: Alice\nage: 30";
        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].source, Source::Yaml));

        let obj = result[0].value.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap().as_str().unwrap(), "Alice");
        assert_eq!(obj.get("age").unwrap().as_u64().unwrap(), 30);
    }

    #[test]
    fn test_parse_nested_yaml() {
        let strategy = YamlStrategy::new();
        let input = "user:\n  name: Bob\n  age: 25";
        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);

        let obj = result[0].value.as_object().unwrap();
        let user = obj.get("user").unwrap().as_object().unwrap();
        assert_eq!(user.get("name").unwrap().as_str().unwrap(), "Bob");
        assert_eq!(user.get("age").unwrap().as_u64().unwrap(), 25);
    }

    #[test]
    fn test_parse_yaml_with_array() {
        let strategy = YamlStrategy::new();
        let input = "names:\n  - Alice\n  - Bob\ncount: 2";
        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);

        let obj = result[0].value.as_object().unwrap();
        let names = obj.get("names").unwrap().as_array().unwrap();
        assert_eq!(names.len(), 2);
        assert_eq!(names[0].as_str().unwrap(), "Alice");
        assert_eq!(names[1].as_str().unwrap(), "Bob");
    }

    #[test]
    fn test_parse_json_not_yaml() {
        let strategy = YamlStrategy::new();
        let input = r#"{"name": "Alice", "age": 30}"#;
        let result = strategy.parse(input).unwrap();

        // Should return empty, not applicable
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let strategy = YamlStrategy::new();
        let input = "name: Alice\n  invalid indentation\nage: 30";
        let _result = strategy.parse(input).unwrap();

        // May or may not parse depending on serde_yaml's tolerance
        // Either way, should not panic
    }

    #[test]
    fn test_yaml_with_comments() {
        let strategy = YamlStrategy::new();
        let input = "# User data\nname: Alice # Full name\nage: 30";
        let result = strategy.parse(input).unwrap();

        assert_eq!(result.len(), 1);
        let obj = result[0].value.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap().as_str().unwrap(), "Alice");
    }

    #[test]
    fn test_strategy_name() {
        let strategy = YamlStrategy::new();
        assert_eq!(strategy.name(), "yaml");
    }

    #[test]
    fn test_strategy_priority() {
        let strategy = YamlStrategy::new();
        assert_eq!(strategy.priority(), 3);
    }
}
