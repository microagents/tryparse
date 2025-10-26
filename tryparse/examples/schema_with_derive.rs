//! Example showing the derive macro for automatic schema generation.
//!
//! This demonstrates using #[derive(SchemaInfo)] to automatically generate
//! schema information for structs and enums.

use tryparse::schema::{Schema, SchemaInfo};
// Enable the derive feature to test the derive macro
#[cfg(feature = "derive")]
use tryparse_derive::SchemaInfo;

#[cfg(feature = "derive")]
#[derive(SchemaInfo)]
struct User {
    name: String,
    age: u32,
    email: Option<String>,
}

#[cfg(feature = "derive")]
#[derive(SchemaInfo)]
struct Product {
    id: i64,
    name: String,
    price: f64,
    tags: Vec<String>,
}

#[cfg(feature = "derive")]
#[derive(SchemaInfo)]
enum Status {
    Active,
    Pending,
    Completed { result: String },
}

#[cfg(feature = "derive")]
#[derive(SchemaInfo)]
struct TupleStruct(String, i32);

fn main() {
    #[cfg(feature = "derive")]
    {
        println!("=== Schema With Derive Example ===\n");

        // User struct
        println!("1. User Schema:");
        let user_schema = User::schema();
        println!("   Type: {:?}", user_schema);
        println!("   Type name: {}", user_schema.type_name());
        println!();

        // Product struct
        println!("2. Product Schema:");
        let product_schema = Product::schema();
        println!("   Type: {:?}", product_schema);
        println!("   Type name: {}", product_schema.type_name());
        println!();

        // Status enum
        println!("3. Status Enum Schema:");
        let status_schema = Status::schema();
        println!("   Type: {:?}", status_schema);
        println!("   Type name: {}", status_schema.type_name());
        println!();

        // Tuple struct
        println!("4. Tuple Struct Schema:");
        let tuple_schema = TupleStruct::schema();
        println!("   Type: {:?}", tuple_schema);
        println!("   Type name: {}", tuple_schema.type_name());
        println!();

        // Verify structure
        if let Schema::Object { name, fields } = user_schema {
            println!("5. User Schema Details:");
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

        println!("\n✓ Derive macro working correctly!");
    }

    #[cfg(not(feature = "derive"))]
    {
        println!("This example requires the 'derive' feature.");
        println!("Run with: cargo run --example schema_with_derive --features derive");
    }
}
