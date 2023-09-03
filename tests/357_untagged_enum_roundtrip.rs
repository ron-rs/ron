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
fn untagged_enum_not_a_list() {
    // Contributed by @obi1kenobi in https://github.com/ron-rs/ron/issues/357

    let value = MyValue::Enum(Enum::First("foo".to_string()));

    let ron = ron::to_string(&value).unwrap();
    assert_eq!(ron, "First(\"foo\")");

    let de = ron::from_str(&ron).unwrap();

    println!("{}", ron);

    // This used to fail as the value was deserialised as `List([String("foo")])`
    assert_eq!(value, de);
}
