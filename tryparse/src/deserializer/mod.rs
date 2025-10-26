//! Smart deserializer with type coercion.

pub mod enum_coercer;
pub mod primitives;
pub mod struct_coercer;
pub mod traits;
pub mod union_coercer;

pub use enum_coercer::{EnumMatcher, EnumVariant};
use primitives::value_type_name;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
pub use struct_coercer::{FieldDescriptor, StructDeserializer};
pub use traits::{CoercionContext, LlmDeserialize};
pub use union_coercer::{UnionDeserializer, UnionMatch};

use crate::{
    error::DeserializeError,
    value::{FlexValue, Transformation},
};

/// A deserializer that performs smart type coercion.
///
/// This deserializer wraps a `FlexValue` and implements the `serde::Deserializer`
/// trait with smart coercion capabilities.
///
/// # Examples
///
/// ```
/// use tryparse::deserializer::CoercingDeserializer;
/// use tryparse::value::{FlexValue, Source};
/// use serde::Deserialize;
/// use serde_json::json;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct User {
///     name: String,
///     age: u32,
/// }
///
/// let value = FlexValue::new(
///     json!({"name": "Alice", "age": "30"}),
///     Source::Direct
/// );
/// let mut deserializer = CoercingDeserializer::new(value);
/// let user = User::deserialize(&mut deserializer).unwrap();
/// assert_eq!(user.age, 30); // String "30" coerced to u32
/// ```
pub struct CoercingDeserializer {
    value: FlexValue,
}

impl CoercingDeserializer {
    /// Creates a new coercing deserializer from a `FlexValue`.
    #[inline]
    pub fn new(value: FlexValue) -> Self {
        Self { value }
    }

    /// Consumes the deserializer and returns the `FlexValue` with all transformations.
    #[inline]
    pub fn into_value(self) -> FlexValue {
        self.value
    }

    /// Adds a transformation to the internal value.
    fn add_transformation(&mut self, trans: Transformation) {
        self.value.add_transformation(trans);
    }
}

