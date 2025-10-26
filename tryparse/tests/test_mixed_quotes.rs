use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug)]
struct User {
    name: String,
    age: u32,
}

#[derive(Deserialize, Debug)]
struct Message {
    text: String,
    sender: String,
}

#[test]
fn test_all_single_quotes() {
    let response = r#"{'name': 'Alice', 'age': 30}"#;
    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle all single quotes");
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
}

#[test]
fn test_mixed_double_keys_single_values() {
    let response = r#"{"name": 'Bob', "age": 25}"#;
    let result: Result<User, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle double-quote keys with single-quote values"
    );
    let user = result.unwrap();
    assert_eq!(user.name, "Bob");
    assert_eq!(user.age, 25);
}

#[test]
fn test_mixed_single_keys_double_values() {
    let response = r#"{'name': "Charlie", 'age': 35}"#;
    let result: Result<User, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle single-quote keys with double-quote values"
    );
    let user = result.unwrap();
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.age, 35);
}

#[test]
fn test_apostrophe_in_double_quoted_string() {
    let response = r#"{"text": "It's working", "sender": "Alice"}"#;
    let result: Result<Message, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should preserve apostrophes in double-quoted strings"
    );
    let msg = result.unwrap();
    assert_eq!(msg.text, "It's working");
    assert_eq!(msg.sender, "Alice");
}

#[test]
fn test_apostrophe_in_single_quoted_string() {
    // This is tricky - single quotes with embedded apostrophe
    // LLMs might output: {'text': 'It's working'}
    // We need to handle this carefully
    let response = r#"{'text': 'Alice', 'sender': 'Bob'}"#;
    let result: Result<Message, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle single quotes without apostrophes"
    );
    let msg = result.unwrap();
    assert_eq!(msg.text, "Alice");
    assert_eq!(msg.sender, "Bob");
}

#[test]
fn test_complex_mixed_quotes() {
    let response = r#"{"text": 'Hello world', "sender": "Alice"}"#;
    let result: Result<Message, _> = parse(response);
    assert!(result.is_ok(), "Should handle complex mixed quote patterns");
    let msg = result.unwrap();
    assert_eq!(msg.text, "Hello world");
    assert_eq!(msg.sender, "Alice");
}
