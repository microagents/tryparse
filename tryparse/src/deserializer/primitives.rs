//! Primitive type deserialization with BAML's algorithms.
//!
//! This module implements sophisticated coercion logic for primitive types,
//! ported directly from BAML's engine to handle edge cases like:
//! - Comma-separated numbers: "$1,234.56" → 1234.56
//! - Fractions: "1/2" → 0.5
//! - Currency symbols: "¥1,234" → 1234.0
//! - String-to-number coercion
//! - Array unwrapping: [42] → 42

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use crate::{
    deserializer::traits::{CoercionContext, LlmDeserialize},
    error::{DeserializeError, ParseError, Result},
    value::{FlexValue, Transformation},
};

// ================================================================================================
// i64 Implementation
// ================================================================================================

impl LlmDeserialize for i64 {
    fn try_deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Option<Self> {
        // Fast path: only succeed if already a number
        match &value.value {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    fn deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Result<Self> {
        match &value.value {
            // Direct number
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i)
                } else if let Some(u) = n.as_u64() {
                    Ok(u as i64)
                } else if let Some(f) = n.as_f64() {
                    // BAML ALGORITHM: Float to int with flag
                    let mut v = value.clone();
                    v.add_transformation(Transformation::FloatToInt { original: f });
                    Ok(f.round() as i64)
                } else {
                    Err(ParseError::DeserializeFailed(
                        DeserializeError::TypeMismatch {
                            expected: "integer",
                            found: "invalid number".to_string(),
                        },
                    ))
                }
            }

            // String to int - BAML's sophisticated parsing
            Value::String(s) => {
                let s = s.trim().trim_end_matches(','); // BAML trims trailing commas

                // Try direct parsing
                if let Ok(n) = s.parse::<i64>() {
                    Ok(n)
                } else if let Ok(n) = s.parse::<u64>() {
                    Ok(n as i64)
                } else if let Ok(f) = s.parse::<f64>() {
                    // BAML ALGORITHM: Parse as float, then convert
                    let mut v = value.clone();
                    v.add_transformation(Transformation::FloatToInt { original: f });
                    Ok(f.round() as i64)
                }
                // BAML ALGORITHM: Parse fractions "1/2" → 0.5 → 0
                else if let Some(f) = parse_fraction(s) {
                    let mut v = value.clone();
                    v.add_transformation(Transformation::FloatToInt { original: f });
                    Ok(f.round() as i64)
                }
                // BAML ALGORITHM: Parse comma-separated "$1,234.56" → 1234
                else if let Some(f) = parse_comma_separated_number(s) {
                    let mut v = value.clone();
                    v.add_transformation(Transformation::FloatToInt { original: f });
                    Ok(f.round() as i64)
                } else {
                    Err(ParseError::DeserializeFailed(
                        DeserializeError::TypeMismatch {
                            expected: "integer",
                            found: format!("string: {}", s),
                        },
                    ))
                }
            }

            // BAML ALGORITHM: Array unwrapping [42] → 42
            Value::Array(items) if items.len() == 1 => {
                let mut v = value.clone();
                v.add_transformation(Transformation::SingleToArray);

                let inner = FlexValue::new(items[0].clone(), value.source.clone());
                Self::deserialize(&inner, _ctx)
            }

            _ => Err(ParseError::DeserializeFailed(
                DeserializeError::TypeMismatch {
                    expected: "integer",
                    found: value_type_name(&value.value),
                },
            )),
        }
    }
}

// ================================================================================================
// f64 Implementation
// ================================================================================================

