use std::num::NonZeroU32;

use ron::error::{Error, ErrorCode, Position};

#[derive(Debug, serde::Deserialize, PartialEq)]
enum Test {
    TupleVariant(i32, NonZeroU32),
    StructVariant { a: bool, b: NonZeroU32, c: i32 },
}

#[test]
fn test_error_positions() {
    assert_eq!(
        ron::from_str::<Test>("NotAVariant"),
        Err(Error {
            code: ErrorCode::Message(String::from(
                "unknown variant `NotAVariant`, expected `TupleVariant` or `StructVariant`"
            )),
            position: Position { line: 1, col: 12 },
        })
    );

    assert_eq!(
        ron::from_str::<Test>("TupleVariant(1, 0)"),
        Err(Error {
            code: ErrorCode::Message(String::from("expected a non-zero value")),
            position: Position { line: 1, col: 18 },
        })
    );

    assert_eq!(
        ron::from_str::<Test>("StructVariant(a: true, b: 0, c: -42)"),
        Err(Error {
            code: ErrorCode::Message(String::from("expected a non-zero value")),
            position: Position { line: 1, col: 28 },
        })
    );

    assert_eq!(
        ron::from_str::<Test>("StructVariant(a: true, c: -42)"),
        Err(Error {
            code: ErrorCode::Message(String::from("missing field `b`")),
            position: Position { line: 1, col: 30 },
        })
    );

    assert_eq!(
        ron::from_str::<Test>("StructVariant(a: true, b: 1, a: false, c: -42)"),
        Err(Error {
            code: ErrorCode::Message(String::from("duplicate field `a`")),
            position: Position { line: 1, col: 31 },
        })
    );
}
