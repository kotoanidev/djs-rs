//! Arbitrary JSON values.

use std::collections::BTreeMap;

use compact_str::CompactString;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{
        self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, Unexpected,
        VariantAccess, Visitor,
        value::{MapDeserializer, SeqDeserializer},
    },
    forward_to_deserialize_any,
    ser::{SerializeMap, SerializeSeq},
};

/// Any JSON value, for the places where the API has arbitrary or "mixed" data (D-070, D-072):
/// [audit log change values](crate::audit_log::AuditLogChange) and game-defined metadata.
///
/// Convert it with [`to_type`](Self::to_type) to a concrete type, e.g.
/// [`Permissions`](crate::permissions::Permissions) for a `permissions` change or
/// `Vec<`[`Overwrite`](crate::channel::Overwrite)`>` for `permission_overwrites`:
/// `&JsonValue` is a serde [`Deserializer`] that lends out its strings.
///
/// # Examples
///
/// ```
/// use djs_model::util::JsonValue;
///
/// let value: JsonValue = serde_json::from_str(r#"{"wins":12,"rank":"Gold"}"#).unwrap();
/// let JsonValue::Object(fields) = &value else { panic!() };
/// assert_eq!(fields["wins"].to_type::<u32>(), Ok(12));
/// assert_eq!(fields["rank"].as_str(), Some("Gold"));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    /// `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A non-negative integer.
    Unsigned(u64),
    /// A negative integer.
    Signed(i64),
    /// A number with a fraction (or out of the integer ranges).
    Float(f64),
    /// A string (snowflakes, permissions, timestamps and hashes are strings too).
    String(CompactString),
    /// An array.
    Array(Vec<JsonValue>),
    /// An object (keys sorted, D-058).
    Object(BTreeMap<CompactString, JsonValue>),
}

impl JsonValue {
    /// Converts the value to `T` (any type that deserializes from JSON of this shape).
    ///
    /// Enums whose JSON is a string (`str_enum`s) convert from strings; other enums are
    /// decoded through their JSON representation as usual.
    ///
    /// # Errors
    ///
    /// Returns an error if the value does not have the shape of `T`.
    pub fn to_type<'a, T: Deserialize<'a>>(&'a self) -> Result<T, JsonValueError> {
        T::deserialize(self)
    }

    /// Returns the string, if the value is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn unexpected(&self) -> Unexpected<'_> {
        match self {
            Self::Null => Unexpected::Unit,
            Self::Bool(value) => Unexpected::Bool(*value),
            Self::Unsigned(value) => Unexpected::Unsigned(*value),
            Self::Signed(value) => Unexpected::Signed(*value),
            Self::Float(value) => Unexpected::Float(*value),
            Self::String(value) => Unexpected::Str(value),
            Self::Array(_) => Unexpected::Seq,
            Self::Object(_) => Unexpected::Map,
        }
    }
}

impl Serialize for JsonValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Unsigned(value) => serializer.serialize_u64(*value),
            Self::Signed(value) => serializer.serialize_i64(*value),
            Self::Float(value) => serializer.serialize_f64(*value),
            Self::String(value) => serializer.serialize_str(value),
            Self::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            Self::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = JsonValue;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("any JSON value")
            }

            fn visit_unit<E: de::Error>(self) -> Result<JsonValue, E> {
                Ok(JsonValue::Null)
            }

            fn visit_none<E: de::Error>(self) -> Result<JsonValue, E> {
                Ok(JsonValue::Null)
            }

            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<JsonValue, D::Error> {
                deserializer.deserialize_any(self)
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<JsonValue, E> {
                Ok(JsonValue::Bool(value))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<JsonValue, E> {
                Ok(JsonValue::Unsigned(value))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<JsonValue, E> {
                Ok(u64::try_from(value).map_or(JsonValue::Signed(value), JsonValue::Unsigned))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<JsonValue, E> {
                Ok(JsonValue::Float(value))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<JsonValue, E> {
                Ok(JsonValue::String(value.into()))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<JsonValue, A::Error> {
                let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(64));
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(JsonValue::Array(values))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<JsonValue, A::Error> {
                let mut entries = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<CompactString, JsonValue>()? {
                    entries.insert(key, value);
                }
                Ok(JsonValue::Object(entries))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

/// The error of converting a [`JsonValue`] to a type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonValueError(CompactString);

impl core::fmt::Display for JsonValueError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl core::error::Error for JsonValueError {}

impl de::Error for JsonValueError {
    fn custom<T: core::fmt::Display>(message: T) -> Self {
        Self(compact_str::format_compact!("{message}"))
    }
}

impl<'de> IntoDeserializer<'de, JsonValueError> for &'de JsonValue {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

impl<'de> Deserializer<'de> for &'de JsonValue {
    type Error = JsonValueError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            JsonValue::Null => visitor.visit_unit(),
            JsonValue::Bool(value) => visitor.visit_bool(*value),
            JsonValue::Unsigned(value) => visitor.visit_u64(*value),
            JsonValue::Signed(value) => visitor.visit_i64(*value),
            JsonValue::Float(value) => visitor.visit_f64(*value),
            JsonValue::String(value) => visitor.visit_borrowed_str(value),
            JsonValue::Array(values) => {
                let mut seq = SeqDeserializer::new(values.iter());
                let value = visitor.visit_seq(&mut seq)?;
                seq.end()?;
                Ok(value)
            }
            JsonValue::Object(entries) => {
                let mut map =
                    MapDeserializer::new(entries.iter().map(|(key, value)| (key.as_str(), value)));
                let value = visitor.visit_map(&mut map)?;
                map.end()?;
                Ok(value)
            }
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            JsonValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self {
            JsonValue::String(variant) => visitor.visit_enum(UnitVariant(variant)),
            other => Err(de::Error::invalid_type(
                other.unexpected(),
                &"a string enum",
            )),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

/// A string as a unit enum variant (externally tagged enums written as `"Variant"`).
struct UnitVariant<'de>(&'de str);

impl<'de> EnumAccess<'de> for UnitVariant<'de> {
    type Error = JsonValueError;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self), Self::Error> {
        let variant = seed.deserialize(de::value::BorrowedStrDeserializer::new(self.0))?;
        Ok((variant, self))
    }
}

impl<'de> VariantAccess<'de> for UnitVariant<'de> {
    type Error = JsonValueError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        _seed: T,
    ) -> Result<T::Value, Self::Error> {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"a newtype variant",
        ))
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"a tuple variant",
        ))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"a struct variant",
        ))
    }
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use super::*;

    #[test]
    fn unit_enums_convert_from_strings() {
        #[derive(Debug, PartialEq, Deserialize)]
        enum Kind {
            Text,
            Voice,
        }

        let value = JsonValue::String("Voice".into());
        assert_eq!(value.to_type::<Kind>(), Ok(Kind::Voice));
        assert!(JsonValue::String("Stage".into()).to_type::<Kind>().is_err());
        assert!(JsonValue::Unsigned(1).to_type::<Kind>().is_err());
    }

    #[test]
    fn size() {
        assert_eq!(size_of::<JsonValue>(), 32);
    }
}