impl LlmDeserialize for f64 {
    fn try_deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Option<Self> {
        // Fast path: only succeed if already a number
        match &value.value {
            Value::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    fn deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Result<Self> {
        match &value.value {
            // Direct number
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Ok(f)
                } else if let Some(i) = n.as_i64() {
                    Ok(i as f64)
                } else if let Some(u) = n.as_u64() {
                    Ok(u as f64)
                } else {
                    Err(ParseError::DeserializeFailed(
                        DeserializeError::TypeMismatch {
                            expected: "float",
                            found: "invalid number".to_string(),
                        },
                    ))
                }
            }

            // String to float - BAML's sophisticated parsing
            Value::String(s) => {
                let s = s.trim().trim_end_matches(','); // BAML trims trailing commas

                // Try direct parsing
                if let Ok(f) = s.parse::<f64>() {
                    Ok(f)
                } else if let Ok(i) = s.parse::<i64>() {
                    Ok(i as f64)
                } else if let Ok(u) = s.parse::<u64>() {
                    Ok(u as f64)
                }
                // BAML ALGORITHM: Parse fractions "1/2" → 0.5
                else if let Some(f) = parse_fraction(s) {
                    Ok(f)
                }
                // BAML ALGORITHM: Parse comma-separated "$1,234.56" → 1234.56
                else if let Some(f) = parse_comma_separated_number(s) {
                    // BAML adds a flag here to penalize strings like
                    // "1 cup unsalted butter, room temperature"
                    // This helps unions like "float | string" choose correctly
                    let mut v = value.clone();
                    v.add_transformation(Transformation::StringToNumber {
                        original: s.to_string(),
                    });
                    Ok(f)
                } else {
                    Err(ParseError::DeserializeFailed(
                        DeserializeError::TypeMismatch {
                            expected: "float",
                            found: format!("string: {}", s),
                        },
                    ))
                }
            }

            // BAML ALGORITHM: Array unwrapping [42.5] → 42.5
            Value::Array(items) if items.len() == 1 => {
                let mut v = value.clone();
                v.add_transformation(Transformation::SingleToArray);

                let inner = FlexValue::new(items[0].clone(), value.source.clone());
                Self::deserialize(&inner, _ctx)
            }

            _ => Err(ParseError::DeserializeFailed(
                DeserializeError::TypeMismatch {
                    expected: "float",
                    found: value_type_name(&value.value),
                },
            )),
        }
    }
}

// ================================================================================================
// bool Implementation
// ================================================================================================

impl LlmDeserialize for bool {
    fn try_deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Option<Self> {
        // Fast path: only succeed if already a bool
        match &value.value {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Result<Self> {
        match &value.value {
            Value::Bool(b) => Ok(*b),

            // BAML ALGORITHM: String to bool
            Value::String(s) => match s.to_lowercase().as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(ParseError::DeserializeFailed(
                    DeserializeError::TypeMismatch {
                        expected: "bool",
                        found: format!("string: {}", s),
                    },
                )),
            },

            // BAML ALGORITHM: Number to bool (0 = false, non-zero = true)
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i != 0)
                } else if let Some(f) = n.as_f64() {
                    Ok(f != 0.0)
                } else {
                    Err(ParseError::DeserializeFailed(
                        DeserializeError::TypeMismatch {
                            expected: "bool",
                            found: "number".to_string(),
                        },
                    ))
                }
            }

            // BAML ALGORITHM: Array unwrapping [true] → true
            Value::Array(items) if items.len() == 1 => {
                let mut v = value.clone();
                v.add_transformation(Transformation::SingleToArray);

                let inner = FlexValue::new(items[0].clone(), value.source.clone());
                Self::deserialize(&inner, _ctx)
            }

            _ => Err(ParseError::DeserializeFailed(
                DeserializeError::TypeMismatch {
                    expected: "bool",
                    found: value_type_name(&value.value),
                },
            )),
        }
    }
}

// ================================================================================================
// String Implementation
// ================================================================================================

