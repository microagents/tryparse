//! Test extraction + fixing combination

use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize, Debug, PartialEq)]
struct User {
    name: String,
    age: u32,
}

#[test]
fn test_extraction_plus_fixing_single_quotes() {
    // JSON with single quotes embedded in prose
    // Should: 1) Extract {name: 'Alice', age: 30}
    //        2) Fix single quotes to double quotes
    //        3) Parse successfully
    let response = r#"
    Here's the user data: {name: 'Alice', age: 30}
    "#;

    // First check what candidates are found
    let parser = tryparse::parser::FlexibleParser::new();
    let candidates = parser.parse(response).unwrap();
    println!("Found {} candidates:", candidates.len());
    for (i, candidate) in candidates.iter().enumerate() {
        println!(
            "  Candidate {}: {:?} = {}",
            i, candidate.source, candidate.value
        );
    }

    let result: Result<User, _> = parse(response);
    println!("\nResult: {:#?}", result);

    assert!(result.is_ok(), "Should extract and fix single quotes");
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
}

#[test]
fn test_extraction_plus_fixing_unquoted_keys() {
    // JSON with unquoted keys in prose
    let response = r#"
    The data is: {"name": "Bob", age: 25}
    "#;

    let result: Result<User, _> = parse(response);
    println!("Result: {:#?}", result);

    assert!(result.is_ok(), "Should extract and fix unquoted keys");
    let user = result.unwrap();
    assert_eq!(user.name, "Bob");
    assert_eq!(user.age, 25);
}

#[test]
fn test_extraction_plus_fixing_both_issues() {
    // JSON with multiple issues: unquoted keys + single quotes
    let response = r#"
    Sure! Here's the user data: {name: 'Alice', age: 30}
    Hope that helps!
    "#;

    let result: Result<User, _> = parse(response);
    println!("Result: {:#?}", result);

    assert!(result.is_ok(), "Should extract and fix multiple issues");
    let user = result.unwrap();
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
}

#[test]
fn test_extraction_plus_fixing_trailing_comma() {
    // JSON with trailing comma in prose
    let response = r#"
    Data: {"name": "Charlie", "age": 35,}
    "#;

    let result: Result<User, _> = parse(response);
    println!("Result: {:#?}", result);

    assert!(result.is_ok(), "Should extract and fix trailing comma");
    let user = result.unwrap();
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.age, 35);
}
