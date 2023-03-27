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
fn test_file_invalid_unicode() {
    let error = from_bytes::<Value>(&[b'\n', b'a', 0b11000000, 0]).unwrap_err();
    assert!(matches!(error.code, Error::Utf8Error(_)));
    assert_eq!(error.position, Position { line: 2, col: 2 });
    let error = from_bytes::<Value>(&[b'\n', b'\n', 0b11000000]).unwrap_err();
    assert!(matches!(error.code, Error::Utf8Error(_)));
    assert_eq!(error.position, Position { line: 3, col: 1 });
}