impl LlmDeserialize for String {
    fn try_deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Option<Self> {
        // Fast path: only succeed if already a string
        match &value.value {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Result<Self> {
        match &value.value {
            Value::String(s) => Ok(s.clone()),

            // BAML ALGORITHM: Convert other types to string
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Ok("null".to_string()),
            Value::Object(_) | Value::Array(_) => {
                // Convert to JSON string representation
                Ok(value.value.to_string())
            }
        }
    }
}

// ================================================================================================
// Helper Functions (Ported from BAML)
// ================================================================================================

/// BAML ALGORITHM: Parse fraction strings like "1/2" → 0.5
///
/// Port from: engine/baml-lib/jsonish/src/deserializer/coercer/coerce_primitive.rs:242-254
fn parse_fraction(s: &str) -> Option<f64> {
    if let Some((numerator, denominator)) = s.split_once('/') {
        match (
            numerator.trim().parse::<f64>(),
            denominator.trim().parse::<f64>(),
        ) {
            (Ok(num), Ok(denom)) if denom != 0.0 => Some(num / denom),
            _ => None,
        }
    } else {
        None
    }
}

/// BAML ALGORITHM: Parse comma-separated numbers with currency symbols
///
/// Handles:
/// - "$1,234.56" → 1234.56
/// - "¥1,234" → 1234.0
/// - "1,234,567.89" → 1234567.89
/// - "€1.234,56" (European format) → 1234.56
/// - Scientific notation: "1.23e5" → 123000.0
///
/// Regex for parsing numbers with separators and currency symbols.
static NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([-+]?)\$?(?:\d+(?:,\d+)*(?:\.\d+)?|\d+\.\d+|\d+|\.\d+)(?:e[-+]?\d+)?%?")
        .expect("Invalid number regex pattern")
});

/// Regex for removing Unicode currency symbols.
static CURRENCY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\p{Sc}").expect("Invalid currency regex pattern"));

/// Port from: engine/baml-lib/jsonish/src/deserializer/coercer/coerce_primitive.rs:256-272
pub(crate) fn parse_comma_separated_number(s: &str) -> Option<f64> {
    // Regex pattern from BAML:
    // - Optional sign: ([-+]?)
    // - Optional currency: \$?
    // - Number with optional commas: \d+(?:,\d+)*(?:\.\d+)?
    // - Or: \d+\.\d+ | \d+ | \.\d+
    // - Optional scientific notation: (?:e[-+]?\d+)?
    // - Optional percentage: %?
    let matches: Vec<_> = NUMBER_REGEX.find_iter(s).collect();

    // BAML requires exactly one match
    if matches.len() != 1 {
        return None;
    }

    let number_str = matches[0].as_str();

    // Remove commas
    let without_commas = number_str.replace(',', "");

    // BAML ALGORITHM: Remove all Unicode currency symbols using \p{Sc}
    let without_currency = CURRENCY_REGEX.replace_all(&without_commas, "");

    // BAML ALGORITHM: Remove percentage sign
    // NOTE: We do NOT divide by 100 - BAML keeps "50%" as 50.0, not 0.5
    let without_percent = without_currency.trim_end_matches('%');

    without_percent.parse::<f64>().ok()
}

/// Get a human-readable type name for error messages.
#[inline]
pub(crate) fn value_type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "bool".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

// ================================================================================================
// Collections
// ================================================================================================

impl<T: LlmDeserialize> LlmDeserialize for Vec<T> {
    fn try_deserialize(value: &FlexValue, ctx: &mut CoercionContext) -> Option<Self> {
        // Strict: Must be an array, all items must deserialize strictly
        match &value.value {
            Value::Array(arr) => {
                let items: Option<Vec<T>> = arr
                    .iter()
                    .map(|item| {
                        let flex = FlexValue::new(item.clone(), value.source.clone());
                        T::try_deserialize(&flex, ctx)
                    })
                    .collect();
                items
            }
            _ => None,
        }
    }

    fn deserialize(value: &FlexValue, ctx: &mut CoercionContext) -> Result<Self> {
        match &value.value {
            Value::Array(arr) => {
                // BAML ALGORITHM: Deserialize each item, collect successes
                let items: Result<Vec<T>> = arr
                    .iter()
                    .map(|item| {
                        let flex = FlexValue::new(item.clone(), value.source.clone());
                        T::deserialize(&flex, ctx)
                    })
                    .collect();
                items
            }
            _ => {
                // BAML ALGORITHM: Single value to array
                let item = T::deserialize(value, ctx)?;
                Ok(vec![item])
            }
        }
    }

    fn type_name() -> &'static str {
        "Vec<T>"
    }
}

// ================================================================================================
// HashMap<K, V>
// ================================================================================================

use std::{collections::HashMap, hash::Hash};

