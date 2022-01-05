use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum MyValue {
    Int(i64),
    String(String),
    Enum(Enum),
    List(Vec<MyValue>),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Enum {
    First(String),
    Second(i64),
}

#[test]
fn test() {
    let value = MyValue::Enum(Enum::First(String::from("foo")));

    let serialized = ron::to_string(&value).unwrap();
    assert_eq!(serialized, r#"First("foo")"#);

    let inner_deserialized: Enum = ron::from_str(&serialized).unwrap();
    assert_eq!(inner_deserialized, Enum::First(String::from("foo")));

    let deserialized: MyValue = ron::from_str(&serialized).unwrap();
    assert_eq!(deserialized, value);
}
