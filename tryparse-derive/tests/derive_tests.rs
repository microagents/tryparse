//! Integration tests for the SchemaInfo derive macro.

use tryparse::schema::{Schema, SchemaInfo};
use tryparse_derive::SchemaInfo;

// ============================================================================
// Struct Tests
// ============================================================================

#[derive(SchemaInfo)]
struct SimpleStruct {
    name: String,
    age: u32,
}

#[test]
fn test_simple_struct() {
    let schema = SimpleStruct::schema();

    match schema {
        Schema::Object { name, fields } => {
            assert_eq!(name, "SimpleStruct");
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].name, "name");
            assert_eq!(fields[0].schema, Schema::String);
            assert!(fields[0].required);

            assert_eq!(fields[1].name, "age");
            assert_eq!(fields[1].schema, Schema::Int);
            assert!(fields[1].required);
        }
        _ => panic!("Expected Object schema"),
    }
}

#[derive(SchemaInfo)]
struct StructWithOptional {
    required: String,
    optional: Option<i32>,
}

#[test]
fn test_struct_with_optional() {
    let schema = StructWithOptional::schema();

    match schema {
        Schema::Object { name, fields } => {
            assert_eq!(name, "StructWithOptional");
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].name, "required");
            assert_eq!(fields[0].schema, Schema::String);

            assert_eq!(fields[1].name, "optional");
            match &fields[1].schema {
                Schema::Optional(inner) => {
                    assert_eq!(**inner, Schema::Int);
                }
                _ => panic!("Expected Optional schema"),
            }
        }
        _ => panic!("Expected Object schema"),
    }
}

#[derive(SchemaInfo)]
struct StructWithVec {
    items: Vec<String>,
    numbers: Vec<i32>,
}

#[test]
fn test_struct_with_vec() {
    let schema = StructWithVec::schema();

    match schema {
        Schema::Object { name, fields } => {
            assert_eq!(name, "StructWithVec");
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].name, "items");
            match &fields[0].schema {
                Schema::Array(inner) => {
                    assert_eq!(**inner, Schema::String);
                }
                _ => panic!("Expected Array schema"),
            }

            assert_eq!(fields[1].name, "numbers");
            match &fields[1].schema {
                Schema::Array(inner) => {
                    assert_eq!(**inner, Schema::Int);
                }
                _ => panic!("Expected Array schema"),
            }
        }
        _ => panic!("Expected Object schema"),
    }
}

#[derive(SchemaInfo)]
struct NestedStruct {
    inner: SimpleStruct,
    value: i32,
}

#[test]
fn test_nested_struct() {
    let schema = NestedStruct::schema();

    match schema {
        Schema::Object { name, fields } => {
            assert_eq!(name, "NestedStruct");
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].name, "inner");
            match &fields[0].schema {
                Schema::Object { name, fields } => {
                    assert_eq!(name, "SimpleStruct");
                    assert_eq!(fields.len(), 2);
                }
                _ => panic!("Expected nested Object schema"),
            }

            assert_eq!(fields[1].name, "value");
            assert_eq!(fields[1].schema, Schema::Int);
        }
        _ => panic!("Expected Object schema"),
    }
}

// ============================================================================
// Tuple Struct Tests
// ============================================================================

#[derive(SchemaInfo)]
struct TupleStruct(String, i32, bool);

#[test]
fn test_tuple_struct() {
    let schema = TupleStruct::schema();

    match schema {
        Schema::Tuple(types) => {
            assert_eq!(types.len(), 3);
            assert_eq!(types[0], Schema::String);
            assert_eq!(types[1], Schema::Int);
            assert_eq!(types[2], Schema::Bool);
        }
        _ => panic!("Expected Tuple schema"),
    }
}

#[derive(SchemaInfo)]
struct SingleTuple(String);

#[test]
fn test_single_tuple() {
    let schema = SingleTuple::schema();

    match schema {
        Schema::Tuple(types) => {
            assert_eq!(types.len(), 1);
            assert_eq!(types[0], Schema::String);
        }
        _ => panic!("Expected Tuple schema"),
    }
}

// ============================================================================
// Unit Struct Tests
// ============================================================================

#[derive(SchemaInfo)]
struct UnitStruct;

#[test]
fn test_unit_struct() {
    let schema = UnitStruct::schema();
    assert_eq!(schema, Schema::Null);
}

// ============================================================================
// Enum Tests
// ============================================================================

#[derive(SchemaInfo)]
enum SimpleEnum {
    Variant1,
    Variant2,
    Variant3,
}

#[test]
fn test_simple_enum() {
    let schema = SimpleEnum::schema();

    match schema {
        Schema::Union { name, variants } => {
            assert_eq!(name, "SimpleEnum");
            assert_eq!(variants.len(), 3);

            assert_eq!(variants[0].name, "Variant1");
            assert_eq!(variants[0].schema, Schema::Null);

            assert_eq!(variants[1].name, "Variant2");
            assert_eq!(variants[1].schema, Schema::Null);

            assert_eq!(variants[2].name, "Variant3");
            assert_eq!(variants[2].schema, Schema::Null);
        }
        _ => panic!("Expected Union schema"),
    }
}

