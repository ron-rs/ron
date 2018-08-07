use serde::ser::{Serialize, Serializer};

use value::Value;

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Bool(b) => serializer.serialize_bool(b),
            Value::Char(c) => serializer.serialize_char(c),
            Value::Map(ref m) => Serialize::serialize(m, serializer),
            Value::Number(ref n) => serializer.serialize_f64(n.get()),
            Value::Option(Some(ref o)) => serializer.serialize_some(o.as_ref()),
            Value::Option(None) => serializer.serialize_none(),
            Value::String(ref s) => serializer.serialize_str(s),
            Value::Seq(ref s) => Serialize::serialize(s, serializer),
            Value::Unit => serializer.serialize_unit(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use de::from_str;
    use ser::to_string;

    #[test]
    fn check_tuple_het() {
        let s = to_string(&Value::Seq(vec![
            Value::Bool(false),
            Value::Char('c'),
        ])).unwrap();

        assert_eq!(from_str::<(bool, char)>(&s).unwrap(), (false, 'c'));
    }

    #[test]
    fn check_list_stays_list() {
        let s = to_string(&Value::Seq(vec![
            Value::Bool(false),
            Value::Bool(true),
        ])).unwrap();

        assert_eq!(from_str::<Vec<bool>>(&s).unwrap(), vec![false, true]);
    }
}
