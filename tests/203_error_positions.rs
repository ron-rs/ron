use std::num::NonZeroU32;

use ron::error::{Error, Position, SpannedError};
use serde::de::{value::Error as DeError, Deserialize, IntoDeserializer};

#[derive(Debug, serde::Deserialize, PartialEq)]
enum Test {
    TupleVariant(i32, NonZeroU32),
    StructVariant { a: bool, b: NonZeroU32, c: i32 },
}

#[test]
fn test_error_positions() {
    assert_eq!(
        ron::from_str::<Test>("NotAVariant"),
        Err(SpannedError {
            code: Error::Message(String::from(
                "unknown variant `NotAVariant`, expected `TupleVariant` or `StructVariant`"
            )),
            position: Position { line: 1, col: 12 },
        })
    );

    assert_eq!(
        ron::from_str::<Test>("TupleVariant(1, 0)"),
        Err(SpannedError {
            code: Error::Message(
                NonZeroU32::deserialize(IntoDeserializer::<DeError>::into_deserializer(0_u32))
                    .unwrap_err()
                    .to_string()
            ),
            position: Position { line: 1, col: 18 },
        })
    );

    assert_eq!(
        ron::from_str::<Test>("StructVariant(a: true, b: 0, c: -42)"),
        Err(SpannedError {
            code: Error::Message(
                NonZeroU32::deserialize(IntoDeserializer::<DeError>::into_deserializer(0_u32))
                    .unwrap_err()
                    .to_string()
            ),
            position: Position { line: 1, col: 28 },
        })
    );

    assert_eq!(
        ron::from_str::<Test>("StructVariant(a: true, c: -42)"),
        Err(SpannedError {
            code: Error::Message(String::from("missing field `b`")),
            position: Position { line: 1, col: 30 },
        })
    );

    assert_eq!(
        ron::from_str::<Test>("StructVariant(a: true, b: 1, a: false, c: -42)"),
        Err(SpannedError {
            code: Error::Message(String::from("duplicate field `a`")),
            position: Position { line: 1, col: 31 },
        })
    );
}
