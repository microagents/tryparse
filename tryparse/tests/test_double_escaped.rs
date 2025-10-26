use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug)]
struct User {
    name: String,
    age: u32,
}

#[derive(Deserialize, Debug)]
struct Config {
    enabled: bool,
    timeout: u32,
}

#[test]
fn test_double_escaped_object() {
    // LLM accidentally serialized JSON as a string
    let response = r#""{\"name\": \"Alice\", \"age\": 30}""#;
    println!("Input: {}", response);

    // First check what DirectJSON sees
    match serde_json::from_str::<serde_json::Value>(response) {
        Ok(val) => println!("DirectJSON parsed as: {:?}", val),
        Err(e) => println!("DirectJSON failed: {}", e),
    }

    let result: Result<User, _> = parse(response);
    match &result {
        Ok(user) => println!("Successfully parsed: {:?}", user),
        Err(e) => println!("Parse failed: {:?}", e),
    }
    assert!(
        result.is_ok(),
        "Should handle double-escaped JSON object: {:?}",
        result.err()
    );
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
}

#[test]
fn test_double_escaped_with_whitespace() {
    let response = r#"  "{\"name\": \"Bob\", \"age\": 25}"  "#;
    let result: Result<User, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle double-escaped JSON with surrounding whitespace"
    );
    let user = result.unwrap();
    assert_eq!(user.name, "Bob");
    assert_eq!(user.age, 25);
}

#[test]
fn test_double_escaped_boolean() {
    let response = r#""{\"enabled\": true, \"timeout\": 100}""#;
    let result: Result<Config, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle double-escaped JSON with booleans"
    );
    let config = result.unwrap();
    assert!(config.enabled);
    assert_eq!(config.timeout, 100);
}

#[test]
fn test_double_escaped_array() {
    #[derive(Deserialize, Debug)]
    struct Users {
        users: Vec<String>,
    }

    let response = r#""{\"users\": [\"Alice\", \"Bob\"]}""#;
    let result: Result<Users, _> = parse(response);
    assert!(
        result.is_ok(),
        "Should handle double-escaped JSON with arrays"
    );
    let data = result.unwrap();
    assert_eq!(data.users.len(), 2);
    assert_eq!(data.users[0], "Alice");
    assert_eq!(data.users[1], "Bob");
}

#[test]
fn test_unnecessary_backslashes() {
    // JSON with unnecessary backslashes (not actually double-escaped)
    let response = r#"{\"name\": \"Alice\", \"age\": 30}"#;
    println!("Testing: {}", response);
    let result: Result<User, _> = parse(response);
    match &result {
        Ok(u) => println!("Success: {:?}", u),
        Err(e) => println!("Failed: {:?}", e),
    }
    // This should either work or we need a fixer for unnecessary backslashes
    // For now, let's see what happens
    assert!(
        result.is_ok(),
        "Should handle unnecessary backslashes: {:?}",
        result.err()
    );
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
}

#[test]
fn test_not_double_escaped() {
    // Regular JSON should still work
    let response = r#"{"name": "Charlie", "age": 35}"#;
    let result: Result<User, _> = parse(response);
    assert!(result.is_ok(), "Regular JSON should still work");
    let user = result.unwrap();
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.age, 35);
}
