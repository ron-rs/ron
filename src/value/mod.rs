//! Value module.

use std::{cmp::Eq, hash::Hash};

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
        let value = T::deserialize(value).unwrap();

        assert_eq!(direct, value, "Deserialization for {:?} is not the same", s);
    }

    fn assert_same_bytes<'de, T>(s: &'de [u8])
    where
        T: Debug + Deserialize<'de> + PartialEq,
    {
        use crate::de::from_bytes;

        let direct: T = from_bytes(s).unwrap();
        let value: Value = from_bytes(s).unwrap();
        let value = T::deserialize(value).unwrap();

        assert_eq!(direct, value, "Deserialization for {:?} is not the same", s);
    }

    #[test]
    fn boolean() {
        assert_same::<bool>("true");
        assert_same::<bool>("false");
    }

    #[test]
    fn float() {
        assert_same::<f64>("0.123");
        assert_same::<f64>("-4.19");
    }

    #[test]
    fn int() {
        assert_same::<u32>("626");
        assert_same::<i32>("-50");
    }

    #[test]
    fn char() {
        assert_same::<char>("'4'");
        assert_same::<char>("'c'");
    }

    #[test]
    fn string() {
        assert_same::<String>(r#""hello world""#);
        assert_same::<String>(r#""this is a Rusty 🦀 string""#);
        assert_same::<String>(r#""this is now valid UTF-8 \xf0\x9f\xa6\x80""#);
    }

    #[test]
    fn bytes() {
        assert_same_bytes::<serde_bytes::ByteBuf>(br#"b"hello world""#);
        assert_same_bytes::<serde_bytes::ByteBuf>(
            br#"b"this is not valid UTF-8 \xf8\xa1\xa1\xa1\xa1""#,
        );
    }

    #[test]
    fn map() {
        assert_same::<BTreeMap<char, String>>(
            "{
'a': \"Hello\",
'b': \"Bye\",
        }",
        );
    }

    #[test]
    fn option() {
        assert_same::<Option<char>>("Some('a')");
        assert_same::<Option<char>>("None");
    }

    #[test]
    fn seq() {
        assert_same::<Vec<f64>>("[1.0, 2.0, 3.0, 4.0]");
    }

    #[test]
    fn unit() {
        assert_same::<()>("()");
    }
}