#[derive(SchemaInfo)]
enum EnumWithData {
    Unit,
    Tuple(String, i32),
    Struct { name: String, value: i32 },
}

#[test]
fn test_enum_with_data() {
    let schema = EnumWithData::schema();

    match schema {
        Schema::Union { name, variants } => {
            assert_eq!(name, "EnumWithData");
            assert_eq!(variants.len(), 3);

            // Unit variant
            assert_eq!(variants[0].name, "Unit");
            assert_eq!(variants[0].schema, Schema::Null);

            // Tuple variant
            assert_eq!(variants[1].name, "Tuple");
            match &variants[1].schema {
                Schema::Tuple(types) => {
                    assert_eq!(types.len(), 2);
                    assert_eq!(types[0], Schema::String);
                    assert_eq!(types[1], Schema::Int);
                }
                _ => panic!("Expected Tuple schema for Tuple variant"),
            }

            // Struct variant
            assert_eq!(variants[2].name, "Struct");
            match &variants[2].schema {
                Schema::Object { name, fields } => {
                    assert_eq!(name, "Struct");
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].name, "name");
                    assert_eq!(fields[0].schema, Schema::String);
                    assert_eq!(fields[1].name, "value");
                    assert_eq!(fields[1].schema, Schema::Int);
                }
                _ => panic!("Expected Object schema for Struct variant"),
            }
        }
        _ => panic!("Expected Union schema"),
    }
}

// ============================================================================
// Complex Types Tests
// ============================================================================

#[derive(SchemaInfo)]
struct ComplexStruct {
    optional_vec: Option<Vec<String>>,
    vec_of_optional: Vec<Option<i32>>,
    nested_option: Option<Option<String>>,
}

#[test]
fn test_complex_nested_types() {
    let schema = ComplexStruct::schema();

    match schema {
        Schema::Object { name, fields } => {
            assert_eq!(name, "ComplexStruct");
            assert_eq!(fields.len(), 3);

            // optional_vec: Option<Vec<String>>
            assert_eq!(fields[0].name, "optional_vec");
            match &fields[0].schema {
                Schema::Optional(inner) => match &**inner {
                    Schema::Array(inner2) => {
                        assert_eq!(**inner2, Schema::String);
                    }
                    _ => panic!("Expected Array inside Optional"),
                },
                _ => panic!("Expected Optional schema"),
            }

            // vec_of_optional: Vec<Option<i32>>
            assert_eq!(fields[1].name, "vec_of_optional");
            match &fields[1].schema {
                Schema::Array(inner) => match &**inner {
                    Schema::Optional(inner2) => {
                        assert_eq!(**inner2, Schema::Int);
                    }
                    _ => panic!("Expected Optional inside Array"),
                },
                _ => panic!("Expected Array schema"),
            }

            // nested_option: Option<Option<String>>
            assert_eq!(fields[2].name, "nested_option");
            match &fields[2].schema {
                Schema::Optional(inner) => match &**inner {
                    Schema::Optional(inner2) => {
                        assert_eq!(**inner2, Schema::String);
                    }
                    _ => panic!("Expected Optional inside Optional"),
                },
                _ => panic!("Expected Optional schema"),
            }
        }
        _ => panic!("Expected Object schema"),
    }
}

// ============================================================================
// Type Name Tests
// ============================================================================

#[test]
fn test_type_names() {
    assert_eq!(SimpleStruct::schema().type_name(), "SimpleStruct");
    assert_eq!(TupleStruct::schema().type_name(), "(string, int, bool)");
    assert_eq!(SimpleEnum::schema().type_name(), "SimpleEnum");
    assert_eq!(UnitStruct::schema().type_name(), "null");
}

// ============================================================================
// Multiple Structs with Same Field Names
// ============================================================================

#[derive(SchemaInfo)]
struct User {
    id: i64,
    name: String,
}

#[derive(SchemaInfo)]
struct Product {
    id: i64,
    name: String,
}

#[test]
fn test_different_types_same_fields() {
    let user_schema = User::schema();
    let product_schema = Product::schema();

    // Should have different type names
    assert_eq!(user_schema.type_name(), "User");
    assert_eq!(product_schema.type_name(), "Product");

    // Both should have the same field structure
    match (user_schema, product_schema) {
        (
            Schema::Object {
                fields: user_fields,
                ..
            },
            Schema::Object {
                fields: product_fields,
                ..
            },
        ) => {
            assert_eq!(user_fields.len(), product_fields.len());
            assert_eq!(user_fields[0].name, product_fields[0].name);
            assert_eq!(user_fields[1].name, product_fields[1].name);
        }
        _ => panic!("Expected Object schemas"),
    }
}
