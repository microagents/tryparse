//! Union deserialization with BAML's scoring algorithm.
//!
//! Ported from `engine/baml-lib/jsonish/src/deserializer/coercer/coerce_union.rs`
//! and `array_helper.rs`.

use crate::{
    deserializer::traits::{CoercionContext, LlmDeserialize},
    error::{DeserializeError, ParseError, Result},
    value::{FlexValue, Transformation},
};

/// Represents a successful union variant match with its score.
#[derive(Debug, Clone)]
pub struct UnionMatch<T> {
    /// The deserialized value
    pub value: T,
    /// Score (lower is better)
    pub score: u32,
    /// Transformations applied
    pub transformations: Vec<Transformation>,
}

/// Helper to try multiple union variants and pick the best match.
///
/// Port from `coerce_union.rs:8-94`.
pub struct UnionDeserializer<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> UnionDeserializer<T> {
    /// Creates a new union deserializer.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Try to deserialize into multiple variants and return all successful matches.
    ///
    /// This is used when you have a union type and want to see which variants match.
    pub fn try_all<V1, V2>(
        &self,
        value: &FlexValue,
        ctx: &mut CoercionContext,
    ) -> Vec<UnionMatch<T>>
    where
        V1: LlmDeserialize + Into<T>,
        V2: LlmDeserialize + Into<T>,
    {
        let mut matches = Vec::new();

        // Try first variant (strict mode)
        let mut ctx1 = ctx.clone();
        if let Some(v1) = V1::try_deserialize(value, &mut ctx1) {
            matches.push(UnionMatch {
                value: v1.into(),
                score: 0, // Strict match has best score
                transformations: ctx1.transformations().to_vec(),
            });
        }

        // Try second variant (strict mode)
        let mut ctx2 = ctx.clone();
        if let Some(v2) = V2::try_deserialize(value, &mut ctx2) {
            matches.push(UnionMatch {
                value: v2.into(),
                score: 0,
                transformations: ctx2.transformations().to_vec(),
            });
        }

        // If no strict matches, try lenient mode
        if matches.is_empty() {
            // Try with separate contexts to track transformations for each variant
            let mut ctx1 = ctx.clone();
            if let Ok(v1) = V1::deserialize(value, &mut ctx1) {
                let score = calculate_score_from_context(&ctx1);
                matches.push(UnionMatch {
                    value: v1.into(),
                    score,
                    transformations: ctx1.transformations().to_vec(),
                });
            }

            let mut ctx2 = ctx.clone();
            if let Ok(v2) = V2::deserialize(value, &mut ctx2) {
                let score = calculate_score_from_context(&ctx2);
                matches.push(UnionMatch {
                    value: v2.into(),
                    score,
                    transformations: ctx2.transformations().to_vec(),
                });
            }
        }

        matches
    }

    /// Deserialize into the best matching variant.
    ///
    /// Port from `coerce_union.rs:69-94` (coerce method).
    pub fn deserialize<V1, V2>(&self, value: &FlexValue, ctx: &mut CoercionContext) -> Result<T>
    where
        V1: LlmDeserialize + Into<T>,
        V2: LlmDeserialize + Into<T>,
    {
        let mut matches = self.try_all::<V1, V2>(value, ctx);

        if matches.is_empty() {
            return Err(ParseError::DeserializeFailed(DeserializeError::Custom(
                "No union variant matched".to_string(),
            )));
        }

        if matches.len() == 1 {
            let winning_match = matches.remove(0);

            // Add UnionMatch transformation to context
            let union_transformation = Transformation::UnionMatch {
                index: 0, // First (and only) variant
                candidates: vec![
                    std::any::type_name::<V1>().to_string(),
                    std::any::type_name::<V2>().to_string(),
                ],
            };
            ctx.add_transformation(union_transformation);

            // Copy all transformations from the winning variant to the context
            for transformation in winning_match.transformations {
                ctx.add_transformation(transformation);
            }

            return Ok(winning_match.value);
        }

        // Multiple matches - pick the best using BAML's heuristics
        matches.sort_by(|a, b| {
            // First, compare by score (lower is better)
            match a.score.cmp(&b.score) {
                std::cmp::Ordering::Equal => {
                    // If scores are equal, apply union-specific heuristics
                    apply_union_heuristics(a, b)
                }
                ordering => ordering,
            }
        });

        let winning_match = matches.remove(0);

        // Add UnionMatch transformation to context
        let union_transformation = Transformation::UnionMatch {
            index: 0, // Winner is always at index 0 after sorting
            candidates: vec![
                std::any::type_name::<V1>().to_string(),
                std::any::type_name::<V2>().to_string(),
            ],
        };
        ctx.add_transformation(union_transformation);

        // Copy all transformations from the winning variant to the context
        for transformation in winning_match.transformations {
            ctx.add_transformation(transformation);
        }

        Ok(winning_match.value)
    }
}

impl<T> Default for UnionDeserializer<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate score from transformations in a CoercionContext.
///
/// This sums up the penalty scores of all transformations.
fn calculate_score_from_context(ctx: &CoercionContext) -> u32 {
    ctx.transformations().iter().map(|t| t.penalty()).sum()
}