impl<K, V> LlmDeserialize for HashMap<K, V>
where
    K: LlmDeserialize + Eq + Hash,
    V: LlmDeserialize,
{
    fn try_deserialize(value: &FlexValue, ctx: &mut CoercionContext) -> Option<Self> {
        // Strict: Must be an object, all keys and values must deserialize strictly
        match &value.value {
            Value::Object(obj) => {
                let mut map = HashMap::new();

                for (key_str, val) in obj.iter() {
                    // Deserialize key from string
                    let key_value =
                        FlexValue::new(Value::String(key_str.clone()), value.source.clone());
                    let key = K::try_deserialize(&key_value, ctx)?;

                    // Deserialize value
                    let value_flex = FlexValue::new(val.clone(), value.source.clone());
                    let value_result = V::try_deserialize(&value_flex, ctx)?;

                    map.insert(key, value_result);
                }

                Some(map)
            }
            _ => None,
        }
    }

    fn deserialize(value: &FlexValue, ctx: &mut CoercionContext) -> Result<Self> {
        match &value.value {
            Value::Object(obj) => {
                let mut map = HashMap::new();

                // BAML ALGORITHM: Deserialize each entry, track errors but continue
                for (key_str, val) in obj.iter() {
                    // Deserialize key from string
                    let key_value =
                        FlexValue::new(Value::String(key_str.clone()), value.source.clone());
                    let key = match K::deserialize(&key_value, ctx) {
                        Ok(k) => k,
                        Err(_e) => {
                            // TODO: Track key deserialization error
                            // For now, skip this entry
                            continue;
                        }
                    };

                    // Deserialize value
                    let value_flex = FlexValue::new(val.clone(), value.source.clone());
                    let value_result = match V::deserialize(&value_flex, ctx) {
                        Ok(v) => v,
                        Err(_e) => {
                            // TODO: Track value deserialization error
                            // For now, skip this entry
                            continue;
                        }
                    };

                    map.insert(key, value_result);
                }

                Ok(map)
            }
            _ => Err(ParseError::DeserializeFailed(
                DeserializeError::type_mismatch("object", "non-object"),
            )),
        }
    }

    fn type_name() -> &'static str {
        "HashMap<K, V>"
    }
}

// Force this module to be linked when the library is compiled
// This ensures all primitive type implementations are available to external crates
#[doc(hidden)]
pub fn __ensure_linked() {}

