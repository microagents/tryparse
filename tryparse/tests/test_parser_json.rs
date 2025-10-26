//! Test JSON parsing edge cases

use tryparse::parser::FlexibleParser;

#[test]
fn test_parse_escaped_quotes_in_string() {
    let parser = FlexibleParser::new();
    let input = r#"{"foo": "[\"bar\"]"}"#;

    println!("Input: {}", input);

    let candidates = parser.parse(input);
    println!("Result: {:#?}", candidates);

    assert!(
        candidates.is_ok(),
        "Should parse escaped quotes in JSON string"
    );
    let values = candidates.unwrap();
    assert!(!values.is_empty(), "Should have at least one candidate");

    // Check that the value is correct
    let first = &values[0];
    println!("First candidate value: {:#?}", first.value);

    if let serde_json::Value::Object(obj) = &first.value {
        let foo_value = obj.get("foo").expect("Should have 'foo' field");
        if let serde_json::Value::String(s) = foo_value {
            assert_eq!(s, r#"["bar"]"#, "Value should be the literal string");
        } else {
            panic!("foo should be a string, got: {:?}", foo_value);
        }
    } else {
        panic!("Should be an object, got: {:?}", first.value);
    }
}

#[test]
fn test_parse_nested_json_string() {
    let parser = FlexibleParser::new();
    let input = r#"{"foo": "{\"foo\": [\"bar\"]}"}"#;

    println!("Input: {}", input);

    let candidates = parser.parse(input);
    assert!(candidates.is_ok());
    let values = candidates.unwrap();
    assert!(!values.is_empty());

    if let serde_json::Value::Object(obj) = &values[0].value {
        let foo_value = obj.get("foo").expect("Should have 'foo' field");
        if let serde_json::Value::String(s) = foo_value {
            assert_eq!(s, r#"{"foo": ["bar"]}"#);
        } else {
            panic!("foo should be a string");
        }
    }
}
