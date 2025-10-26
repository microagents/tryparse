//! Basic example showing the schema system.
//!
//! This demonstrates how to use the SchemaInfo trait to get compile-time
//! schema information from types.

use std::collections::HashMap;

use tryparse::schema::{Field, Schema, SchemaInfo};

fn main() {
    println!("=== Schema Basics Example ===\n");

    // Primitive types
    println!("1. Primitive Schemas:");
    println!("   String schema: {:?}", String::schema());
    println!("   i32 schema: {:?}", i32::schema());
    println!("   bool schema: {:?}", bool::schema());
    println!();

    // Composite types
    println!("2. Composite Schemas:");

    // Vec<String>
    let vec_schema = Vec::<String>::schema();
    println!("   Vec<String> schema: {:?}", vec_schema);
    println!("   Type name: {}", vec_schema.type_name());
    println!();

    // Option<i32>
    let option_schema = Option::<i32>::schema();
    println!("   Option<i32> schema: {:?}", option_schema);
    println!("   Type name: {}", option_schema.type_name());
    println!();

    // HashMap<String, i32>
    let map_schema = HashMap::<String, i32>::schema();
    println!("   HashMap<String, i32> schema: {:?}", map_schema);
    println!("   Type name: {}", map_schema.type_name());
    println!();

    // Nested types
    println!("3. Nested Schemas:");
    let nested = Vec::<Option<String>>::schema();
    println!("   Vec<Option<String>> schema: {:?}", nested);
    println!("   Type name: {}", nested.type_name());
    println!();

    // Tuple types
    println!("4. Tuple Schemas:");
    let tuple_schema = <(String, i32, bool)>::schema();
    println!("   (String, i32, bool) schema: {:?}", tuple_schema);
    println!("   Type name: {}", tuple_schema.type_name());
    println!();

    // Manual schema construction (for testing without derive macro)
    println!("5. Manual Schema Construction:");
    let user_schema = Schema::Object {
        name: "User".to_string(),
        fields: vec![
            Field::new("name", Schema::String),
            Field::new("age", Schema::Int),
            Field::new("email", Schema::String).optional(),
        ],
    };
    println!("   User schema: {:?}", user_schema);
    println!("   Type name: {}", user_schema.type_name());
    println!();

    // Schema introspection
    println!("6. Schema Properties:");
    println!("   Is String primitive? {}", Schema::String.is_primitive());
    println!(
        "   Is Vec<i32> primitive? {}",
        Schema::Array(Box::new(Schema::Int)).is_primitive()
    );
    println!(
        "   Is Vec<i32> composite? {}",
        Schema::Array(Box::new(Schema::Int)).is_composite()
    );
    println!();

    // Field with aliases
    println!("7. Field Aliases (for name normalization):");
    let field = Field::new("user_name", Schema::String)
        .with_alias("userName")
        .with_alias("UserName");

    println!("   Field: {}", field.name);
    println!(
        "   Matches 'user_name': {}",
        field.matches_name("user_name")
    );
    println!("   Matches 'userName': {}", field.matches_name("userName"));
    println!("   Matches 'UserName': {}", field.matches_name("UserName"));
    println!("   Matches 'name': {}", field.matches_name("name"));
    println!();

    println!("✓ Schema system working correctly!");
}
