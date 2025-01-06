use serde::ser::{Serialize, SerializeStruct, SerializeTuple, SerializeTupleStruct, Serializer};

use crate::value::Value;

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Char(c) => serializer.serialize_char(c),
            Value::Map(ref m) => Serialize::serialize(m, serializer),
            Value::Number(ref number) => Serialize::serialize(number, serializer),
            Value::Option(Some(ref o)) => serializer.serialize_some(o.as_ref()),
            Value::Option(None) => serializer.serialize_none(),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Bytes(ref b) => serializer.serialize_bytes(b),
            Value::List(ref s) => Serialize::serialize(s, serializer),
            Value::Unit => serializer.serialize_unit(),
            Value::Struct(name, map) => {
                // serializer.serialize_struct(name, len)
                // serializer.serialize_struct_variant(name, variant_index, variant, len)

                // serializer.serialize_newtype_struct(name, value)
                // serializer.serialize_newtype_variant(name, variant_index, variant, value)

                // serializer.serialize_unit_struct(name)
                // serializer.serialize_unit_variant(name, variant_index, variant)

                // serializer.serialize_map(len)

                // serializer.serialize_tuple(len)
                // serializer.serialize_tuple_struct(name, len)
                // serializer.serialize_tuple_variant(name, variant_index, variant, len)

                // https://github.com/serde-rs/json/blob/master/src/value/ser.rs

                match name {
                    Some(name) => {
                        let mut state = serializer.serialize_struct("", map.len())?;

                        for (k, v) in map {
                            state.serialize_field(&k, &v)?;
                        }

                        state.end()
                    }
                    None => {
                        todo!()
                    }
                }
            }
            Value::Tuple(name, vec) => match name {
                Some(name) => {
                    let mut state = serializer.serialize_tuple_struct("", vec.len())?;

                    for v in vec {
                        state.serialize_field(&v)?;
                    }

                    state.end()
                }
                None => {
                    let mut state = serializer.serialize_tuple(vec.len())?;

                    for v in vec {
                        state.serialize_element(&v)?;
                    }

                    state.end()
                }
            },
        }
    }
}