impl<'de> de::Deserializer<'de> for &mut CoercingDeserializer {
    type Error = DeserializeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.value.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    visitor.visit_i64(i)
                } else if let Some(u) = n.as_u64() {
                    visitor.visit_u64(u)
                } else if let Some(f) = n.as_f64() {
                    visitor.visit_f64(f)
                } else {
                    Err(DeserializeError::invalid_value("invalid number"))
                }
            }
            Value::String(s) => visitor.visit_string(s.clone()),
            Value::Array(_) => self.deserialize_seq(visitor),
            Value::Object(_) => self.deserialize_map(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.value.value {
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::String(s) => {
                // Try to parse string as bool
                let lower = s.to_lowercase();
                if lower == "true" || lower == "yes" || lower == "1" {
                    self.add_transformation(Transformation::StringToNumber {
                        original: s.clone(),
                    });
                    visitor.visit_bool(true)
                } else if lower == "false" || lower == "no" || lower == "0" {
                    self.add_transformation(Transformation::StringToNumber {
                        original: s.clone(),
                    });
                    visitor.visit_bool(false)
                } else {
                    Err(DeserializeError::type_mismatch(
                        "bool",
                        format!("string: {}", s),
                    ))
                }
            }
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    self.add_transformation(Transformation::FloatToInt { original: i as f64 });
                    visitor.visit_bool(i != 0)
                } else {
                    Err(DeserializeError::type_mismatch("bool", "number"))
                }
            }
            _ => Err(DeserializeError::type_mismatch(
                "bool",
                value_type_name(&self.value.value),
            )),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor, |n| n)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor, |n| n)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor, |n| n)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor, |n| n)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor, |n| n)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor, |n| n)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor, |n| n)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_integer(visitor, |n| n)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_float(visitor, |f| f)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_float(visitor, |f| f)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.value.value {
            Value::String(s) => {
                let mut chars = s.chars();
                if let Some(c) = chars.next() {
                    if chars.next().is_none() {
                        visitor.visit_char(c)
                    } else {
                        Err(DeserializeError::invalid_value(
                            "string has more than one character",
                        ))
                    }
                } else {
                    Err(DeserializeError::invalid_value("empty string"))
                }
            }
            _ => Err(DeserializeError::type_mismatch(
                "char",
                value_type_name(&self.value.value),
            )),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.value.clone() {
            Value::String(s) => visitor.visit_string(s),
            Value::Number(n) => {
                let s = n.to_string();
                self.add_transformation(Transformation::StringToNumber {
                    original: s.clone(),
                });
                visitor.visit_string(s)
            }
            Value::Bool(b) => {
                let s = b.to_string();
                self.add_transformation(Transformation::StringToNumber {
                    original: s.clone(),
                });
                visitor.visit_string(s)
            }
            _ => Err(DeserializeError::type_mismatch(
                "string",
                value_type_name(&self.value.value),
            )),
        }
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(DeserializeError::Custom("bytes not supported".to_string()))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(DeserializeError::Custom(
            "byte_buf not supported".to_string(),
        ))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.value.value {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.value.value {
            Value::Null => visitor.visit_unit(),
            _ => Err(DeserializeError::type_mismatch(
                "unit",
                value_type_name(&self.value.value),
            )),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.value.clone() {
            Value::Array(arr) => {
                let seq = SeqDeserializer {
                    items: arr,
                    index: 0,
                    source: self.value.source.clone(),
                };
                visitor.visit_seq(seq)
            }
            _ => {
                // Try to wrap single value in array
                self.add_transformation(Transformation::SingleToArray);
                let seq = SingleValueSeq { value: Some(self) };
                visitor.visit_seq(seq)
            }
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.value.clone() {
            Value::Object(obj) => {
                let entries: Vec<_> = obj.into_iter().collect();
                let map = MapDeserializer {
                    entries,
                    index: 0,
                    value: None,
                    source: self.value.source.clone(),
                };
                visitor.visit_map(map)
            }
            _ => Err(DeserializeError::type_mismatch(
                "map",
                value_type_name(&self.value.value),
            )),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.value.clone() {
            Value::String(s) => visitor.visit_enum(s.as_str().into_deserializer()),
            Value::Object(obj) if obj.len() == 1 => {
                // SAFETY: We've already checked that obj.len() == 1
                if let Some((key, value)) = obj.into_iter().next() {
                    visitor.visit_enum(EnumDeserializer {
                        variant: key,
                        value,
                        source: self.value.source.clone(),
                    })
                } else {
                    // This should be unreachable given the guard above
                    Err(DeserializeError::Custom(
                        "empty object despite len check".to_string(),
                    ))
                }
            }
            _ => Err(DeserializeError::type_mismatch(
                "enum",
                value_type_name(&self.value.value),
            )),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

// Helper implementations
impl CoercingDeserializer {
    fn deserialize_integer<'de, V>(
        &mut self,
        visitor: V,
        _convert: fn(i64) -> i64,
    ) -> Result<V::Value, DeserializeError>
    where
        V: Visitor<'de>,
    {
        match self.value.value.clone() {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    visitor.visit_i64(i)
                } else if let Some(u) = n.as_u64() {
                    visitor.visit_u64(u)
                } else if let Some(f) = n.as_f64() {
                    self.add_transformation(Transformation::FloatToInt { original: f });
                    visitor.visit_i64(f as i64)
                } else {
                    Err(DeserializeError::invalid_value("invalid number"))
                }
            }
            Value::String(s) => {
                // Try standard parsing first
                if let Ok(i) = s.parse::<i64>() {
                    self.add_transformation(Transformation::StringToNumber { original: s });
                    visitor.visit_i64(i)
                }
                // Fall back to BAML's advanced number parsing (handles percentages, fractions, currency)
                else if let Some(f) = primitives::parse_comma_separated_number(&s) {
                    self.add_transformation(Transformation::FloatToInt { original: f });
                    visitor.visit_i64(f.round() as i64)
                } else {
                    Err(DeserializeError::type_mismatch(
                        "integer",
                        format!("string: {}", s),
                    ))
                }
            }
            _ => Err(DeserializeError::type_mismatch(
                "integer",
                value_type_name(&self.value.value),
            )),
        }
    }

    fn deserialize_float<'de, V>(
        &mut self,
        visitor: V,
        _convert: fn(f64) -> f64,
    ) -> Result<V::Value, DeserializeError>
    where
        V: Visitor<'de>,
    {
        match self.value.value.clone() {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    visitor.visit_f64(f)
                } else {
                    Err(DeserializeError::type_mismatch("float", "number"))
                }
            }
            Value::String(s) => {
                // Try standard parsing first
                if let Ok(f) = s.parse::<f64>() {
                    self.add_transformation(Transformation::StringToNumber { original: s });
                    visitor.visit_f64(f)
                }
                // Fall back to BAML's advanced number parsing (handles percentages, fractions, currency)
                else if let Some(f) = primitives::parse_comma_separated_number(&s) {
                    self.add_transformation(Transformation::StringToNumber { original: s });
                    visitor.visit_f64(f)
                } else {
                    Err(DeserializeError::type_mismatch(
                        "float",
                        format!("string: {}", s),
                    ))
                }
            }
            _ => Err(DeserializeError::type_mismatch(
                "float",
                value_type_name(&self.value.value),
            )),
        }
    }
}

