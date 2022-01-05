use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum MyValue {
    Int(i64),
    String(String),
    Enum(Enum),
    List(Vec<MyValue>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Enum {
    First(String),
    Second(i64),
}

#[test]
fn test_untagged_roundtrip() {
    let value = MyValue::Enum(Enum::First(String::from("foo")));

    let serialized = ron::to_string(&value).unwrap();
    assert_eq!(serialized, r#"First("foo")"#);

    let inner_deserialized: Enum = ron::from_str(&serialized).unwrap();
    assert_eq!(inner_deserialized, Enum::First(String::from("foo")));

    let deserialized: MyValue = ron::from_str(&serialized).unwrap();
    assert_eq!(deserialized, value);
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Newtype(i8);

#[test]
fn test_value_newtype() {
    let value = Newtype(-128);

    let serialized = ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(true)).unwrap();
    assert_eq!(serialized, r#"Newtype(-128)"#);

    let typed_deserialized: Newtype = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(value_deserialized, ron::Value::Map(
        vec![(
            ron::Value::String(String::from("Newtype")),
            ron::Value::Number(ron::value::Number::from(-128)),
        )].into_iter().collect()
    ));

    let untyped_deserialized: Newtype = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}
