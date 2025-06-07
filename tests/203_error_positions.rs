use std::num::NonZeroU32;

use ron::error::{Error, Position, Span, SpannedError};
use serde::{
    de::{Deserialize, Error as DeError, Unexpected},
    Deserializer,
};

#[cfg(feature = "internal-span-substring-test")]
use ron::util::span_substring::check_error_span_inclusive;

#[cfg(feature = "internal-span-substring-test")]
use ron::util::span_substring::check_error_span_exclusive;

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
enum Test {
    TupleVariant(i32, String),
    StructVariant { a: bool, b: NonZeroU32, c: i32 },
}

#[derive(Debug, PartialEq)] // GRCOV_EXCL_LINE
struct TypeError;

impl<'de> Deserialize<'de> for TypeError {
    fn deserialize<D: Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Err(D::Error::invalid_type(Unexpected::Unit, &"impossible"))
    }
}

#[test]
fn test_error_positions() {
    let bogus_struct = "  ()";
    let expected_err = Err(SpannedError {
        code: Error::InvalidValueForType {
            expected: String::from("impossible"),
            found: String::from("a unit value"),
        },
        span: Span {
            start: Position { line: 1, col: 1 },
            end: Position { line: 1, col: 3 },
        },
    });

    assert_eq!(ron::from_str::<TypeError>(bogus_struct), expected_err,);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<TypeError>(bogus_struct, expected_err, "  (");

    let bogus_struct = "StructVariant(a: true, b: 0, c: -42)";
    let expected_err = Err(SpannedError {
        code: Error::InvalidValueForType {
            expected: String::from("a nonzero u32"),
            found: String::from("the unsigned integer `0`"),
        },
        span: Span {
            start: Position { line: 1, col: 27 },
            end: Position { line: 1, col: 28 },
        },
    });

    assert_eq!(ron::from_str::<Test>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<Test>(bogus_struct, expected_err, "0,");

    let bogus_struct = "TupleVariant(42)";
    let expected_err = Err(SpannedError {
        code: Error::ExpectedDifferentLength {
            expected: String::from("tuple variant Test::TupleVariant with 2 elements"),
            found: 1,
        },
        span: Span {
            start: Position { line: 1, col: 16 },
            end: Position { line: 1, col: 16 },
        },
    });

    assert_eq!(ron::from_str::<Test>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<Test>(bogus_struct, expected_err, ")");

    let bogus_struct = "NotAVariant";
    let expected_err = Err(SpannedError {
        code: Error::NoSuchEnumVariant {
            expected: &["TupleVariant", "StructVariant"],
            found: String::from("NotAVariant"),
            outer: Some(String::from("Test")),
        },
        span: Span {
            start: Position { line: 1, col: 1 },
            end: Position { line: 1, col: 12 },
        },
    });

    assert_eq!(ron::from_str::<Test>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<Test>(bogus_struct, expected_err, "NotAVariant");

    let bogus_struct = "StructVariant(a: true, b: 1, c: -42, d: \"gotcha\")";
    let expected_err = Err(SpannedError {
        code: Error::NoSuchStructField {
            expected: &["a", "b", "c"],
            found: String::from("d"),
            outer: Some(String::from("StructVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 38 },
            end: Position { line: 1, col: 39 },
        },
    });

    assert_eq!(ron::from_str::<Test>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<Test>(bogus_struct, expected_err, "d:");

    let bogus_struct = "StructVariant(a: true, c: -42)";
    let expected_err = Err(SpannedError {
        code: Error::MissingStructField {
            field: "b",
            outer: Some(String::from("StructVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 30 },
            end: Position { line: 1, col: 30 },
        },
    });

    assert_eq!(ron::from_str::<Test>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<Test>(bogus_struct, expected_err, ")");

    let bogus_struct = "StructVariant(a: true, b: 1, a: false, c: -42)";
    let expected_err = Err(SpannedError {
        code: Error::DuplicateStructField {
            field: "a",
            outer: Some(String::from("StructVariant")),
        },
        span: Span {
            start: Position { line: 1, col: 30 },
            end: Position { line: 1, col: 31 },
        },
    });

    assert_eq!(ron::from_str::<Test>(bogus_struct), expected_err);

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<Test>(bogus_struct, expected_err, "a:");
}
