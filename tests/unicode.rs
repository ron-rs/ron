use ron::{
    de::{from_bytes, from_str},
    error::Position,
    Error, Value,
};

#[test]
fn test_char() {
    let de: char = from_str("'Փ'").unwrap();
    assert_eq!(de, 'Փ');
}

#[test]
fn test_string() {
    let de: String = from_str("\"My string: ऄ\"").unwrap();
    assert_eq!(de, "My string: ऄ");
}

#[test]
fn test_char_not_a_comment() {
    let _ = from_str::<ron::Value>("A('/')").unwrap();
}

#[test]
fn ident_starts_with_non_ascii_byte() {
    let _ = from_str::<Value>("שּׁȬSSSSSSSSSSR").unwrap();
}

#[test]
fn test_file_invalid_unicode() {
    let error = from_bytes::<Value>(&[b'\n', b'a', 0b11000000, 0]).unwrap_err();
    assert!(matches!(error.code, Error::Utf8Error(_)));
    assert_eq!(error.position, Position { line: 2, col: 2 });
    let error = from_bytes::<Value>(&[b'\n', b'\n', 0b11000000]).unwrap_err();
    assert!(matches!(error.code, Error::Utf8Error(_)));
    assert_eq!(error.position, Position { line: 3, col: 1 });
}

#[test]
fn serialize_invalid_whitespace() {
    assert_eq!(
        ron::ser::to_string_pretty(&42, ron::ser::PrettyConfig::default().new_line("a"))
            .unwrap_err(),
        Error::Message(String::from(
            "Invalid non-whitespace `PrettyConfig::new_line`"
        ))
    );
    assert_eq!(
        ron::ser::to_string_pretty(&42, ron::ser::PrettyConfig::default().indentor("a"))
            .unwrap_err(),
        Error::Message(String::from(
            "Invalid non-whitespace `PrettyConfig::indentor`"
        ))
    );
    assert_eq!(
        ron::ser::to_string_pretty(&42, ron::ser::PrettyConfig::default().separator("a"))
            .unwrap_err(),
        Error::Message(String::from(
            "Invalid non-whitespace `PrettyConfig::separator`"
        ))
    );
}
