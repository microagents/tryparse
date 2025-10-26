//! Comprehensive example of schema-aware parsing.
//!
//! This example demonstrates the full power of tryparse with schema information:
//! - Automatic schema derivation
//! - Field name normalization
//! - Type coercion
//! - Robust parsing with state machine

use serde::Deserialize;
use tryparse::parse_with_schema;
#[cfg(feature = "derive")]
use tryparse::schema::SchemaInfo;
#[cfg(feature = "derive")]
use tryparse_derive::SchemaInfo;

#[cfg(feature = "derive")]
#[derive(Debug, Deserialize, SchemaInfo, PartialEq)]
struct User {
    user_name: String,
    age: u32,
    email: Option<String>,
    tags: Vec<String>,
}

#[cfg(feature = "derive")]
#[derive(Debug, Deserialize, SchemaInfo, PartialEq)]
struct Product {
    product_id: i64,
    product_name: String,
    price: f64,
    in_stock: bool,
}

#[cfg(feature = "derive")]
#[derive(Debug, Deserialize, SchemaInfo)]
enum Status {
    Active,
    Pending,
    Completed { message: String },
}

fn main() {
    #[cfg(feature = "derive")]
    {
        println!("=== Schema-Aware Parsing Example ===\n");

        // Example 1: Field name normalization (camelCase → snake_case)
        println!("1. Field Name Normalization:");
        let llm_response = r#"{"userName": "Alice", "age": 30, "email": "alice@example.com", "tags": ["rust", "ai"]}"#;

        match parse_with_schema::<User>(llm_response) {
            Ok(user) => {
                println!("   ✓ Parsed user: {:?}", user);
                assert_eq!(user.user_name, "Alice");
                assert_eq!(user.age, 30);
            }
            Err(e) => println!("   ✗ Error: {:?}", e),
        }
        println!();

        // Example 2: Type coercion (string → number)
        println!("2. Type Coercion:");
        let llm_response = r#"{"userName": "Bob", "age": "25", "tags": []}"#;

        match parse_with_schema::<User>(llm_response) {
            Ok(user) => {
                println!("   ✓ Parsed user: {:?}", user);
                assert_eq!(user.age, 25); // String "25" coerced to u32
            }
            Err(e) => println!("   ✗ Error: {:?}", e),
        }
        println!();

        // Example 3: Messy LLM output with markdown
        println!("3. Messy LLM Output (Markdown):");
        let llm_response = r#"
Here's the user data:
```json
{
  "userName": "Charlie",
  "age": 35,
  "email": "charlie@example.com",
  "tags": ["senior", "mentor"]
}
```
"#;

        match parse_with_schema::<User>(llm_response) {
            Ok(user) => {
                println!("   ✓ Parsed user: {:?}", user);
                assert_eq!(user.user_name, "Charlie");
            }
            Err(e) => println!("   ✗ Error: {:?}", e),
        }
        println!();

        // Example 4: Malformed JSON (trailing comma, unquoted keys)
        println!("4. Malformed JSON (Auto-Fix):");
        let llm_response = r#"{userName: "Dave", age: 40, tags: ["lead"],}"#;

        match parse_with_schema::<User>(llm_response) {
            Ok(user) => {
                println!("   ✓ Parsed user: {:?}", user);
                assert_eq!(user.user_name, "Dave");
            }
            Err(e) => println!("   ✗ Error: {:?}", e),
        }
        println!();

        // Example 5: Product with different naming conventions
        println!("5. Product Parsing:");
        let llm_response =
            r#"{"productId": 12345, "productName": "Widget", "price": 29.99, "inStock": true}"#;

        match parse_with_schema::<Product>(llm_response) {
            Ok(product) => {
                println!("   ✓ Parsed product: {:?}", product);
                assert_eq!(product.product_id, 12345);
                assert_eq!(product.product_name, "Widget");
            }
            Err(e) => println!("   ✗ Error: {:?}", e),
        }
        println!();

        // Example 6: Unclosed JSON (state machine auto-closes)
        println!("6. Unclosed JSON (Auto-Close):");
        let llm_response = r#"{"userName": "Eve", "age": 28, "tags": ["dev""#;

        match parse_with_schema::<User>(llm_response) {
            Ok(user) => {
                println!("   ✓ Parsed user: {:?}", user);
                assert_eq!(user.user_name, "Eve");
            }
            Err(e) => println!("   ✗ Error: {:?}", e),
        }
        println!();

        // Example 7: Schema inspection
        println!("7. Schema Inspection:");
        let user_schema = User::schema();
        println!("   User schema type: {}", user_schema.type_name());
        if let tryparse::schema::Schema::Object { name, fields } = user_schema {
            println!("   Object name: {}", name);
            println!("   Fields:");
            for field in fields {
                println!(
                    "     - {}: {} (required: {})",
                    field.name,
                    field.schema.type_name(),
                    field.required
                );
            }
        }
        println!();

        println!("✓ All examples completed successfully!");
    }

    #[cfg(not(feature = "derive"))]
    {
        println!("This example requires the 'derive' feature.");
        println!("Run with: cargo run --example schema_aware_parsing --features derive");
    }
}

#[cfg(test)]
#[cfg(feature = "derive")]
mod tests {
    use super::*;

    #[test]
    fn test_field_name_normalization() {
        let response = r#"{"userName": "Alice", "age": 30, "tags": []}"#;
        let user: User = parse_with_schema(response).unwrap();
        assert_eq!(user.user_name, "Alice");
    }

    #[test]
    fn test_type_coercion() {
        let response = r#"{"userName": "Bob", "age": "25", "tags": []}"#;
        let user: User = parse_with_schema(response).unwrap();
        assert_eq!(user.age, 25);
    }

    #[test]
    fn test_messy_output() {
        let response = r#"
        Here's the data:
        ```json
        {"userName": "Charlie", "age": 35, "tags": ["senior"]}
        ```
        "#;
        let user: User = parse_with_schema(response).unwrap();
        assert_eq!(user.user_name, "Charlie");
    }

    #[test]
    fn test_malformed_json() {
        let response = r#"{userName: "Dave", age: 40, tags: []}"#;
        let user: User = parse_with_schema(response).unwrap();
        assert_eq!(user.user_name, "Dave");
    }
}
