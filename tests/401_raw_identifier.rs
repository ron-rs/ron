use ron::error::{Position, Span, SpannedError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Hello World")]
struct InvalidStruct;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "")]
struct EmptyStruct;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename = "Hello+World")]
#[serde(deny_unknown_fields)]
struct RawStruct {
    #[serde(rename = "ab.cd-ef")]
    field: bool,
    really_not_raw: i32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
enum RawEnum {
    #[serde(rename = "Hello-World")]
    RawVariant,
}

#[test]
fn test_invalid_identifiers() {
    let ser = ron::ser::to_string_pretty(
        &InvalidStruct,
        ron::ser::PrettyConfig::default().struct_names(true),
    );
    assert_eq!(
        ser,
        Err(ron::Error::InvalidIdentifier(String::from("Hello World")))
    );

    let ser = ron::ser::to_string_pretty(
        &EmptyStruct,
        ron::ser::PrettyConfig::default().struct_names(true),
    );
    assert_eq!(ser, Err(ron::Error::InvalidIdentifier(String::from(""))));

    let de = ron::from_str::<InvalidStruct>("Hello World").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::ExpectedDifferentStructName {
                expected: "Hello World",
                found: String::from("Hello"),
            },
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 6 },
            }
        }
    );

    let de = ron::from_str::<EmptyStruct>("").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::ExpectedUnit,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            }
        }
    );

    let de = ron::from_str::<EmptyStruct>("r#").unwrap_err();
    assert_eq!(
        format!("{}", de),
        "1:1: Expected only opening `(`, no name, for un-nameable struct"
    );

    let de = ron::from_str::<RawStruct>("").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::ExpectedNamedStructLike("Hello+World"),
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            }
        },
    );

    let de = ron::from_str::<RawStruct>("r#").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::ExpectedNamedStructLike("Hello+World"),
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            }
        },
    );

    let de = ron::from_str::<RawStruct>("Hello+World").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::SuggestRawIdentifier(String::from("Hello+World")),
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            }
        }
    );

    let de = ron::from_str::<RawStruct>(
        "r#Hello+World(
        ab.cd-ef: true,
    )",
    )
    .unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::SuggestRawIdentifier(String::from("ab.cd-ef")),
            span: Span {
                start: Position { line: 1, col: 15 },
                end: Position { line: 2, col: 9 },
            }
        }
    );

    let de = ron::from_str::<RawStruct>(
        "r#Hello+World(
        rab.cd-ef: true,
    )",
    )
    .unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::SuggestRawIdentifier(String::from("rab.cd-ef")),
            span: Span {
                start: Position { line: 1, col: 15 },
                end: Position { line: 2, col: 9 },
            }
        }
    );

    let de = ron::from_str::<RawStruct>(
        "r#Hello+World(
        r#ab.cd+ef: true,
    )",
    )
    .unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::NoSuchStructField {
                expected: &["ab.cd-ef", "really_not_raw"],
                found: String::from("ab.cd+ef"),
                outer: Some(String::from("Hello+World")),
            },
            span: Span {
                start: Position { line: 2, col: 11 },
                end: Position { line: 2, col: 19 },
            }
        }
    );

    let de = ron::from_str::<RawEnum>("Hello-World").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::SuggestRawIdentifier(String::from("Hello-World")),
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            }
        }
    );

    let de = ron::from_str::<RawEnum>("r#Hello+World").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: ron::Error::NoSuchEnumVariant {
                expected: &["Hello-World"],
                found: String::from("Hello+World"),
                outer: Some(String::from("RawEnum")),
            },
            span: Span {
                start: Position { line: 1, col: 3 },
                end: Position { line: 1, col: 14 },
            }
        }
    );

    let de = ron::from_str::<EmptyStruct>("r#+").unwrap_err();
    assert_eq!(
        format!("{}", de),
        r#"1:3-1:4: Expected only opening `(`, no name, for un-nameable struct"#,
    );
}

#[test]
fn test_raw_identifier_roundtrip() {
    let val = RawStruct {
        field: true,
        really_not_raw: 42,
    };

    let ser =
        ron::ser::to_string_pretty(&val, ron::ser::PrettyConfig::default().struct_names(true))
            .unwrap();
    assert_eq!(
        ser,
        "r#Hello+World(\n    r#ab.cd-ef: true,\n    really_not_raw: 42,\n)"
    );

    let de: RawStruct = ron::from_str(&ser).unwrap();
    assert_eq!(de, val);

    let val = RawEnum::RawVariant;

    let ser = ron::ser::to_string(&val).unwrap();
    assert_eq!(ser, "r#Hello-World");

    let de: RawEnum = ron::from_str(&ser).unwrap();
    assert_eq!(de, val);
}
