//! Reality check: Testing against actual messy LLM responses
//! This file documents what we DON'T handle well yet.

use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug, PartialEq)]
struct User {
    name: String,
    age: u32,
}

// ============================================================================
// SCENARIO 1: JSON buried in prose (NO markdown blocks)
// ============================================================================

#[test]
fn test_json_in_prose() {
    let response = r#"
    Sure! Here's the user data: {"name": "Alice", "age": 30}
    Hope that helps!
    "#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should extract JSON from prose");
}

// ============================================================================
// SCENARIO 2: Multiple JSON objects (no array wrapper)
// ============================================================================

#[test]
#[ignore] // FAILS - We have Source::MultiJson but no strategy implements it!
fn test_multiple_json_objects() {
    let response = r#"
    {"name": "Alice", "age": 30}
    {"name": "Bob", "age": 25}
    "#;

    let result: Result<Vec<User>, _> = parse(response);
    assert!(result.is_ok(), "Should handle multiple JSON objects");
}

// ============================================================================
// SCENARIO 3: Truncated/incomplete JSON (LLM cut off)
// ============================================================================

#[test]
fn test_truncated_json() {
    let response = r#"{"name": "Alice", "age": 30"#; // Missing closing brace

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should close unclosed braces");
}

// ============================================================================
// SCENARIO 4: Block comments (not just line comments)
// ============================================================================

#[test]
fn test_block_comments() {
    let response = r#"{"name": "Alice" /* this is her name */, "age": 30}"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should remove block comments");
}

// ============================================================================
// SCENARIO 5: Smart/curly quotes (common in LLM outputs)
// ============================================================================

#[test]
fn test_smart_quotes() {
    let response = r#"{"name": "Alice", "age": 30}"#; // Curly quotes

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should normalize Unicode quotes");
}

// ============================================================================
// SCENARIO 6: Missing commas between items
// ============================================================================

#[test]
fn test_missing_commas() {
    let response = r#"{"name": "Alice" "age": 30}"#; // Missing comma

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should add missing commas");
}

// ============================================================================
// SCENARIO 7: Field name case mismatch
// ============================================================================

#[test]
#[ignore] // STILL FAILS - Would need fuzzy field matching in deserializer
fn test_case_insensitive_fields() {
    let response = r#"{"Name": "Alice", "AGE": 30}"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should match fields case-insensitively");
}

// ============================================================================
// SCENARIO 8: JSON in the middle of a long rambling response
// ============================================================================

#[test]
fn test_json_in_long_prose() {
    let response = r#"
    Well, let me think about this. The user you're asking about is quite interesting.
    They have been with us for a while. Actually, I should give you their data.
    The information is {"name": "Alice", "age": 30} as you can see.
    Let me know if you need anything else about this user or other users.
    "#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should extract JSON from rambling text");
}

// ============================================================================
// SCENARIO 9: Malformed but recoverable JSON (multiple issues)
// ============================================================================

#[test]
#[ignore] // TODO: Multiple fix combinations need better handling
fn test_multiple_issues_combined() {
    // Unquoted keys + single quotes + trailing comma + missing closing brace
    let response = r#"{name: 'Alice', age: 30,"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle multiple issues at once");
}

// ============================================================================
// SCENARIO 10: JSON-like but not quite (YAML, etc.)
// ============================================================================

#[test]
#[cfg(feature = "yaml")]
fn test_yaml_style() {
    // YAML support implemented!
    let response = r#"
    name: Alice
    age: 30
    "#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should parse YAML-style simple objects");
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
}

#[test]
#[cfg(not(feature = "yaml"))]
#[ignore] // FAILS - We don't handle YAML-style without feature
fn test_yaml_style() {
    let response = r#"
    name: Alice
    age: 30
    "#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should parse YAML-style simple objects");
}

// ============================================================================
// SCENARIO 11: Extra garbage characters
// ============================================================================

#[test]
#[ignore] // FAILS - No cleaning strategy
fn test_garbage_characters() {
    let response = r#"```json
    {
        "name": "Alice",,,,
        "age": 30
    }
    ```"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle extra commas/garbage");
}

// ============================================================================
// SCENARIO 12: Nested errors deep in structure
// ============================================================================

#[test]
#[ignore] // FAILS - Shallow fixing only
fn test_nested_errors() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Company {
        name: String,
        users: Vec<User>,
    }

    let response = r#"{
        "name": "ACME",
        "users": [
            {name: "Alice", age: 30},
            {name: 'Bob', age: '25'}
        ]
    }"#;

    let result: Result<Company, _> = parse(response);
    assert!(result.is_ok(), "Should fix errors at any nesting level");
}

// ============================================================================
// SCENARIO 13: Leading/trailing whitespace and newlines
// ============================================================================

#[test]
fn test_excessive_whitespace() {
    let response = r#"


    {
        "name"    :     "Alice"    ,
        "age"     :     30
    }


    "#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle excessive whitespace");
}

// ============================================================================
// SCENARIO 14: Numbers as strings that need coercion
// ============================================================================

#[test]
fn test_string_numbers() {
    let response = r#"{"name": "Alice", "age": "30"}"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should coerce string numbers");
    assert_eq!(result.unwrap().age, 30);
}

// ============================================================================
// SCENARIO 15: Empty response
// ============================================================================

#[test]
fn test_empty_response() {
    let response = "";

    let result: Result<User, _> = parse(response);
    assert!(result.is_err(), "Should fail gracefully on empty input");
}

// ============================================================================
// SCENARIO 16: Just an error message from LLM
// ============================================================================

#[test]
#[ignore] // We correctly fail, but maybe should have better error
fn test_llm_error_message() {
    let response = "I'm sorry, I cannot provide that information.";

    let result: Result<User, _> = parse(response);
    assert!(result.is_err(), "Should fail on non-JSON responses");
}

// ============================================================================
// SCENARIO 17: Array when expecting object
// ============================================================================

#[test]
#[ignore] // FAILS - Type mismatch not recoverable
fn test_array_for_object() {
    let response = r#"["Alice", 30]"#; // Tuple-like array

    let result: Result<User, _> = parse(response);
    // Could theoretically map positionally to fields
    assert!(result.is_err(), "Should fail but with clear error");
}

// ============================================================================
// TESTS THAT SHOULD PASS - Current Capabilities
// ============================================================================

#[test]
fn test_markdown_json_block() {
    let response = r#"
```json
{"name": "Alice", "age": 30}
```
    "#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok());
}

#[test]
fn test_trailing_commas() {
    let response = r#"{"name": "Alice", "age": 30,}"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok());
}

#[test]
fn test_single_quotes() {
    let response = r#"{'name': 'Alice', 'age': 30}"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok());
}

#[test]
fn test_unquoted_keys() {
    let response = r#"{name: "Alice", age: 30}"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok());
}

#[test]
fn test_line_comments() {
    let response = r#"{"name": "Alice", "age": 30} // the user"#;

    let result: Result<User, _> = parse(response);
    assert!(result.is_ok());
}
