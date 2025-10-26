//! Tests for percentage parsing.
//!
//! BAML algorithm: Parse percentages like "50%" as 50.0 (NOT divided by 100).

use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug, PartialEq)]
struct Stats {
    success_rate: f64,
    growth: f64,
}

#[test]
fn test_parse_percentage_basic() {
    let response = r#"{"success_rate": "50%", "growth": "3.15%"}"#;
    let result: Result<Stats, _> = parse(response);
    if let Err(e) = &result {
        eprintln!("Error: {:?}", e);
    }
    assert!(result.is_ok(), "Should parse percentages");
    let stats = result.unwrap();

    eprintln!("Stats: {:?}", stats);
    // BAML keeps percentages as-is (NOT divided by 100)
    assert_eq!(stats.success_rate, 50.0);
    assert_eq!(stats.growth, 3.15);
}

#[test]
fn test_parse_percentage_decimal() {
    let response = r#"{"success_rate": "0.009%", "growth": "100%"}"#;
    let result: Result<Stats, _> = parse(response);
    assert!(result.is_ok());
    let stats = result.unwrap();

    assert_eq!(stats.success_rate, 0.009);
    assert_eq!(stats.growth, 100.0);
}

#[test]
fn test_parse_percentage_to_integer() {
    #[derive(Deserialize, Debug)]
    struct IntStats {
        completion: i64,
    }

    let response = r#"{"completion": "85%"}"#;
    let result: Result<IntStats, _> = parse(response);
    assert!(result.is_ok(), "Should parse percentage to integer");
    let stats = result.unwrap();

    // Percentage parsed as 85.0, then converted to i64
    assert_eq!(stats.completion, 85);
}

#[test]
fn test_parse_percentage_with_sign() {
    let response = r#"{"success_rate": "+50%", "growth": "-3.5%"}"#;
    let result: Result<Stats, _> = parse(response);
    assert!(result.is_ok());
    let stats = result.unwrap();

    assert_eq!(stats.success_rate, 50.0);
    assert_eq!(stats.growth, -3.5);
}

#[test]
fn test_percentage_without_sign_still_works() {
    let response = r#"{"success_rate": "50", "growth": "3.15"}"#;
    let result: Result<Stats, _> = parse(response);
    assert!(result.is_ok());
    let stats = result.unwrap();

    assert_eq!(stats.success_rate, 50.0);
    assert_eq!(stats.growth, 3.15);
}

#[test]
fn test_percentage_mixed_format() {
    let response = r#"{"success_rate": "50%", "growth": 3.15}"#;
    let result: Result<Stats, _> = parse(response);
    assert!(result.is_ok());
    let stats = result.unwrap();

    assert_eq!(stats.success_rate, 50.0);
    assert_eq!(stats.growth, 3.15);
}
