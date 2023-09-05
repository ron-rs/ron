//! Value module.

use std::{borrow::Cow, cmp::Eq, hash::Hash};

use serde::{
    de::{DeserializeOwned, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor},
    forward_to_deserialize_any,
};

use crate::{de::Error, error::Result};

mod map;
mod number;
pub(crate) mod raw;

pub use map::Map;
pub use number::{Number, F32, F64};
#[allow(clippy::useless_attribute, clippy::module_name_repetitions)]
pub use raw::RawValue;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Char(char),
    Map(Map),
    Number(Number),
    Option(Option<Box<Value>>),
    String(String),
    Bytes(Vec<u8>),
    Seq(Vec<Value>),
    Unit,
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<char> for Value {
    fn from(value: char) -> Self {
        Self::Char(value)
    }
}

impl<K: Into<Value>, V: Into<Value>> FromIterator<(K, V)> for Value {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self::Map(iter.into_iter().collect())
    }
}

impl From<Map> for Value {
    fn from(value: Map) -> Self {
        Self::Map(value)
    }
}

impl<T: Into<Number>> From<T> for Value {
    fn from(value: T) -> Self {
        Self::Number(value.into())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        Self::Option(value.map(Into::into).map(Box::new))
    }
}

impl<'a> From<&'a str> for Value {
    fn from(value: &'a str) -> Self {
        String::from(value).into()
    }
}

impl<'a> From<Cow<'a, str>> for Value {
    fn from(value: Cow<'a, str>) -> Self {
        String::from(value).into()
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

/// Special case to allow `Value::from(b"byte string")`
impl<const N: usize> From<&'static [u8; N]> for Value {
    fn from(value: &'static [u8; N]) -> Self {
        Self::Bytes(Vec::from(*value))
    }
}

impl<T: Into<Value>> FromIterator<T> for Value {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::Seq(iter.into_iter().map(Into::into).collect())
    }
}

impl<'a, T: Clone + Into<Value>> From<&'a [T]> for Value {
    fn from(value: &'a [T]) -> Self {
        value.iter().map(Clone::clone).map(Into::into).collect()
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        value.into_iter().collect()
    }
}

impl From<()> for Value {
    fn from(_value: ()) -> Self {
        Value::Unit
    }
}

impl Value {
    /// Tries to deserialize this [`Value`] into `T`.
    pub fn into_rust<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        T::deserialize(self)
    }
}

/// Deserializer implementation for RON [`Value`].
/// This does not support enums (because [`Value`] does not store them).
impl<'de> Deserializer<'de> for Value {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    #[cfg(feature = "integer128")]
    forward_to_deserialize_any! {
        i128 u128
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Char(c) => visitor.visit_char(c),
            Value::Map(m) => {
                let old_len = m.len();

                let mut items: Vec<(Value, Value)> = m.into_iter().collect();
                items.reverse();

                let value = visitor.visit_map(MapAccessor {
                    items: &mut items,
                    value: None,
                })?;

                if items.is_empty() {
                    Ok(value)
                } else {
                    Err(Error::ExpectedDifferentLength {
                        expected: format!("a map of length {}", old_len - items.len()),
                        found: old_len,
                    })
                }
            }
            Value::Number(number) => number.visit(visitor),
            Value::Option(Some(o)) => visitor.visit_some(*o),
            Value::Option(None) => visitor.visit_none(),
            Value::String(s) => visitor.visit_string(s),
            Value::Bytes(b) => visitor.visit_byte_buf(b),
            Value::Seq(mut seq) => {
                let old_len = seq.len();

                seq.reverse();
                let value = visitor.visit_seq(SeqAccessor { seq: &mut seq })?;

                if seq.is_empty() {
                    Ok(value)
                } else {
                    Err(Error::ExpectedDifferentLength {
                        expected: format!("a sequence of length {}", old_len - seq.len()),
                        found: old_len,
                    })
                }
            }
            Value::Unit => visitor.visit_unit(),
        }
    }
}