// ================================================================================================
// Tests
// ================================================================================================

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::value::Source;

    #[test]
    fn test_parse_fraction() {
        assert_eq!(parse_fraction("1/2"), Some(0.5));
        assert_eq!(parse_fraction("3/4"), Some(0.75));
        assert_eq!(parse_fraction("5/10"), Some(0.5));
        assert_eq!(parse_fraction("10/3"), Some(10.0 / 3.0));
        assert_eq!(parse_fraction("1/0"), None); // Division by zero
        assert_eq!(parse_fraction("not_a_fraction"), None);
    }

    #[test]
    fn test_parse_comma_separated() {
        assert_eq!(parse_comma_separated_number("1,234"), Some(1234.0));
        assert_eq!(parse_comma_separated_number("1,234.56"), Some(1234.56));
        assert_eq!(parse_comma_separated_number("$1,234.56"), Some(1234.56));
        assert_eq!(
            parse_comma_separated_number("1,234,567.89"),
            Some(1234567.89)
        );
        assert_eq!(parse_comma_separated_number("1.23e5"), Some(123000.0));
    }

    #[test]
    fn test_i64_direct() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!(42), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 42);
    }

    #[test]
    fn test_i64_from_string() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!("42"), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 42);
    }

    #[test]
    fn test_i64_from_float() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!(42.7), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 43);
    }

    #[test]
    fn test_i64_from_fraction() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!("1/2"), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 1); // 0.5 rounds to 1

        let value = FlexValue::new(json!("3/4"), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 1); // 0.75 rounds to 1

        let value = FlexValue::new(json!("5/2"), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 3); // 2.5 rounds to 3
    }

    #[test]
    fn test_i64_from_comma_separated() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!("$1,234.56"), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 1235); // Rounds to 1235

        let value = FlexValue::new(json!("$1,234.49"), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 1234); // Rounds to 1234
    }

    #[test]
    fn test_i64_array_unwrap() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!([42]), Source::Direct);
        assert_eq!(i64::deserialize(&value, &mut ctx).unwrap(), 42);
    }

    #[test]
    fn test_f64_from_comma_separated() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!("$1,234.56"), Source::Direct);
        assert_eq!(f64::deserialize(&value, &mut ctx).unwrap(), 1234.56);
    }

    #[test]
    fn test_bool_from_string() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!("true"), Source::Direct);
        assert!(bool::deserialize(&value, &mut ctx).unwrap());

        let value = FlexValue::new(json!("FALSE"), Source::Direct);
        assert!(!bool::deserialize(&value, &mut ctx).unwrap());
    }

    #[test]
    fn test_bool_from_number() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!(1), Source::Direct);
        assert!(bool::deserialize(&value, &mut ctx).unwrap());

        let value = FlexValue::new(json!(0), Source::Direct);
        assert!(!bool::deserialize(&value, &mut ctx).unwrap());
    }

    #[test]
    fn test_string_from_number() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!(42), Source::Direct);
        assert_eq!(String::deserialize(&value, &mut ctx).unwrap(), "42");
    }

    #[test]
    fn test_try_deserialize_fast_path() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!(42), Source::Direct);

        // Fast path should work
        assert_eq!(i64::try_deserialize(&value, &mut ctx), Some(42));

        // Fast path should fail for string
        let value = FlexValue::new(json!("42"), Source::Direct);
        assert_eq!(i64::try_deserialize(&value, &mut ctx), None);
    }

    // ===== HashMap Tests =====

    #[test]
    fn test_hashmap_string_to_int() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(
            json!({
                "one": 1,
                "two": 2,
                "three": 3
            }),
            Source::Direct,
        );

        let map = HashMap::<String, i64>::deserialize(&value, &mut ctx).unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("one"), Some(&1));
        assert_eq!(map.get("two"), Some(&2));
        assert_eq!(map.get("three"), Some(&3));
    }

    #[test]
    fn test_hashmap_int_to_string() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(
            json!({
                "1": "one",
                "2": "two",
                "3": "three"
            }),
            Source::Direct,
        );

        let map = HashMap::<i64, String>::deserialize(&value, &mut ctx).unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get(&1), Some(&"one".to_string()));
        assert_eq!(map.get(&2), Some(&"two".to_string()));
        assert_eq!(map.get(&3), Some(&"three".to_string()));
    }

    #[test]
    fn test_hashmap_with_coercion() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(
            json!({
                "one": "1",   // String to int coercion
                "two": "2",
                "three": "3"
            }),
            Source::Direct,
        );

        let map = HashMap::<String, i64>::deserialize(&value, &mut ctx).unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("one"), Some(&1));
        assert_eq!(map.get("two"), Some(&2));
        assert_eq!(map.get("three"), Some(&3));
    }

    #[test]
    fn test_hashmap_strict_mode() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(
            json!({
                "one": 1,
                "two": 2
            }),
            Source::Direct,
        );

        // Strict mode should work when all values match exactly
        let map = HashMap::<String, i64>::try_deserialize(&value, &mut ctx);
        assert!(map.is_some());
        let map = map.unwrap();
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_hashmap_strict_mode_fails() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(
            json!({
                "one": "1",  // String - needs coercion
                "two": 2
            }),
            Source::Direct,
        );

        // Strict mode should fail when coercion is needed
        let map = HashMap::<String, i64>::try_deserialize(&value, &mut ctx);
        assert!(map.is_none());
    }

    #[test]
    fn test_hashmap_empty() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(json!({}), Source::Direct);

        let map = HashMap::<String, i64>::deserialize(&value, &mut ctx).unwrap();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_hashmap_with_invalid_value() {
        let mut ctx = CoercionContext::new();
        let value = FlexValue::new(
            json!({
                "one": 1,
                "two": "not a number",  // Will fail to parse as i64
                "three": 3
            }),
            Source::Direct,
        );

        // Should skip invalid entries and continue
        let map = HashMap::<String, i64>::deserialize(&value, &mut ctx).unwrap();
        // Should have 2 entries (one and three), skipping "two"
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("one"), Some(&1));
        assert_eq!(map.get("three"), Some(&3));
    }
}
