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
fn test_value_newtype_struct_unnamed() {
    let value = Newtype(-128);

    let serialized =
        ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(false))
            .unwrap();
    assert_eq!(serialized, "(-128)");

    let typed_deserialized: Newtype = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Seq(
            vec![ron::Value::Number(ron::value::Number::from(-128)),]
                .into_iter()
                .collect()
        )
    );

    let untyped_deserialized: Newtype = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}

#[test]
fn test_value_newtype_struct_named() {
    let value = Newtype(-128);

    let serialized =
        ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(true))
            .unwrap();
    assert_eq!(serialized, "Newtype(-128)");

    let typed_deserialized: Newtype = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Map(
            vec![(
                ron::Value::String(String::from("Newtype")),
                ron::Value::Number(ron::value::Number::from(-128)),
            )]
            .into_iter()
            .collect()
        )
    );

    let untyped_deserialized: Newtype = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);

    // TODO: test if trailing comma otherwise allowed in newtype structs + variants
    // TODO: test if normal struct without name still works
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Struct {
    a: u8,
}

#[test]
fn test_value_struct_unnamed() {
    let value = Struct { a: 42 };

    let serialized =
        ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(false))
            .unwrap();
    assert_eq!(serialized, "(\n    a: 42,\n)");

    let typed_deserialized: Struct = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Map(
            vec![(
                ron::Value::String(String::from("a")),
                ron::Value::Number(ron::value::Number::from(42)),
            )]
            .into_iter()
            .collect()
        )
    );

    let untyped_deserialized: Struct = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}

#[test]
fn test_value_struct_named() {
    let value = Struct { a: 42 };

    let serialized =
        ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(true))
            .unwrap();
    assert_eq!(serialized, "Struct(\n    a: 42,\n)");

    let typed_deserialized: Struct = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Map(
            vec![(
                ron::Value::String(String::from("Struct")),
                ron::Value::Map(
                    vec![(
                        ron::Value::String(String::from("a")),
                        ron::Value::Number(ron::value::Number::from(42)),
                    )]
                    .into_iter()
                    .collect()
                ),
            )]
            .into_iter()
            .collect()
        )
    );

    let untyped_deserialized: Struct = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Unit;

#[test]
fn test_value_unit_struct_unnamed() {
    let value = Unit;

    let serialized =
        ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(false))
            .unwrap();
    assert_eq!(serialized, "()");

    let typed_deserialized: Unit = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(value_deserialized, ron::Value::Unit);

    let untyped_deserialized: Unit = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}

#[test]
fn test_value_unit_struct_named() {
    let value = Unit;

    let serialized =
        ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(true))
            .unwrap();
    assert_eq!(serialized, "Unit");

    let typed_deserialized: Unit = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    // let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    // assert_eq!(value_deserialized, ron::Value::String(String::from("Unit")));

    // let untyped_deserialized: Unit = value_deserialized.into_rust().unwrap();
    // assert_eq!(untyped_deserialized, value);
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Tuple(i8, bool);

#[test]
fn test_value_tuple_struct_unnamed() {
    let value = Tuple(42, false);

    let serialized =
        ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(false))
            .unwrap();
    assert_eq!(serialized, "(42, false)");

    let typed_deserialized: Tuple = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Seq(vec![
            ron::Value::Number(ron::value::Number::from(42)),
            ron::Value::Bool(false),
        ])
    );

    let untyped_deserialized: Tuple = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}

#[test]
fn test_value_tuple_struct_named() {
    let value = Tuple(42, false);

    let serialized =
        ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new().struct_names(true))
            .unwrap();
    assert_eq!(serialized, "Tuple(42, false)");

    let typed_deserialized: Tuple = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Map(
            vec![(
                ron::Value::String(String::from("Tuple")),
                ron::Value::Seq(vec![
                    ron::Value::Number(ron::value::Number::from(42)),
                    ron::Value::Bool(false),
                ]),
            )]
            .into_iter()
            .collect()
        )
    );

    let untyped_deserialized: Tuple = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum AnyEnum {
    Unit,
    Newtype(i8),
    Tuple(i8, bool),
    Struct { a: i8 },
}

#[test]
fn test_value_unit_variant() {
    let value = AnyEnum::Unit;

    let serialized = ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new()).unwrap();
    assert_eq!(serialized, "Unit");

    let typed_deserialized: AnyEnum = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    // let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    // assert_eq!(value_deserialized, ron::Value::String(String::from("Unit")));

    // let untyped_deserialized: AnyEnum = value_deserialized.into_rust().unwrap();
    // assert_eq!(untyped_deserialized, value);
}

#[test]
fn test_value_newtype_variant() {
    let value = AnyEnum::Newtype(-42);

    let serialized = ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new()).unwrap();
    assert_eq!(serialized, "Newtype(-42)");

    let typed_deserialized: AnyEnum = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Map(
            vec![(
                ron::Value::String(String::from("Newtype")),
                ron::Value::Number(ron::value::Number::from(-42)),
            )]
            .into_iter()
            .collect()
        )
    );

    let untyped_deserialized: AnyEnum = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}

#[test]
fn test_value_tuple_variant() {
    let value = AnyEnum::Tuple(-42, true);

    let serialized = ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new()).unwrap();
    assert_eq!(serialized, "Tuple(-42, true)");

    let typed_deserialized: AnyEnum = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Map(
            vec![(
                ron::Value::String(String::from("Tuple")),
                ron::Value::Seq(vec![
                    ron::Value::Number(ron::value::Number::from(-42)),
                    ron::Value::Bool(true),
                ]),
            )]
            .into_iter()
            .collect()
        )
    );

    let untyped_deserialized: AnyEnum = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}

#[test]
fn test_value_struct_variant() {
    let value = AnyEnum::Struct { a: 24 };

    let serialized = ron::ser::to_string_pretty(&value, ron::ser::PrettyConfig::new()).unwrap();
    assert_eq!(serialized, "Struct(\n    a: 24,\n)");

    let typed_deserialized: AnyEnum = ron::from_str(&serialized).unwrap();
    assert_eq!(typed_deserialized, value);

    let value_deserialized: ron::Value = ron::from_str(&serialized).unwrap();
    assert_eq!(
        value_deserialized,
        ron::Value::Map(
            vec![(
                ron::Value::String(String::from("Struct")),
                ron::Value::Map(
                    vec![(
                        ron::Value::String(String::from("a")),
                        ron::Value::Number(ron::value::Number::from(24)),
                    )]
                    .into_iter()
                    .collect()
                ),
            )]
            .into_iter()
            .collect()
        )
    );

    let untyped_deserialized: AnyEnum = value_deserialized.into_rust().unwrap();
    assert_eq!(untyped_deserialized, value);
}