struct SeqAccessor<'a> {
    seq: &'a mut Vec<Value>,
}

impl<'a, 'de> SeqAccess<'de> for SeqAccessor<'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        // The `Vec` is reversed, so we can pop to get the originally first element
        self.seq
            .pop()
            .map_or(Ok(None), |v| seed.deserialize(v).map(Some))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.seq.len())
    }
}

struct MapAccessor<'a> {
    items: &'a mut Vec<(Value, Value)>,
    value: Option<Value>,
}

impl<'a, 'de> MapAccess<'de> for MapAccessor<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // The `Vec` is reversed, so we can pop to get the originally first element
        match self.items.pop() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    #[allow(clippy::panic)]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => panic!("Contract violation: value before key"),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.items.len())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fmt::Debug};

    use serde::Deserialize;

    use super::*;

    fn assert_same<'de, T>(s: &'de str)
    where
        T: Debug + Deserialize<'de> + PartialEq,
    {
        use crate::de::from_str;

        let direct: T = from_str(s).unwrap();
        let value: Value = from_str(s).unwrap();
        let de = T::deserialize(value.clone()).unwrap();

        assert_eq!(direct, de, "Deserialization for {:?} is not the same", s);

        let value_roundtrip = Value::deserialize(value.clone()).unwrap();
        assert_eq!(value_roundtrip, value);
    }

    fn assert_same_bytes<'de, T>(s: &'de [u8])
    where
        T: Debug + Deserialize<'de> + PartialEq,
    {
        use crate::de::from_bytes;

        let direct: T = from_bytes(s).unwrap();
        let value: Value = from_bytes(s).unwrap();
        let de = T::deserialize(value.clone()).unwrap();

        assert_eq!(direct, de, "Deserialization for {:?} is not the same", s);

        let value_roundtrip = Value::deserialize(value.clone()).unwrap();
        assert_eq!(value_roundtrip, value);
    }

    #[test]
    fn boolean() {
        assert_same::<bool>("true");
        assert_same::<bool>("false");

        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(false), Value::Bool(false));
    }

    #[test]
    fn float() {
        assert_same::<f64>("0.123");
        assert_same::<f64>("-4.19");

        assert_eq!(
            Value::from(42_f32),
            Value::Number(Number::F32(42_f32.into()))
        );
        assert_eq!(
            Value::from(42_f64),
            Value::Number(Number::F64(42_f64.into()))
        );
    }

    #[test]
    fn int() {
        assert_same::<u32>("626");
        assert_same::<i32>("-50");

        assert_eq!(Value::from(0_i8), Value::Number(Number::I8(0)));
        assert_eq!(Value::from(0_i16), Value::Number(Number::I16(0)));
        assert_eq!(Value::from(0_i32), Value::Number(Number::I32(0)));
        assert_eq!(Value::from(0_i64), Value::Number(Number::I64(0)));
        #[cfg(feature = "integer128")]
        assert_eq!(Value::from(0_i128), Value::Number(Number::I128(0)));
        assert_eq!(Value::from(0_u8), Value::Number(Number::U8(0)));
        assert_eq!(Value::from(0_u16), Value::Number(Number::U16(0)));
        assert_eq!(Value::from(0_u32), Value::Number(Number::U32(0)));
        assert_eq!(Value::from(0_u64), Value::Number(Number::U64(0)));
        #[cfg(feature = "integer128")]
        assert_eq!(Value::from(0_u128), Value::Number(Number::U128(0)));
    }

    #[test]
    fn char() {
        assert_same::<char>("'4'");
        assert_same::<char>("'c'");

        assert_eq!(Value::from('🦀'), Value::Char('🦀'));
    }

    #[test]
    fn string() {
        assert_same::<String>(r#""hello world""#);
        assert_same::<String>(r#""this is a Rusty 🦀 string""#);
        assert_same::<String>(r#""this is now valid UTF-8 \xf0\x9f\xa6\x80""#);

        assert_eq!(Value::from("slice"), Value::String(String::from("slice")));
        assert_eq!(
            Value::from(String::from("string")),
            Value::String(String::from("string"))
        );
        assert_eq!(
            Value::from(Cow::Borrowed("cow")),
            Value::String(String::from("cow"))
        );
    }

    #[test]
    fn bytes() {
        assert_same_bytes::<serde_bytes::ByteBuf>(br#"b"hello world""#);
        assert_same_bytes::<serde_bytes::ByteBuf>(
            br#"b"this is not valid UTF-8 \xf8\xa1\xa1\xa1\xa1""#,
        );

        assert_eq!(Value::from(b"bytes"), Value::Bytes(Vec::from(*b"bytes")));
    }

    #[test]
    fn map() {
        assert_same::<BTreeMap<char, String>>(
            "{
'a': \"Hello\",
'b': \"Bye\",
        }",
        );

        assert_eq!(Value::from(Map::new()), Value::Map(Map::new()));
        assert_eq!(
            Value::from_iter([("a", 42)]),
            Value::Map({
                let mut map = Map::new();
                map.insert(Value::from("a"), Value::from(42));
                map
            })
        );
    }

    #[test]
    fn option() {
        assert_same::<Option<char>>("Some('a')");
        assert_same::<Option<char>>("None");

        assert_eq!(Value::from(Option::<bool>::None), Value::Option(None));
        assert_eq!(
            Value::from(Some(false)),
            Value::Option(Some(Box::new(Value::Bool(false))))
        );
        assert_eq!(
            Value::from(Some(Option::<bool>::None)),
            Value::Option(Some(Box::new(Value::Option(None))))
        );
    }

    #[test]
    fn seq() {
        assert_same::<Vec<f64>>("[1.0, 2.0, 3.0, 4.0]");

        assert_eq!(
            Value::from([-1_i8, 2, -3].as_slice()),
            Value::Seq(vec![
                Value::from(-1_i8),
                Value::from(2_i8),
                Value::from(-3_i8)
            ])
        );
        assert_eq!(
            Value::from(vec![-1_i8, 2, -3]),
            Value::Seq(vec![
                Value::from(-1_i8),
                Value::from(2_i8),
                Value::from(-3_i8)
            ])
        );
        assert_eq!(
            Value::from_iter([-1_i8, 2, -3]),
            Value::Seq(vec![
                Value::from(-1_i8),
                Value::from(2_i8),
                Value::from(-3_i8)
            ])
        );
    }

    #[test]
    fn unit() {
        assert_same::<()>("()");

        assert_eq!(Value::from(()), Value::Unit);
    }

    #[test]
    #[should_panic(expected = "Contract violation: value before key")]
    fn map_access_contract_violation() {
        struct BadVisitor;

        impl<'de> Visitor<'de> for BadVisitor {
            type Value = ();

            // GRCOV_EXCL_START
            fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.write_str("a map")
            }
            // GRCOV_EXCL_STOP

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                map.next_value::<()>()
            }
        }

        let value = Value::Map([("a", 42)].into_iter().collect());
        let _ = value.deserialize_map(BadVisitor);
    }

    #[test]
    fn transparent_value_newtype() {
        struct NewtypeDeserializer;

        impl<'de> Deserializer<'de> for NewtypeDeserializer {
            type Error = Error;

            fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
                visitor.visit_newtype_struct(serde::de::value::CharDeserializer::new('🦀'))
            }

            // GRCOV_EXCL_START
            forward_to_deserialize_any! {
                bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
                bytes byte_buf option unit unit_struct newtype_struct seq tuple
                tuple_struct map struct enum identifier ignored_any
            }

            #[cfg(feature = "integer128")]
            forward_to_deserialize_any! { i128 u128 }
            // GRCOV_EXCL_STOP
        }

        assert_eq!(
            Value::deserialize(NewtypeDeserializer).unwrap(),
            Value::from('🦀')
        );
    }
}
