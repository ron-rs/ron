use serde::ser::{Serialize, SerializeStruct, SerializeTuple, SerializeTupleStruct, Serializer};

use crate::value::Value;

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Char(c) => serializer.serialize_char(*c),
            Value::Map(m) => Serialize::serialize(m, serializer),
            Value::Number(number) => Serialize::serialize(number, serializer),
            Value::Option(Some(o)) => serializer.serialize_some(o.as_ref()),
            Value::Option(None) => serializer.serialize_none(),
            Value::String(s) => serializer.serialize_str(s),
            Value::Bytes(b) => serializer.serialize_bytes(b),
            Value::List(s) => Serialize::serialize(s, serializer),
            Value::Unit => serializer.serialize_unit(),
            Value::NamedStruct(name, map) => {
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
                    std::borrow::Cow::Borrowed(a) => {
                        let mut state = serializer.serialize_struct(a, map.len())?;

                        for (k, v) in &map.0 {
                            match k {
                                std::borrow::Cow::Borrowed(a) => {
                                    state.serialize_field(a, &v)?;
                                }
                                std::borrow::Cow::Owned(_) => todo!(),
                            }
                        }

                        state.end()
                    }
                    std::borrow::Cow::Owned(_) => todo!(),
                }
            }
            Value::Tuple(vec) => {
                let mut state = serializer.serialize_tuple(vec.len())?;

                for v in vec {
                    state.serialize_element(&v)?;
                }

                state.end()
            }
            _ => todo!(),
        }
    }
}
