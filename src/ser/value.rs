use serde::ser::{Serialize, SerializeStruct, SerializeTuple, SerializeTupleStruct, Serializer};

use crate::value::{Number, Value};

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Char(c) => serializer.serialize_char(c),
            Value::Map(ref m) => Serialize::serialize(m, serializer),
            Value::Number(Number::Float(ref f)) => serializer.serialize_f64(f.get()),
            Value::Number(Number::Integer(i)) => serializer.serialize_i64(i),
            Value::Option(Some(ref o)) => serializer.serialize_some(o.as_ref()),
            Value::Option(None) => serializer.serialize_none(),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Seq(ref s) => Serialize::serialize(s, serializer),
            Value::Unit => serializer.serialize_unit(),
            Value::NamedUnit { name } => serializer.serialize_unit_struct(name),
            Value::Tuple(ref fields) => {
                let mut tuple = serializer.serialize_tuple(fields.len())?;

                for field in fields {
                    tuple.serialize_element(field)?;
                }

                tuple.end()
            }
            Value::TupleStructLike { name, ref fields } => {
                let mut tuple = serializer.serialize_tuple_struct(name, fields.len())?;

                for field in fields {
                    tuple.serialize_field(field)?;
                }

                tuple.end()
            }
            Value::StructLike { name, ref fields } => {
                let /*mut*/ r#struct = serializer.serialize_struct(name, fields.len())?;

                // TODO: the field names have to be interned static str's as well

                // for (key, value) in fields.iter() {
                //     r#struct.serialize_field(key, value)?;
                // }

                r#struct.end()
            }
        }
    }
}
