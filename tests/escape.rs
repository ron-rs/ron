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
