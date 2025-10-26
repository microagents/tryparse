//! Scoring system for ranking parsing candidates.

use crate::value::{FlexValue, Source};

/// Multiplier applied per depth level for recursive scoring.
/// Transformations at depth N are penalized by (DEPTH_SCORE_MULTIPLIER ^ N).
pub const DEPTH_SCORE_MULTIPLIER: u32 = 10;

/// Scores a candidate based on how it was parsed and transformed.
///
/// Lower scores are better. The score is calculated from:
/// - Base score from the source type
/// - Transformation penalties (with recursive multiplier)
/// - Inverse of confidence (1.0 - confidence)
///
/// **Recursive Scoring**: Transformations that occurred at deeper nesting levels
/// are penalized more heavily. This matches BAML's algorithm where nested
/// values multiply scores by 10 per level.
///
/// # Examples
///
/// ```
/// use tryparse::scoring::score_candidate;
/// use tryparse::value::{FlexValue, Source};
/// use serde_json::json;
///
/// let value = FlexValue::new(json!({"test": 1}), Source::Direct);
/// let score = score_candidate(&value);
/// assert_eq!(score, 0); // Direct source with no transformations
/// ```
#[inline]
pub fn score_candidate(candidate: &FlexValue) -> u32 {
    score_candidate_recursive(candidate, false)
}

/// Scores a candidate with optional recursive scoring.
///
/// When `use_recursive` is true, applies BAML's recursive scoring algorithm
/// where transformations are multiplied by 10^depth.
pub fn score_candidate_recursive(candidate: &FlexValue, use_recursive: bool) -> u32 {
    let mut score = source_base_score(&candidate.source);

    // Add transformation penalties
    let transformation_score: u32 = candidate
        .transformations()
        .iter()
        .map(|t| t.penalty())
        .sum();

    if use_recursive && candidate.max_transformation_depth() > 0 {
        // Apply recursive multiplier: DEPTH_SCORE_MULTIPLIER per depth level
        // This matches BAML's scoring where nested transformations are penalized heavily
        let multiplier = DEPTH_SCORE_MULTIPLIER.pow(candidate.max_transformation_depth() as u32);
        score += transformation_score * multiplier;
    } else {
        score += transformation_score;
    }

    // Add confidence penalty (inverse of confidence, scaled)
    // Confidence is 0.0-1.0, we want lower confidence to increase score
    let confidence_penalty = ((1.0 - candidate.confidence()) * 100.0) as u32;
    score += confidence_penalty;

    score
}

/// Returns the base score for a source type.
///
/// Lower is better. Direct JSON gets the lowest score.
#[inline(always)]
fn source_base_score(source: &Source) -> u32 {
    match source {
        Source::Direct => 0,
        Source::Markdown { .. } => 10,
        Source::Yaml => 15,
        Source::Fixed { fixes } => {
            // Sum the penalty of each fix type (different fixes have different reliability)
            20 + fixes.iter().map(|f| f.penalty()).sum::<u32>()
        }
        // MultiJsonArray has lower score than MultiJson because:
        // - For Vec<T>: MultiJsonArray is perfect (no wrapping needed), MultiJson needs SingleToArray
        // - For T: Both work, but MultiJson picks first which is common pattern
        Source::MultiJsonArray => 25, // Lower score = higher priority
        Source::MultiJson { .. } => 30,
        Source::Heuristic { .. } => 50,
    }
}

/// Ranks candidates by score and returns them sorted (best first).
///
/// The input vector is consumed and returned sorted by score.
/// Lower scores appear first.
///
/// # Examples
///
/// ```
/// use tryparse::scoring::rank_candidates;
/// use tryparse::value::{FlexValue, Source};
/// use serde_json::json;
///
/// let candidates = vec![
///     FlexValue::from_fixed_json(json!(1), vec![]),
///     FlexValue::new(json!(2), Source::Direct),
/// ];
///
/// let ranked = rank_candidates(candidates);
/// assert_eq!(ranked[0].value, json!(2)); // Direct source wins
/// ```
pub fn rank_candidates(mut candidates: Vec<FlexValue>) -> Vec<FlexValue> {
    candidates.sort_by_cached_key(score_candidate);
    candidates
}

