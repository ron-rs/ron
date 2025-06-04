#![allow(dead_code)]

use ron::error::{Error, Position, Span, SpannedError};

#[cfg(feature = "internal-span-substring-test")]
use ron::util::span_substring::check_error_span_inclusive;

#[derive(Debug, serde::Deserialize)]
struct Test {
    a: i32,
    b: i32,
}

#[test]
fn test_missing_comma_error() {
    let tuple_string = r#"(
        1 // <-- forgotten comma here
        2
    )"#;

    assert_eq!(
        ron::from_str::<(i32, i32)>(tuple_string).unwrap_err(),
        SpannedError {
            code: Error::ExpectedComma,
            span: Span {
                start: Position { line: 3, col: 9 },
                end: Position { line: 3, col: 9 }
            }
        }
    );

    let list_string = r#"[
        0,
        1 // <-- forgotten comma here
        2
    ]"#;

    assert_eq!(
        ron::from_str::<Vec<i32>>(list_string).unwrap_err(),
        SpannedError {
            code: Error::ExpectedComma,
            span: Span {
                start: Position { line: 4, col: 9 },
                end: Position { line: 4, col: 9 }
            }
        }
    );

    let struct_string = r#"Test(
        a: 1 // <-- forgotten comma here
        b: 2
    )"#;

    assert_eq!(
        ron::from_str::<Test>(struct_string).unwrap_err(),
        SpannedError {
            code: Error::ExpectedComma,
            span: Span {
                start: Position { line: 3, col: 9 },
                end: Position { line: 3, col: 9 }
            }
        }
    );

    let map_string = r#"{
        "a": 1 // <-- forgotten comma here
        "b": 2
    }"#;

    assert_eq!(
        ron::from_str::<std::collections::HashMap<String, i32>>(map_string).unwrap_err(),
        SpannedError {
            code: Error::ExpectedComma,
            span: Span {
                start: Position { line: 3, col: 9 },
                end: Position { line: 3, col: 9 }
            }
        }
    );

    let extensions_string = r#"#![enable(
        implicit_some // <-- forgotten comma here
        unwrap_newtypes
    ]) 42"#;

    assert_eq!(
        ron::from_str::<u8>(extensions_string).unwrap_err(),
        SpannedError {
            code: Error::ExpectedComma,
            span: Span {
                start: Position { line: 2, col: 50 },
                end: Position { line: 3, col: 9 }
            }
        }
    );

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<u8>(
        extensions_string,
        Err(SpannedError {
            code: Error::ExpectedComma,
            span: Span {
                start: Position { line: 2, col: 50 },
                end: Position { line: 3, col: 9 },
            },
        }),
        "\n        u",
    );
}

#[test]
fn test_comma_end() {
    assert_eq!(ron::from_str::<(i32, i32)>("(0, 1)").unwrap(), (0, 1));
    assert_eq!(ron::from_str::<(i32, i32)>("(0, 1,)").unwrap(), (0, 1));
    assert_eq!(ron::from_str::<()>("()"), Ok(()));
}
