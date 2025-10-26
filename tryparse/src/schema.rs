//! Schema representation for compile-time type information.
//!
//! This module provides the [`Schema`] type and [`SchemaInfo`] trait for representing
//! the structure of types that can be deserialized from LLM output.
//!
//! The schema information is used by the parser to make better decisions about
//! how to interpret ambiguous LLM output.

use std::collections::HashMap;

/// Represents the schema of a type that can be deserialized from LLM output.
///
/// This is a compile-time representation of type structure that helps the parser
/// make intelligent decisions about how to parse ambiguous content.
#[derive(Debug, Clone, PartialEq)]
pub enum Schema {
    /// String type
    String,

    /// Integer type (i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize)
    Int,

    /// Float type (f32, f64)
    Float,

    /// Boolean type
    Bool,

    /// Null/unit type
    Null,

    /// Object/struct with named fields
    Object {
        /// Name of the type (e.g., "User", "Config")
        name: String,
        /// Fields in the object
        fields: Vec<Field>,
    },

    /// Array/Vec of elements
    Array(Box<Schema>),

    /// Optional type (Option<T>)
    Optional(Box<Schema>),

    /// Union type (enum with variants)
    Union {
        /// Name of the enum (e.g., "Status", "Result")
        name: String,
        /// Possible variants
        variants: Vec<Variant>,
    },

    /// Tuple (T1, T2, ...)
    Tuple(Vec<Schema>),

    /// Map (HashMap, BTreeMap)
    Map {
        /// Key type
        key: Box<Schema>,
        /// Value type
        value: Box<Schema>,
    },
}

impl Schema {
    /// Returns a human-readable name for the schema type.
    #[inline]
    pub fn type_name(&self) -> String {
        match self {
            Schema::String => "string".to_string(),
            Schema::Int => "int".to_string(),
            Schema::Float => "float".to_string(),
            Schema::Bool => "bool".to_string(),
            Schema::Null => "null".to_string(),
            Schema::Object { name, .. } => name.clone(),
            Schema::Array(inner) => format!("array<{}>", inner.type_name()),
            Schema::Optional(inner) => format!("optional<{}>", inner.type_name()),
            Schema::Union { name, .. } => name.clone(),
            Schema::Tuple(schemas) => {
                let types = schemas
                    .iter()
                    .map(|s| s.type_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", types)
            }
            Schema::Map { key, value } => {
                format!("map<{}, {}>", key.type_name(), value.type_name())
            }
        }
    }

    /// Returns true if this schema represents a primitive type.
    #[inline]
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Schema::String | Schema::Int | Schema::Float | Schema::Bool | Schema::Null
        )
    }

    /// Returns true if this schema represents a composite type (object, array, etc.).
    #[inline]
    pub fn is_composite(&self) -> bool {
        !self.is_primitive()
    }
}

/// Represents a field in an object schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Field name
    pub name: String,
    /// Field type schema
    pub schema: Schema,
    /// Whether the field is required
    pub required: bool,
    /// Alternative names for this field (for field name normalization)
    pub aliases: Vec<String>,
}

impl Field {
    /// Create a new required field.
    pub fn new(name: impl Into<String>, schema: Schema) -> Self {
        Self {
            name: name.into(),
            schema,
            required: true,
            aliases: Vec::new(),
        }
    }

    /// Mark this field as optional.
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Add an alias for this field name.
    ///
    /// This is useful for handling different naming conventions (camelCase, snake_case, etc.)
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Add multiple aliases at once.
    pub fn with_aliases(mut self, aliases: impl IntoIterator<Item = String>) -> Self {
        self.aliases.extend(aliases);
        self
    }

    /// Check if a given name matches this field (including aliases).
    #[inline]
    pub fn matches_name(&self, name: &str) -> bool {
        self.name == name || self.aliases.iter().any(|alias| alias == name)
    }
}

/// Represents a variant in a union schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    /// Variant name
    pub name: String,
    /// Schema of the variant's content
    pub schema: Schema,
}

impl Variant {
    /// Create a new variant.
    pub fn new(name: impl Into<String>, schema: Schema) -> Self {
        Self {
            name: name.into(),
            schema,
        }
    }
}

/// Trait for types that can provide compile-time schema information.
///
/// This trait is typically derived automatically using `#[derive(SchemaInfo)]`.
///
/// # Example
///
/// ```ignore
/// use tryparse::SchemaInfo;
///
/// #[derive(SchemaInfo)]
/// struct User {
///     name: String,
///     age: u32,
/// }
///
/// let schema = User::schema();
/// // Schema::Object { name: "User", fields: [...] }
/// ```
pub trait SchemaInfo {
    /// Returns the schema for this type.
    fn schema() -> Schema;
}

// ============================================================================
// Manual SchemaInfo implementations for primitive types
// ============================================================================

impl SchemaInfo for String {
    fn schema() -> Schema {
        Schema::String
    }
}

impl SchemaInfo for &str {
    fn schema() -> Schema {
        Schema::String
    }
}

