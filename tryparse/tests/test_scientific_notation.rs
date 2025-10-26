use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug)]
struct Measurement {
    value: f64,
    precision: f64,
}

#[derive(Deserialize, Debug)]
struct Stats {
    count: u64,
    rate: f64,
}

#[test]
fn test_basic_scientific_notation() {
    let response = r#"{"value": 1e10, "precision": 1.5e-3}"#;
    let result: Result<Measurement, _> = parse(response);
    assert!(result.is_ok(), "Should handle basic scientific notation");
    let m = result.unwrap();
    assert_eq!(m.value, 10000000000.0);
    assert_eq!(m.precision, 0.0015);
}

#[test]
fn test_uppercase_e_notation() {
    let response = r#"{"value": 3.14E+2, "precision": 2.5E-4}"#;
    let result: Result<Measurement, _> = parse(response);
    assert!(result.is_ok(), "Should handle uppercase E notation");
    let m = result.unwrap();
    assert_eq!(m.value, 314.0);
    assert_eq!(m.precision, 0.00025);
}

#[test]
fn test_scientific_with_integers() {
    let response = r#"{"count": 1e6, "rate": 5.5e-2}"#;
    let result: Result<Stats, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle scientific notation with integer fields"
    );
    let s = result.unwrap();
    assert_eq!(s.count, 1000000);
    assert!((s.rate - 0.055).abs() < 1e-10);
}

#[test]
fn test_negative_scientific_notation() {
    let response = r#"{"value": -1.23e4, "precision": -5e-3}"#;
    let result: Result<Measurement, _> = parse(response);
    assert!(result.is_ok(), "Should handle negative scientific notation");
    let m = result.unwrap();
    assert_eq!(m.value, -12300.0);
    assert_eq!(m.precision, -0.005);
}

#[test]
fn test_mixed_notation() {
    let response = r#"{"value": 123.45, "precision": 1e-5}"#;
    let result: Result<Measurement, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle mixed decimal and scientific notation"
    );
    let m = result.unwrap();
    assert_eq!(m.value, 123.45);
    assert_eq!(m.precision, 0.00001);
}
