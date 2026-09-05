use std::{char::from_u32, fmt::Debug};

use ron::{de::from_str, ser::to_string};
use serde::{Deserialize, Serialize};

#[test]
fn test_escape_basic() {
    assert_eq!(to_string(&"\x07").unwrap(), "\"\\u{7}\"");

    assert_eq!(from_str::<String>("\"\\x07\"").unwrap(), "\x07");
    assert_eq!(from_str::<String>("\"\\u{7}\"").unwrap(), "\x07");

    assert_eq!(from_str::<char>("\'\\x07\'").unwrap(), '\x07');
    assert_eq!(from_str::<char>("\'\\u{7}\'").unwrap(), '\x07');

    assert_eq!(
        from_str::<char>("\'\\u{}\'").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::InvalidEscape("Expected 1-6 digits, got 0 digits in Unicode escape"),
            span: ron::error::Span {
                start: ron::error::Position { line: 1, col: 4 },
                end: ron::error::Position { line: 1, col: 5 },
            }
        }
    );

    assert_eq!(
        from_str::<char>("\'\\q\'").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::InvalidEscape("Unknown escape character"),
            span: ron::error::Span {
                start: ron::error::Position { line: 1, col: 1 },
                end: ron::error::Position { line: 1, col: 4 },
            }
        }
    )
}

fn check_same<T>(t: T)
where
    T: Debug + for<'a> Deserialize<'a> + PartialEq + Serialize,
{
    let s: String = to_string(&t).unwrap();

    println!("Serialized: \n\n{}\n\n", s);

    assert_eq!(from_str(&s), Ok(t));
}

#[test]
fn test_ascii_10() {
    check_same("\u{10}".to_owned());
}

#[test]
fn test_ascii_chars() {
    (1..128).flat_map(from_u32).for_each(check_same)
}

#[test]
fn test_ascii_string() {
    let s: String = (1..128).flat_map(from_u32).collect();

    check_same(s);
}

#[test]
fn test_non_ascii() {
    assert_eq!(to_string(&"♠").unwrap(), "\"♠\"");
    assert_eq!(to_string(&"ß").unwrap(), "\"ß\"");
    assert_eq!(to_string(&"ä").unwrap(), "\"ä\"");
    assert_eq!(to_string(&"ö").unwrap(), "\"ö\"");
    assert_eq!(to_string(&"ü").unwrap(), "\"ü\"");
}

#[test]
fn test_chars() {
    assert_eq!(to_string(&'♠').unwrap(), "'♠'");
    assert_eq!(to_string(&'ß').unwrap(), "'ß'");
    assert_eq!(to_string(&'ä').unwrap(), "'ä'");
    assert_eq!(to_string(&'ö').unwrap(), "'ö'");
    assert_eq!(to_string(&'ü').unwrap(), "'ü'");
    assert_eq!(to_string(&'\u{715}').unwrap(), "'\u{715}'");
    assert_eq!(
        from_str::<char>("'\u{715}'").unwrap(),
        from_str("'\\u{715}'").unwrap()
    );
}

#[test]
fn test_nul_in_string() {
    assert_eq!(
        from_str("\"Hello\0World!\""),
        Ok(String::from("Hello\0World!"))
    );

    check_same("Hello\0World!".to_owned());
    check_same("Hello\x00World!".to_owned());
    check_same("Hello\u{0}World!".to_owned());
}

#[test]
fn test_string_continuation_escape() {
    let cases = [
        (concat!("\"foo\\", "\n    bar\""), "foobar"),
        (concat!("\"foo\\", "\n\n    \\nbar\""), "foo\nbar"),
        (concat!("\"foo\\", "\r\n \tbar\""), "foobar"),
        (concat!("\"foo\\", "\n\r \tbar\""), "foobar"),
        (concat!("\"foo\\", "\n\""), "foo"),
    ];

    for (source, expected) in cases {
        assert_eq!(from_str::<String>(source).unwrap(), expected);
    }

    for source in [
        concat!("b\"foo\\", "\n    bar\""),
        concat!("b\"foo\\", "\r\n    bar\""),
    ] {
        let bytes = from_str::<bytes::Bytes>(source).unwrap();
        assert_eq!(&*bytes, b"foobar");
    }
}

#[test]
fn test_string_continuation_whitespace_boundary() {
    let retained = "\u{a0}\u{b}\u{c}\u{85}\u{200e}\u{200f}\u{2028}\u{2029}";
    let source = ["\"foo\\\n", retained, "bar\""].concat();
    let expected = ["foo", retained, "bar"].concat();

    assert_eq!(from_str::<String>(&source).unwrap(), expected);
}

#[test]
fn test_string_continuation_rejected_outside_strings() {
    for error in [
        from_str::<char>(concat!("'\\", "\n'")).unwrap_err().code,
        from_str::<u8>(concat!("b'\\", "\n'")).unwrap_err().code,
    ] {
        assert_eq!(error, ron::Error::InvalidEscape("Unknown escape character"));
    }
}

#[test]
fn test_string_continuation_raw_strings_are_unchanged() {
    let raw = from_str::<String>(concat!("r\"foo\\", "\n  bar\"")).unwrap();
    assert_eq!(raw, concat!("foo\\", "\n  bar"));

    let raw_bytes = from_str::<bytes::Bytes>(concat!("br\"foo\\", "\n  bar\"")).unwrap();
    assert_eq!(&*raw_bytes, concat!("foo\\", "\n  bar").as_bytes());
}

#[test]
fn test_string_continuation_errors() {
    assert_eq!(
        from_str::<String>(concat!("\"foo\\", "\rbar\""))
            .unwrap_err()
            .code,
        ron::Error::InvalidEscape("Unknown escape character")
    );

    assert_eq!(
        from_str::<String>(concat!("\"foo\\", "\n"))
            .unwrap_err()
            .code,
        ron::Error::ExpectedStringEnd
    );
}