impl SchemaInfo for i8 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for i16 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for i32 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for i64 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for i128 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for isize {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for u8 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for u16 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for u32 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for u64 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for u128 {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for usize {
    fn schema() -> Schema {
        Schema::Int
    }
}

impl SchemaInfo for f32 {
    fn schema() -> Schema {
        Schema::Float
    }
}

impl SchemaInfo for f64 {
    fn schema() -> Schema {
        Schema::Float
    }
}

impl SchemaInfo for bool {
    fn schema() -> Schema {
        Schema::Bool
    }
}

impl SchemaInfo for () {
    fn schema() -> Schema {
        Schema::Null
    }
}

// ============================================================================
// Generic SchemaInfo implementations
// ============================================================================

impl<T: SchemaInfo> SchemaInfo for Option<T> {
    fn schema() -> Schema {
        Schema::Optional(Box::new(T::schema()))
    }
}

impl<T: SchemaInfo> SchemaInfo for Vec<T> {
    fn schema() -> Schema {
        Schema::Array(Box::new(T::schema()))
    }
}

impl<T: SchemaInfo, const N: usize> SchemaInfo for [T; N] {
    fn schema() -> Schema {
        Schema::Array(Box::new(T::schema()))
    }
}

impl<K: SchemaInfo, V: SchemaInfo> SchemaInfo for HashMap<K, V> {
    fn schema() -> Schema {
        Schema::Map {
            key: Box::new(K::schema()),
            value: Box::new(V::schema()),
        }
    }
}

// Tuple implementations (up to 4 elements for now)
impl<T1: SchemaInfo> SchemaInfo for (T1,) {
    fn schema() -> Schema {
        Schema::Tuple(vec![T1::schema()])
    }
}

impl<T1: SchemaInfo, T2: SchemaInfo> SchemaInfo for (T1, T2) {
    fn schema() -> Schema {
        Schema::Tuple(vec![T1::schema(), T2::schema()])
    }
}

impl<T1: SchemaInfo, T2: SchemaInfo, T3: SchemaInfo> SchemaInfo for (T1, T2, T3) {
    fn schema() -> Schema {
        Schema::Tuple(vec![T1::schema(), T2::schema(), T3::schema()])
    }
}

impl<T1: SchemaInfo, T2: SchemaInfo, T3: SchemaInfo, T4: SchemaInfo> SchemaInfo
    for (T1, T2, T3, T4)
{
    fn schema() -> Schema {
        Schema::Tuple(vec![T1::schema(), T2::schema(), T3::schema(), T4::schema()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_schemas() {
        assert_eq!(String::schema(), Schema::String);
        assert_eq!(i32::schema(), Schema::Int);
        assert_eq!(u64::schema(), Schema::Int);
        assert_eq!(f64::schema(), Schema::Float);
        assert_eq!(bool::schema(), Schema::Bool);
        assert_eq!(<()>::schema(), Schema::Null);
    }

    #[test]
    fn test_optional_schema() {
        let schema = Option::<String>::schema();
        assert_eq!(schema, Schema::Optional(Box::new(Schema::String)));

        let schema = Option::<i32>::schema();
        assert_eq!(schema, Schema::Optional(Box::new(Schema::Int)));
    }

    #[test]
    fn test_vec_schema() {
        let schema = Vec::<i32>::schema();
        assert_eq!(schema, Schema::Array(Box::new(Schema::Int)));

        let schema = Vec::<String>::schema();
        assert_eq!(schema, Schema::Array(Box::new(Schema::String)));
    }

    #[test]
    fn test_array_schema() {
        let schema = <[i32; 5]>::schema();
        assert_eq!(schema, Schema::Array(Box::new(Schema::Int)));
    }

    #[test]
    fn test_hashmap_schema() {
        let schema = HashMap::<String, i32>::schema();
        assert_eq!(
            schema,
            Schema::Map {
                key: Box::new(Schema::String),
                value: Box::new(Schema::Int)
            }
        );
    }

    #[test]
    fn test_tuple_schema() {
        let schema = <(String, i32)>::schema();
        assert_eq!(schema, Schema::Tuple(vec![Schema::String, Schema::Int]));

        let schema = <(String, i32, bool)>::schema();
        assert_eq!(
            schema,
            Schema::Tuple(vec![Schema::String, Schema::Int, Schema::Bool])
        );
    }

    #[test]
    fn test_nested_schemas() {
        // Vec<Option<String>>
        let schema = Vec::<Option<String>>::schema();
        assert_eq!(
            schema,
            Schema::Array(Box::new(Schema::Optional(Box::new(Schema::String))))
        );

        // Option<Vec<i32>>
        let schema = Option::<Vec<i32>>::schema();
        assert_eq!(
            schema,
            Schema::Optional(Box::new(Schema::Array(Box::new(Schema::Int))))
        );
    }

    #[test]
    fn test_schema_type_name() {
        assert_eq!(Schema::String.type_name(), "string");
        assert_eq!(Schema::Int.type_name(), "int");
        assert_eq!(
            Schema::Array(Box::new(Schema::String)).type_name(),
            "array<string>"
        );
        assert_eq!(
            Schema::Optional(Box::new(Schema::Int)).type_name(),
            "optional<int>"
        );
    }

    #[test]
    fn test_schema_is_primitive() {
        assert!(Schema::String.is_primitive());
        assert!(Schema::Int.is_primitive());
        assert!(Schema::Bool.is_primitive());
        assert!(!Schema::Array(Box::new(Schema::String)).is_primitive());
        assert!(!Schema::Optional(Box::new(Schema::Int)).is_primitive());
    }

    #[test]
    fn test_field_creation() {
        let field = Field::new("name", Schema::String);
        assert_eq!(field.name, "name");
        assert_eq!(field.schema, Schema::String);
        assert!(field.required);
        assert!(field.aliases.is_empty());
    }

    #[test]
    fn test_field_optional() {
        let field = Field::new("age", Schema::Int).optional();
        assert!(!field.required);
    }

    #[test]
    fn test_field_with_alias() {
        let field = Field::new("user_name", Schema::String)
            .with_alias("userName")
            .with_alias("UserName");

        assert!(field.matches_name("user_name"));
        assert!(field.matches_name("userName"));
        assert!(field.matches_name("UserName"));
        assert!(!field.matches_name("name"));
    }

    #[test]
    fn test_variant_creation() {
        let variant = Variant::new("Active", Schema::Null);
        assert_eq!(variant.name, "Active");
        assert_eq!(variant.schema, Schema::Null);
    }
}
