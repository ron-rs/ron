use ron::de::from_str;

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
