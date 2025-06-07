use ron::error::{Position, Span, SpannedError};

#[cfg(feature = "internal-span-substring-test")]
use ron::util::span_substring::check_error_span_inclusive;

#[cfg(feature = "internal-span-substring-test")]
use ron::util::span_substring::check_error_span_exclusive;

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
enum TestEnum {
    StructVariant { a: bool, b: char, c: i32 },
    NewtypeVariant(TestStruct),
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(tag = "type")]
enum TestEnumInternal {
    StructVariant { a: bool },
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(tag = "type", content = "content")]
enum TestEnumAdjacent {
    StructVariant { a: bool },
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(untagged)]
enum TestEnumUntagged {
    StructVariant { a: bool },
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
            code: ron::Error::NoSuchEnumVariant {
                expected: &["StructVariant", "NewtypeVariant"],
                found: String::from("NotAVariant"),
                outer: Some(String::from("TestEnum")),
            },
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 12 },
            }
        })
    );

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<TestEnum>(
        "NotAVariant",
        Err(SpannedError {
            code: ron::Error::NoSuchEnumVariant {
                expected: &["StructVariant", "NewtypeVariant"],
                found: String::from("NotAVariant"),
                outer: Some(String::from("TestEnum")),
            },
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 12 },
            },
        }),
        "NotAVariant",
    );
}

#[test]
fn test_struct_enum_fields() {
    let bogus_struct = "StructVariant(a: true, b: 'b', c: -42, d: \"gotcha\")";
    let expected_err = Err(SpannedError {
        code: ron::Error::NoSuchStructField {
            expected: &["a", "b", "c"],
            found: String::from("d"),
            outer: Some(String::from("StructVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 40 },
            end: Position { line: 1, col: 41 },
        },
    });

    assert_eq!(ron::from_str::<TestEnum>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TestEnum>(bogus_struct, expected_err, "d:");

    let bogus_struct = "StructVariant(a: true, c: -42)";
    let expected_err = Err(SpannedError {
        code: ron::Error::MissingStructField {
            field: "b",
            outer: Some(String::from("StructVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 30 },
            end: Position { line: 1, col: 30 },
        },
    });

    assert_eq!(ron::from_str::<TestEnum>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TestEnum>(bogus_struct, expected_err, ")");

    let bogus_struct = "StructVariant(a: true, b: 'b', a: false, c: -42)";
    let expected_err = Err(SpannedError {
        code: ron::Error::DuplicateStructField {
            field: "a",
            outer: Some(String::from("StructVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 32 },
            end: Position { line: 1, col: 33 },
        },
    });

    assert_eq!(ron::from_str::<TestEnum>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TestEnum>(bogus_struct, expected_err, "a:");
}

#[test]
fn test_newtype_enum_fields() {
    let bogus_struct = "#![enable(unwrap_variant_newtypes)] NewtypeVariant(a: true, b: 'b', c: -42, d: \"gotcha\")";
    let expected_err = Err(SpannedError {
        code: ron::Error::NoSuchStructField {
            expected: &["a", "b", "c"],
            found: String::from("d"),
            outer: Some(String::from("NewtypeVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 77 },
            end: Position { line: 1, col: 78 },
        },
    });

    assert_eq!(ron::from_str::<TestEnum>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TestEnum>(bogus_struct, expected_err, "d:");

    let bogus_struct = "#![enable(unwrap_variant_newtypes)] NewtypeVariant(a: true, c: -42)";
    let expected_err = Err(SpannedError {
        code: ron::Error::MissingStructField {
            field: "b",
            outer: Some(String::from("NewtypeVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 67 },
            end: Position { line: 1, col: 67 },
        },
    });

    assert_eq!(ron::from_str::<TestEnum>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TestEnum>(bogus_struct, expected_err, ")");

    let bogus_struct =
        "#![enable(unwrap_variant_newtypes)] NewtypeVariant(a: true, b: 'b', a: false, c: -42)";
    let expected_err = Err(SpannedError {
        code: ron::Error::DuplicateStructField {
            field: "a",
            outer: Some(String::from("NewtypeVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 69 },
            end: Position { line: 1, col: 70 },
        },
    });

    assert_eq!(ron::from_str::<TestEnum>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TestEnum>(bogus_struct, expected_err, "a:");
}

#[test]
fn test_struct_fields() {
    let bogus_struct = "TestStruct(a: true, b: 'b', c: -42, d: \"gotcha\")";
    let expected_err = Err(SpannedError {
        code: ron::Error::NoSuchStructField {
            expected: &["a", "b", "c"],
            found: String::from("d"),
            outer: Some(String::from("TestStruct")),
        },
        span: Span {
            start: Position { line: 1, col: 37 },
            end: Position { line: 1, col: 38 },
        },
    });

    assert_eq!(ron::from_str::<TestStruct>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TestStruct>(bogus_struct, expected_err, "d:");

    assert_eq!(
        ron::from_str::<TestStruct>("TestStruct(a: true, c: -42)"),
        Err(SpannedError {
            code: ron::Error::MissingStructField {
                field: "b",
                outer: Some(String::from("TestStruct")),
            },
            span: Span {
                start: Position { line: 1, col: 27 },
                end: Position { line: 1, col: 27 },
            }
        })
    );

    let bogus_struct = "TestStruct(a: true, b: 'b', a: false, c: -42)";
    let expected_err = Err(SpannedError {
        code: ron::Error::DuplicateStructField {
            field: "a",
            outer: Some(String::from("TestStruct")),
        },
        span: Span {
            start: Position { line: 1, col: 29 },
            end: Position { line: 1, col: 30 },
        },
    });

    assert_eq!(ron::from_str::<TestStruct>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TestStruct>(bogus_struct, expected_err, "a:");
}

#[test]
fn test_internally_tagged_enum() {
    // Note: Not extracting the variant type is not great,
    //        but at least not wrong either
    //       Since the error occurs in serde-generated user code,
    //        after successfully deserialising, we cannot annotate

    let bogus_struct = "(type: \"StructVariant\")";
    let expected_err = Err(SpannedError {
        code: ron::Error::MissingStructField {
            field: "a",
            outer: None,
        },
        span: Span {
            start: Position { line: 1, col: 23 },
            end: Position { line: 1, col: 24 },
        },
    });

    assert_eq!(
        ron::from_str::<TestEnumInternal>(bogus_struct),
        expected_err
    );

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<TestEnumInternal>(bogus_struct, expected_err, ")");
}

#[test]
fn test_adjacently_tagged_enum() {
    // Note: TestEnumAdjacent makes sense here since we are now treating
    //        the enum as a struct

    assert_eq!(
        ron::from_str::<TestEnumAdjacent>("(type: StructVariant, content: (d: 4))"),
        Err(SpannedError {
            code: ron::Error::MissingStructField {
                field: "a",
                outer: Some(String::from("TestEnumAdjacent")),
            },
            span: Span {
                start: Position { line: 1, col: 37 },
                end: Position { line: 1, col: 37 },
            }
        })
    );
}

#[test]
fn test_untagged_enum() {
    // Note: Errors inside untagged enums are not bubbled up

    let bogus_struct = "(a: true, a: false)";
    let expected_err = Err(SpannedError {
        code: ron::Error::Message(String::from(
            "data did not match any variant of untagged enum TestEnumUntagged",
        )),
        span: Span {
            start: Position { line: 1, col: 19 },
            end: Position { line: 1, col: 20 },
        },
    });

    assert_eq!(
        ron::from_str::<TestEnumUntagged>(bogus_struct),
        expected_err
    );

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<TestEnumUntagged>(bogus_struct, expected_err, ")");
}
