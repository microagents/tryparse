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
fn test_backticks_wrapping_json() {
    // Backticks around the entire JSON (like markdown without the language tag)
    let response = r#"`{"name": "Alice", "age": 30}`"#;
    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Should handle backticks wrapping JSON");
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
}

#[test]
fn test_backticks_as_string_delimiters() {
    // Backticks used instead of quotes (JavaScript-style)
    let response = r#"{`name`: `Bob`, `age`: 25}"#;
    let result: Result<User, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle backticks as string delimiters"
    );
    let user = result.unwrap();
    assert_eq!(user.name, "Bob");
    assert_eq!(user.age, 25);
}

#[test]
fn test_mixed_backticks_and_quotes() {
    let response = r#"{"name": `Charlie`, "age": 35}"#;
    let result: Result<User, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle mixed backticks and double quotes"
    );
    let user = result.unwrap();
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.age, 35);
}

#[test]
fn test_backticks_with_complex_content() {
    let response = r#"{`text`: `Hello world`, `sender`: `Alice`}"#;
    let result: Result<Message, _> = parse(response);
    assert!(result.is_ok(), "Should handle backticks with text content");
    let msg = result.unwrap();
    assert_eq!(msg.text, "Hello world");
    assert_eq!(msg.sender, "Alice");
}
