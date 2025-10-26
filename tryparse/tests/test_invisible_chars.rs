use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug)]
struct User {
    name: String,
    age: u32,
}

#[test]
fn test_zero_width_space() {
    // Zero-width space (U+200B) in the JSON
    let response = "{\"name\u{200B}\": \"Alice\", \"age\": 30}";
    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle zero-width spaces");
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
}

#[test]
fn test_bom_at_start() {
    // BOM (Byte Order Mark) at the beginning
    let response = "\u{FEFF}{\"name\": \"Bob\", \"age\": 25}";
    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle BOM at start");
    let user = result.unwrap();
    assert_eq!(user.name, "Bob");
    assert_eq!(user.age, 25);
}

#[test]
fn test_zero_width_non_joiner() {
    // Zero-width non-joiner (U+200C)
    let response = "{\"na\u{200C}me\": \"Charlie\", \"age\": 35}";
    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle zero-width non-joiner");
    let user = result.unwrap();
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.age, 35);
}

#[test]
fn test_multiple_invisible_chars() {
    // Multiple invisible characters
    let response = "\u{FEFF}{\"name\u{200B}\": \"Dave\u{200C}\", \"age\u{200D}\": 40}";
    let result: Result<User, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle multiple invisible characters"
    );
    let user = result.unwrap();
    assert_eq!(user.name, "Dave");
    assert_eq!(user.age, 40);
}

#[test]
fn test_rtl_marks() {
    // Right-to-left and left-to-right marks
    let response = "{\"name\u{200E}\": \"Eve\u{200F}\", \"age\": 45}";
    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle RTL/LTR marks");
    let user = result.unwrap();
    assert_eq!(user.name, "Eve");
    assert_eq!(user.age, 45);
}

#[test]
fn test_clean_json_unaffected() {
    // Clean JSON should still work
    let response = r#"{"name": "Frank", "age": 50}"#;
    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Clean JSON should still work");
    let user = result.unwrap();
    assert_eq!(user.name, "Frank");
    assert_eq!(user.age, 50);
}