/// Returns the best candidate from a list.
///
/// Returns `None` if the list is empty.
///
/// # Examples
///
/// ```
/// use tryparse::scoring::best_candidate;
/// use tryparse::value::{FlexValue, Source};
/// use serde_json::json;
///
/// let candidates = vec![
///     FlexValue::from_fixed_json(json!(1), vec![]),
///     FlexValue::new(json!(2), Source::Direct),
/// ];
///
/// let best = best_candidate(candidates).unwrap();
/// assert_eq!(best.value, json!(2));
/// ```
pub fn best_candidate(candidates: Vec<FlexValue>) -> Option<FlexValue> {
    if candidates.is_empty() {
        return None;
    }

    let ranked = rank_candidates(candidates);
    ranked.into_iter().next()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::value::{JsonFix, Transformation};

    #[test]
    fn test_score_direct_source() {
        let value = FlexValue::new(json!(1), Source::Direct);
        assert_eq!(score_candidate(&value), 0);
    }

    #[test]
    fn test_score_markdown_source() {
        let value = FlexValue::new(json!(1), Source::Markdown { lang: None });
        assert_eq!(score_candidate(&value), 10);
    }

    #[test]
    fn test_score_fixed_source() {
        let value = FlexValue::from_fixed_json(
            json!(1),
            vec![JsonFix::TrailingCommas, JsonFix::SingleQuotes],
        );
        // Base: 20
        // TrailingCommas penalty: 1
        // SingleQuotes penalty: 2
        // Confidence penalty: (1.0 - 0.9) * 100 = 10
        // Total: 20 + 1 + 2 + 10 = 33
        assert_eq!(score_candidate(&value), 33);
    }

    #[test]
    fn test_score_with_transformations() {
        let mut value = FlexValue::new(json!(42), Source::Direct);
        value.add_transformation(Transformation::StringToNumber {
            original: "42".to_string(),
        });
        // Base: 0, transformation: 2, confidence penalty: (1.0 - 0.95) * 100 = 5
        // Total: 0 + 2 + 5 = 7
        assert_eq!(score_candidate(&value), 7);
    }

    #[test]
    fn test_rank_candidates() {
        let candidates = vec![
            FlexValue::from_fixed_json(json!(1), vec![JsonFix::TrailingCommas]),
            FlexValue::new(json!(2), Source::Direct),
            FlexValue::new(json!(3), Source::Markdown { lang: None }),
        ];

        let ranked = rank_candidates(candidates);

        // Direct should be first
        assert_eq!(ranked[0].value, json!(2));
        // Markdown should be second
        assert_eq!(ranked[1].value, json!(3));
        // Fixed should be last
        assert_eq!(ranked[2].value, json!(1));
    }

    #[test]
    fn test_best_candidate() {
        let candidates = vec![
            FlexValue::from_fixed_json(json!(1), vec![]),
            FlexValue::new(json!(2), Source::Direct),
            FlexValue::new(json!(3), Source::Markdown { lang: None }),
        ];

        let best = best_candidate(candidates).unwrap();
        assert_eq!(best.value, json!(2));
    }

    #[test]
    fn test_best_candidate_empty() {
        let candidates = vec![];
        let best = best_candidate(candidates);
        assert!(best.is_none());
    }

    #[test]
    fn test_source_base_score() {
        assert_eq!(source_base_score(&Source::Direct), 0);
        assert_eq!(source_base_score(&Source::Markdown { lang: None }), 10);
        assert_eq!(
            source_base_score(&Source::Fixed {
                fixes: vec![JsonFix::TrailingCommas] // penalty 1
            }),
            21 // 20 + 1
        );
        assert_eq!(source_base_score(&Source::MultiJsonArray), 25);
        assert_eq!(source_base_score(&Source::MultiJson { index: 0 }), 30);
        assert_eq!(
            source_base_score(&Source::Heuristic {
                pattern: "test".to_string()
            }),
            50
        );
    }

    #[test]
    fn test_multiple_transformations() {
        let mut value = FlexValue::new(json!(42), Source::Direct);
        value.add_transformation(Transformation::StringToNumber {
            original: "42".to_string(),
        });
        value.add_transformation(Transformation::SingleToArray);

        // Base: 0
        // Transform 1 penalty: 2
        // Transform 2 penalty: 5
        // Confidence penalty: (1.0 - 0.95 * 0.95) * 100 ≈ 10
        let score = score_candidate(&value);
        assert!((7..=20).contains(&score)); // Approximate range due to confidence
    }

    #[test]
    fn test_rank_preserves_all_candidates() {
        let candidates = vec![
            FlexValue::new(json!(1), Source::Direct),
            FlexValue::new(json!(2), Source::Direct),
            FlexValue::new(json!(3), Source::Direct),
        ];

        let count = candidates.len();
        let ranked = rank_candidates(candidates);

        assert_eq!(ranked.len(), count);
    }
}
