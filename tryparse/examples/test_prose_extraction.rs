use serde::Deserialize;
use tryparse::parser::FlexibleParser;

#[derive(Deserialize, Debug, PartialEq)]
struct User {
    name: String,
    age: u32,
}

fn main() {
    // Test from brutal_reality.rs
    let response = r#"
    Sure! Here's the user data: {name: 'Alice', age: 30}
    Hope that helps!
    "#;

    println!("Input:\n{}\n", response);

    // First check what candidates the parser finds
    let parser = FlexibleParser::new();
    match parser.parse(response) {
        Ok(candidates) => {
            println!("Found {} candidates:", candidates.len());
            for (i, cand) in candidates.iter().enumerate() {
                println!("\nCandidate {}: source={:?}", i, cand.source);
                println!(
                    "  Value: {}",
                    serde_json::to_string_pretty(&cand.value).unwrap()
                );
            }
        }
        Err(e) => {
            println!("Parser error: {:?}", e);
        }
    }

    // Now try full parse with deserialize
    println!("\n\n=== Full Parse with Deserialize ===");
    use tryparse::parse;
    let result: Result<User, _> = parse(response);
    match &result {
        Ok(user) => {
            println!("✅ SUCCESS! Parsed user: {:?}", user);
            println!("  name: {}", user.name);
            println!("  age: {}", user.age);
        }
        Err(e) => {
            println!("❌ FAILED: {:?}", e);
        }
    }
}