// Sequence deserializer for arrays
struct SeqDeserializer {
    items: Vec<Value>,
    index: usize,
    source: crate::value::Source,
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = DeserializeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.index < self.items.len() {
            let value = self.items[self.index].clone();
            self.index += 1;
            let flex_value = FlexValue::new(value, self.source.clone());
            let mut deserializer = CoercingDeserializer::new(flex_value);
            seed.deserialize(&mut deserializer).map(Some)
        } else {
            Ok(None)
        }
    }
}

// Single value sequence (for coercing single values to arrays)
struct SingleValueSeq<'a> {
    value: Option<&'a mut CoercingDeserializer>,
}

impl<'de, 'a> SeqAccess<'de> for SingleValueSeq<'a> {
    type Error = DeserializeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(deserializer) => seed.deserialize(deserializer).map(Some),
            None => Ok(None),
        }
    }
}

// Map deserializer
struct MapDeserializer {
    entries: Vec<(String, Value)>,
    index: usize,
    value: Option<Value>,
    source: crate::value::Source,
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = DeserializeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.index < self.entries.len() {
            let (key, value) = &self.entries[self.index];
            self.value = Some(value.clone());
            let key_value = Value::String(key.clone());
            let flex_value = FlexValue::new(key_value, self.source.clone());
            let mut deserializer = CoercingDeserializer::new(flex_value);
            seed.deserialize(&mut deserializer).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                self.index += 1;
                let flex_value = FlexValue::new(value, self.source.clone());
                let mut deserializer = CoercingDeserializer::new(flex_value);
                seed.deserialize(&mut deserializer)
            }
            None => Err(DeserializeError::Custom("value is missing".to_string())),
        }
    }
}

// Enum deserializer
struct EnumDeserializer {
    variant: String,
    value: Value,
    source: crate::value::Source,
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer {
    type Error = DeserializeError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant_value = Value::String(self.variant.clone());
        let flex_value = FlexValue::new(variant_value, self.source.clone());
        let mut deserializer = CoercingDeserializer::new(flex_value);
        let v = seed.deserialize(&mut deserializer)?;
        Ok((v, self))
    }
}

impl<'de> de::VariantAccess<'de> for EnumDeserializer {
    type Error = DeserializeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let flex_value = FlexValue::new(self.value, self.source);
        let mut deserializer = CoercingDeserializer::new(flex_value);
        seed.deserialize(&mut deserializer)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let flex_value = FlexValue::new(self.value, self.source);
        let mut deserializer = CoercingDeserializer::new(flex_value);
        de::Deserializer::deserialize_seq(&mut deserializer, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let flex_value = FlexValue::new(self.value, self.source);
        let mut deserializer = CoercingDeserializer::new(flex_value);
        de::Deserializer::deserialize_map(&mut deserializer, visitor)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;
    use crate::value::Source;

    #[test]
    fn test_deserialize_string_to_number() {
        let value = FlexValue::new(json!({"age": "30"}), Source::Direct);
        let mut deserializer = CoercingDeserializer::new(value);

        #[derive(Deserialize, Debug, PartialEq)]
        struct Test {
            age: u32,
        }

        let result: Test = Test::deserialize(&mut deserializer).unwrap();
        assert_eq!(result.age, 30);
    }

    #[test]
    fn test_deserialize_float_to_int() {
        let value = FlexValue::new(json!({"count": 42.7}), Source::Direct);
        let mut deserializer = CoercingDeserializer::new(value);

        #[derive(Deserialize, Debug, PartialEq)]
        struct Test {
            count: i32,
        }

        let result: Test = Test::deserialize(&mut deserializer).unwrap();
        assert_eq!(result.count, 42);
    }

    #[test]
    fn test_deserialize_single_to_array() {
        let value = FlexValue::new(json!({"items": "single"}), Source::Direct);
        let mut deserializer = CoercingDeserializer::new(value);

        #[derive(Deserialize, Debug, PartialEq)]
        struct Test {
            items: Vec<String>,
        }

        let result: Test = Test::deserialize(&mut deserializer).unwrap();
        assert_eq!(result.items, vec!["single"]);
    }
}
