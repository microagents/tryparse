//! Test JSON fixer strategy
//!
//! JsonFixerStrategy only returns candidates when it successfully applies fixes.
//! For already-valid JSON, it returns an empty list (no fixes needed).
//! Field name normalization has been disabled to preserve HashMap keys.

use tryparse::parser::strategies::{JsonFixerStrategy, ParsingStrategy};

#[test]
fn test_json_fixer_with_valid_json() {
    let fixer = JsonFixerStrategy::default();
    let input = r#"{"userName": "Alice", "maxCount": 30}"#;

    println!("Testing with input: {}", input);

    let candidates = fixer.parse(input).unwrap();
    println!("Found {} candidates", candidates.len());

    // JsonFixerStrategy only returns candidates when fixes are applied
    // For valid JSON with no syntax errors, it returns an empty list
    // This is expected behavior - no fixes needed = no candidates from fixer
    assert!(
        candidates.is_empty(),
        "JsonFixerStrategy should return no candidates for valid JSON (no fixes needed)"
    );
}

#[test]
fn test_json_fixer_with_broken_json() {
    let fixer = JsonFixerStrategy::default();
    // JSON with trailing comma (needs fixing)
    let input = r#"{"userName": "Alice", "maxCount": 30,}"#;

    let candidates = fixer.parse(input).unwrap();

    // Should find candidates because trailing comma needs to be fixed
    assert!(
        !candidates.is_empty(),
        "Should find candidates for broken JSON"
    );
    println!("Found {} candidates for broken JSON", candidates.len());
}
