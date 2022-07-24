use ron::error::{Error, Position, SpannedError};

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
enum TestEnum {
    StructVariant { a: bool, b: char, c: i32 },
    NewtypeVariant(TestStruct),
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct TestStruct {
    a: bool,
    b: char,
    c: i32,
}

#[test]
fn test_unknown_enum_variant() {
    assert_eq!(
        ron::from_str::<TestEnum>("NotAVariant"),
        Err(SpannedError {
            code: Error::NoSuchEnumVariant {
                expected: &["StructVariant", "NewtypeVariant"],
                found: String::from("NotAVariant"),
                outer: Some(String::from("TestEnum")),
            },
            position: Position { line: 1, col: 12 },
        })
    );
}

#[test]
fn test_struct_enum_fields() {
    assert_eq!(
        ron::from_str::<TestEnum>("StructVariant(a: true, b: 'b', c: -42, d: \"gotcha\")"),
        Err(SpannedError {
            code: Error::NoSuchStructField {
                expected: &["a", "b", "c"],
                found: String::from("d"),
                outer: Some(String::from("StructVariant")),
            },
            position: Position { line: 1, col: 41 },
        })
    );

    assert_eq!(
        ron::from_str::<TestEnum>("StructVariant(a: true, c: -42)"),
        Err(SpannedError {
            code: Error::MissingStructField {
                field: "b",
                outer: Some(String::from("StructVariant")),
            },
            position: Position { line: 1, col: 30 },
        })
    );

    assert_eq!(
        ron::from_str::<TestEnum>("StructVariant(a: true, b: 'b', a: false, c: -42)"),
        Err(SpannedError {
            code: Error::DuplicateStructField {
                field: "a",
                outer: Some(String::from("StructVariant")),
            },
            position: Position { line: 1, col: 33 },
        })
    );
}

#[test]
fn test_newtype_enum_fields() {
    assert_eq!(
        ron::from_str::<TestEnum>("#![enable(unwrap_variant_newtypes)] NewtypeVariant(a: true, b: 'b', c: -42, d: \"gotcha\")"),
        Err(SpannedError {
            code: Error::NoSuchStructField {
                expected: &["a", "b", "c"],
                found: String::from("d"),
                outer: Some(String::from("NewtypeVariant")),
            },
            position: Position { line: 1, col: 78 },
        })
    );

    assert_eq!(
        ron::from_str::<TestEnum>(
            "#![enable(unwrap_variant_newtypes)] NewtypeVariant(a: true, c: -42)"
        ),
        Err(SpannedError {
            code: Error::MissingStructField {
                field: "b",
                outer: Some(String::from("NewtypeVariant")),
            },
            position: Position { line: 1, col: 67 },
        })
    );

    assert_eq!(
        ron::from_str::<TestEnum>(
            "#![enable(unwrap_variant_newtypes)] NewtypeVariant(a: true, b: 'b', a: false, c: -42)"
        ),
        Err(SpannedError {
            code: Error::DuplicateStructField {
                field: "a",
                outer: Some(String::from("NewtypeVariant")),
            },
            position: Position { line: 1, col: 70 },
        })
    );
}

#[test]
fn test_struct_fields() {
    assert_eq!(
        ron::from_str::<TestStruct>("TestStruct(a: true, b: 'b', c: -42, d: \"gotcha\")"),
        Err(SpannedError {
            code: Error::NoSuchStructField {
                expected: &["a", "b", "c"],
                found: String::from("d"),
                outer: Some(String::from("TestStruct")),
            },
            position: Position { line: 1, col: 38 },
        })
    );

    assert_eq!(
        ron::from_str::<TestStruct>("TestStruct(a: true, c: -42)"),
        Err(SpannedError {
            code: Error::MissingStructField {
                field: "b",
                outer: Some(String::from("TestStruct")),
            },
            position: Position { line: 1, col: 27 },
        })
    );

    assert_eq!(
        ron::from_str::<TestStruct>("TestStruct(a: true, b: 'b', a: false, c: -42)"),
        Err(SpannedError {
            code: Error::DuplicateStructField {
                field: "a",
                outer: Some(String::from("TestStruct")),
            },
            position: Position { line: 1, col: 30 },
        })
    );
}
