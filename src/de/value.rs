use std::collections::BTreeMap;
use std::cmp::Eq;
use std::fmt;
use std::hash::{Hash, Hasher};

use serde::de::*;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Field {
    pub name: String,
    pub value: Value,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Number(f64);

impl Number {
    /// Panics if `f` is not finite.
    pub fn new(f: f64) -> Self {
        assert!(f.is_finite());

        Number(f)
    }

    pub fn into_float(self) -> f64 {
        self.into()
    }
}

impl Into<f64> for Number {
    fn into(self) -> f64 {
        self.0
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i64(self.0 as i64);
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Value {
    Boolean(bool),
    Byte(u8),
    Char(char),
    List(Vec<Value>),
    Option(Option<Box<Value>>),
    Map(BTreeMap<Value, Value>),
    Number(Number),
    String(String),
    Struct(Option<String>, Vec<Field>),
    Tuple(Vec<Value>),
    Unit,
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

pub struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a RON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where E: Error,
    {
        Ok(Value::Boolean(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
        where E: Error,
    {
        unimplemented!()
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where E: Error,
    {
        Ok(Value::Number(Number::new(v)))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
        where E: Error,
    {
        Ok(Value::Char(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: Error,
    {
        Ok(Value::String(v.to_string()))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
        where E: Error,
    {
        self.visit_str(v)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where E: Error,
    {
        Ok(Value::String(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where E: Error,
    {
        Ok(Value::List(v.iter().map(|b| Value::Byte(*b)).collect()))
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
        where E: Error,
    {
        self.visit_bytes(v)
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where E: Error,
    {
        self.visit_bytes(v.as_slice())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
        where E: Error,
    {
        Ok(Value::Option(None))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: Deserializer<'de>,
    {
        Ok(Value::Option(Some(Box::new(Deserialize::deserialize(deserializer)?))))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
        where E: Error,
    {
        Ok(Value::Unit)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        while let Some(elem) = seq.next_element()? {
            vec.push(elem);
        }

        Ok(Value::List(vec))
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where A: MapAccess<'de>,
    {

    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error> where
        A: EnumAccess<'de>, {
        unimplemented!()
    }
}
