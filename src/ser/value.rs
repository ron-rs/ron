use serde::ser::{Serialize, Serializer};

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
            Value::Number(Number::I8(v)) => serializer.serialize_i8(v),
            Value::Number(Number::I16(v)) => serializer.serialize_i16(v),
            Value::Number(Number::I32(v)) => serializer.serialize_i32(v),
            Value::Number(Number::I64(v)) => serializer.serialize_i64(v),
            #[cfg(feature = "integer128")]
            Value::Number(Number::I128(v)) => serializer.serialize_i128(v),
            Value::Number(Number::U8(v)) => serializer.serialize_u8(v),
            Value::Number(Number::U16(v)) => serializer.serialize_u16(v),
            Value::Number(Number::U32(v)) => serializer.serialize_u32(v),
            Value::Number(Number::U64(v)) => serializer.serialize_u64(v),
            #[cfg(feature = "integer128")]
            Value::Number(Number::U128(v)) => serializer.serialize_u128(v),
            Value::Number(Number::F32(v)) => serializer.serialize_f32(v.get()),
            Value::Number(Number::F64(v)) => serializer.serialize_f64(v.get()),
            Value::Option(Some(ref o)) => serializer.serialize_some(o.as_ref()),
            Value::Option(None) => serializer.serialize_none(),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Seq(ref s) => Serialize::serialize(s, serializer),
            Value::Unit => serializer.serialize_unit(),
        }
    }
}