/// Apply BAML's union-specific heuristics to pick the best match.
///
/// Port from `array_helper.rs:66-243` with all 8 heuristics.
///
/// Heuristics (in priority order):
/// 1. For lists: Prefer real arrays over single-to-array transformations
/// 2. For lists: Prefer lists without markdown-wrapped objects
/// 3. For lists: Prefer lists with fewer parse errors
/// 4. For unions with structs: De-value single-field structs with ImpliedKey
/// 5. Prefer structs with non-default values over all-default structs
/// 6. Prefer composite types over strings cast from objects (JsonToString)
/// 7. Sort by score (handled in caller)
/// 8. Prefer non-default values
fn apply_union_heuristics<T>(a: &UnionMatch<T>, b: &UnionMatch<T>) -> std::cmp::Ordering {
    // Both are lists - apply list-specific heuristics
    let a_is_list = is_list_transformation(&a.transformations);
    let b_is_list = is_list_transformation(&b.transformations);

    if a_is_list && b_is_list {
        // HEURISTIC 1: Prefer real arrays over single-to-array
        let a_has_single_to_array = a
            .transformations
            .iter()
            .any(|t| matches!(t, Transformation::SingleToArray));
        let b_has_single_to_array = b
            .transformations
            .iter()
            .any(|t| matches!(t, Transformation::SingleToArray));

        match (a_has_single_to_array, b_has_single_to_array) {
            (true, false) => return std::cmp::Ordering::Greater, // Prefer B
            (false, true) => return std::cmp::Ordering::Less,    // Prefer A
            _ => {}
        }

        // HEURISTIC 2: Prefer lists without markdown strings
        // Check for ObjectFromMarkdown in the transformations
        let a_has_markdown = a
            .transformations
            .iter()
            .any(|t| matches!(t, Transformation::ObjectFromMarkdown { .. }));
        let b_has_markdown = b
            .transformations
            .iter()
            .any(|t| matches!(t, Transformation::ObjectFromMarkdown { .. }));

        match (a_has_markdown, b_has_markdown) {
            (true, false) => return std::cmp::Ordering::Greater, // Prefer B
            (false, true) => return std::cmp::Ordering::Less,    // Prefer A
            _ => {}
        }

        // HEURISTIC 3: Prefer lists with fewer parse errors
        let a_error_count = count_array_errors(&a.transformations);
        let b_error_count = count_array_errors(&b.transformations);

        match a_error_count.cmp(&b_error_count) {
            std::cmp::Ordering::Equal => {}
            ordering => return ordering,
        }
    }

    // HEURISTIC 4: De-value single-field structs with ImpliedKey in unions
    // This catches cases like: union string | struct with single string field
    let a_is_implied_single = a
        .transformations
        .iter()
        .any(|t| matches!(t, Transformation::ImpliedKey { .. }));
    let b_is_implied_single = b
        .transformations
        .iter()
        .any(|t| matches!(t, Transformation::ImpliedKey { .. }));

    match (a_is_implied_single, b_is_implied_single) {
        (true, false) => return std::cmp::Ordering::Greater, // Prefer B
        (false, true) => return std::cmp::Ordering::Less,    // Prefer A
        _ => {}
    }

    // HEURISTIC 5: Prefer structs with non-default values
    let a_is_all_defaults = is_all_defaults(&a.transformations);
    let b_is_all_defaults = is_all_defaults(&b.transformations);

    match (a_is_all_defaults, b_is_all_defaults) {
        (true, false) => return std::cmp::Ordering::Greater, // Prefer B
        (false, true) => return std::cmp::Ordering::Less,    // Prefer A
        _ => {}
    }

    // HEURISTIC 6: Devalue strings that were cast from objects
    // Prefer composite types over strings converted from JSON
    let a_is_json_to_string = a
        .transformations
        .iter()
        .any(|t| matches!(t, Transformation::JsonToString { .. }));
    let b_is_json_to_string = b
        .transformations
        .iter()
        .any(|t| matches!(t, Transformation::JsonToString { .. }));

    match (a_is_json_to_string, b_is_json_to_string) {
        (true, false) => return std::cmp::Ordering::Greater, // Prefer B
        (false, true) => return std::cmp::Ordering::Less,    // Prefer A
        _ => {}
    }

    // Default: equal (scores will be compared by caller)
    std::cmp::Ordering::Equal
}

/// Checks if transformations indicate this is a list type.
fn is_list_transformation(transformations: &[Transformation]) -> bool {
    transformations.iter().any(|t| {
        matches!(
            t,
            Transformation::SingleToArray | Transformation::ArrayItemParseError { .. }
        )
    })
}

/// Counts the number of array item parse errors.
fn count_array_errors(transformations: &[Transformation]) -> usize {
    transformations
        .iter()
        .filter(|t| matches!(t, Transformation::ArrayItemParseError { .. }))
        .count()
}

