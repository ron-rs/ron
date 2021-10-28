use std::io;

use serde::ser::{Serialize, SerializeMap, SerializeSeq, SerializeStruct, Serializer};

use crate::value::{Number, Value};
use crate::{Error, Map};

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Char(c) => serializer.serialize_char(c),
            Value::Map(ref m) => Serialize::serialize(m, serializer),
            Value::Struct(ref s) => Serialize::serialize(
                &Map(s
                    .fields
                    .iter()
                    .map(|(k, v)| (Value::String(k.clone()), v.clone()))
                    .collect()),
                serializer,
            ),
            Value::Number(Number::Float(ref f)) => serializer.serialize_f64(f.get()),
            Value::Number(Number::Integer(i)) => serializer.serialize_i64(i),
            Value::Option(Some(ref o)) => serializer.serialize_some(o.as_ref()),
            Value::Option(None) => serializer.serialize_none(),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Seq(ref s) => Serialize::serialize(s, serializer),
            Value::Unit => serializer.serialize_unit(),
        }
    }
}

impl Value {
    pub fn enhanced_serialize<W: io::Write>(
        &self,
        serializer: &mut crate::ser::Serializer<W>,
    ) -> Result<(), Error> {
        match *self {
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Char(c) => serializer.serialize_char(c),
            Value::Map(ref m) => {
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m.iter() {
                    map.serialize_key_dyn(k)?;
                    map.serialize_value_dyn(v)?;
                }
                SerializeMap::end(map)
            }
            Value::Struct(ref s) => {
                let mut c = serializer
                    .serialize_struct_dyn(s.name.as_deref().unwrap_or(""), s.fields.len())?;
                for (field, value) in s.iter() {
                    c.serialize_field_dyn(field.as_ref(), value)?;
                }
                SerializeStruct::end(c)
            }
            Value::Number(Number::Float(ref f)) => serializer.serialize_f64(f.get()),
            Value::Number(Number::Integer(i)) => serializer.serialize_i64(i),
            Value::Option(Some(ref o)) => serializer.serialize_some(o.as_ref()),
            Value::Option(None) => serializer.serialize_none(),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Seq(ref s) => {
                let mut seq = serializer.serialize_seq(Some(s.len()))?;
                for v in s.iter() {
                    seq.serialize_element_dyn(v)?;
                }
                SerializeSeq::end(seq)
            }
            Value::Unit => serializer.serialize_unit(),
        }
    }
}
