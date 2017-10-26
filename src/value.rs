//! Value module.

use std::cmp::{Eq, Ordering};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

use de::{Error as RonError, Result};

/// A wrapper for `f64` which guarantees that the inner value
/// is finite and thus implements `Eq`, `Hash` and `Ord`.
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub struct Number(f64);

impl Number {
    /// Panics if `v` is not a real number
    /// (infinity, NaN, ..).
    pub fn new(v: f64) -> Self {
        if !v.is_finite() {
            panic!("Tried to create Number with a NaN / infinity");
        }

        Number(v)
    }

    /// Returns the wrapped float.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0 as u64);
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("Bug: Contract violation")
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Char(char),
    Map(BTreeMap<Value, Value>),
    Number(Number),
    Option(Option<Box<Value>>),
    String(String),
    Seq(Vec<Value>),
    Unit,
}

/// Deserializer implementation for RON `Value`.
/// This does not support enums (because `Value` doesn't store them).
impl<'de> Deserializer<'de> for Value {
    type Error = RonError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Bool(b) => visitor.visit_bool(b),
            Value::Char(c) => visitor.visit_char(c),
            Value::Map(m) => visitor.visit_map(Map {
                keys: m.keys().cloned().collect(),
                values: m.values().cloned().collect(),
            }),
            Value::Number(n) => visitor.visit_f64(n.get()),
            Value::Option(Some(o)) => visitor.visit_some(*o),
            Value::Option(None) => visitor.visit_none(),
            Value::String(s) => visitor.visit_string(s),
            Value::Seq(seq) => visitor.visit_seq(Seq { seq }),
            Value::Unit => visitor.visit_unit(),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct Map {
    keys: Vec<Value>,
    values: Vec<Value>,
}

impl<'de> MapAccess<'de> for Map {
    type Error = RonError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        self.keys
            .drain(..)
            .next()
            .map_or(Ok(None), |v| seed.deserialize(v).map(Some))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.values
            .drain(..)
            .next()
            .map(|v| seed.deserialize(v))
            .expect("Contract violation")
    }
}

struct Seq {
    seq: Vec<Value>,
}

impl<'de> SeqAccess<'de> for Seq {
    type Error = RonError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        self.seq
            .drain(..)
            .next()
            .map_or(Ok(None), |v| seed.deserialize(v).map(Some))
    }
}