/// Checks if all values were defaults.
fn is_all_defaults(transformations: &[Transformation]) -> bool {
    // If there's at least one DefaultValueInserted and no other substantial transformations
    let has_defaults = transformations
        .iter()
        .any(|t| matches!(t, Transformation::DefaultValueInserted { .. }));

    let has_real_values = transformations.iter().any(|t| {
        matches!(
            t,
            Transformation::StringToNumber { .. }
                | Transformation::FloatToInt { .. }
                | Transformation::FieldNameCaseChanged { .. }
        )
    });

    has_defaults && !has_real_values
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::value::Source;

    #[derive(Debug, Clone, PartialEq)]
    struct StringVariant(String);

    impl LlmDeserialize for StringVariant {
        fn try_deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Option<Self> {
            match &value.value {
                serde_json::Value::String(s) => Some(StringVariant(s.clone())),
                _ => None,
            }
        }

        fn deserialize(value: &FlexValue, ctx: &mut CoercionContext) -> Result<Self> {
            Self::try_deserialize(value, ctx).ok_or_else(|| {
                ParseError::DeserializeFailed(DeserializeError::type_mismatch(
                    "string",
                    "non-string",
                ))
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct IntVariant(i64);

    impl LlmDeserialize for IntVariant {
        fn try_deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Option<Self> {
            match &value.value {
                serde_json::Value::Number(n) => n.as_i64().map(IntVariant),
                _ => None,
            }
        }

        fn deserialize(value: &FlexValue, _ctx: &mut CoercionContext) -> Result<Self> {
            match &value.value {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(IntVariant(i))
                    } else {
                        Err(ParseError::DeserializeFailed(
                            DeserializeError::type_mismatch("integer", "non-integer number"),
                        ))
                    }
                }
                serde_json::Value::String(s) => s.parse::<i64>().map(IntVariant).map_err(|_| {
                    ParseError::DeserializeFailed(DeserializeError::type_mismatch(
                        "integer",
                        "unparseable string",
                    ))
                }),
                _ => Err(ParseError::DeserializeFailed(
                    DeserializeError::type_mismatch("integer", "non-numeric"),
                )),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    enum StringOrInt {
        String(StringVariant),
        Int(IntVariant),
    }

    impl From<StringVariant> for StringOrInt {
        fn from(v: StringVariant) -> Self {
            StringOrInt::String(v)
        }
    }

    impl From<IntVariant> for StringOrInt {
        fn from(v: IntVariant) -> Self {
            StringOrInt::Int(v)
        }
    }

    #[test]
    fn test_union_string_match() {
        let json = json!("hello");
        let value = FlexValue::new(json, Source::Direct);
        let mut ctx = CoercionContext::new();

        let deserializer = UnionDeserializer::<StringOrInt>::new();
        let result = deserializer.deserialize::<StringVariant, IntVariant>(&value, &mut ctx);

        assert!(result.is_ok());
        let union = result.unwrap();
        assert!(matches!(union, StringOrInt::String(_)));
    }

    #[test]
    fn test_union_int_match() {
        let json = json!(42);
        let value = FlexValue::new(json, Source::Direct);
        let mut ctx = CoercionContext::new();

        let deserializer = UnionDeserializer::<StringOrInt>::new();
        let result = deserializer.deserialize::<StringVariant, IntVariant>(&value, &mut ctx);

        assert!(result.is_ok());
        let union = result.unwrap();
        assert!(matches!(union, StringOrInt::Int(_)));
    }

    #[test]
    fn test_union_string_coercion_to_int() {
        // String "42" should match as String (strict match wins)
        let json = json!("42");
        let value = FlexValue::new(json, Source::Direct);
        let mut ctx = CoercionContext::new();

        let deserializer = UnionDeserializer::<StringOrInt>::new();
        let result = deserializer.deserialize::<StringVariant, IntVariant>(&value, &mut ctx);

        assert!(result.is_ok());
        // String should win because it's a strict match (score 0)
        // Int would require coercion (score > 0)
        let union = result.unwrap();
        assert!(matches!(union, StringOrInt::String(_)));
    }

    #[test]
    fn test_union_no_match() {
        let json = json!(null);
        let value = FlexValue::new(json, Source::Direct);
        let mut ctx = CoercionContext::new();

        let deserializer = UnionDeserializer::<StringOrInt>::new();
        let result = deserializer.deserialize::<StringVariant, IntVariant>(&value, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_score() {
        use crate::scoring::score_candidate;

        let mut value = FlexValue::new(json!("42"), Source::Direct);
        assert_eq!(score_candidate(&value), 0);

        value.add_transformation(Transformation::StringToNumber {
            original: "42".to_string(),
        });
        assert!(score_candidate(&value) > 0);
    }

    #[test]
    fn test_is_all_defaults() {
        let transformations = vec![Transformation::DefaultValueInserted {
            field: "age".to_string(),
        }];
        assert!(is_all_defaults(&transformations));

        let transformations2 = vec![
            Transformation::DefaultValueInserted {
                field: "age".to_string(),
            },
            Transformation::StringToNumber {
                original: "42".to_string(),
            },
        ];
        assert!(!is_all_defaults(&transformations2));
    }
}
