use ron::{
    de::from_str,
    ser::{to_string, to_string_pretty, PrettyConfig},
};

fn test_string_roundtrip(s: &str, config: Option<PrettyConfig>) -> String {
    let ser = match config {
        Some(config) => to_string_pretty(s, config),
        None => to_string(s),
    }
    .unwrap();

    let de: String = from_str(&ser).unwrap();

    assert_eq!(s, de);

    ser
}

#[test]
fn test_escaped_string() {
    let config = Some(PrettyConfig::default());

    assert_eq!(test_string_roundtrip("a\nb", None), r#""a\nb""#);
    assert_eq!(test_string_roundtrip("a\nb", config.clone()), r#""a\nb""#);

    assert_eq!(test_string_roundtrip("", None), "\"\"");
    assert_eq!(test_string_roundtrip("", config.clone()), "\"\"");

    assert_eq!(test_string_roundtrip("\"", None), r#""\"""#);
    assert_eq!(test_string_roundtrip("\"", config.clone()), r#""\"""#);

    assert_eq!(test_string_roundtrip("#", None), "\"#\"");
    assert_eq!(test_string_roundtrip("#", config.clone()), "\"#\"");

    assert_eq!(test_string_roundtrip("\"#", None), r##""\"#""##);
    assert_eq!(test_string_roundtrip("\"#", config.clone()), r##""\"#""##);

    assert_eq!(test_string_roundtrip("#\"#", None), r##""#\"#""##);
    assert_eq!(test_string_roundtrip("#\"#", config.clone()), r##""#\"#""##);

    assert_eq!(test_string_roundtrip("#\"##", None), r###""#\"##""###);
    assert_eq!(test_string_roundtrip("#\"##", config), r###""#\"##""###);
}

#[test]
fn test_unescaped_string() {
    let config = Some(PrettyConfig::default().escape_strings(false));

    assert_eq!(test_string_roundtrip("a\nb", config.clone()), "\"a\nb\"");
    assert_eq!(test_string_roundtrip("", config.clone()), "\"\"");
    assert_eq!(test_string_roundtrip("\"", config.clone()), "r#\"\"\"#");
    assert_eq!(test_string_roundtrip("#", config.clone()), "\"#\"");
    assert_eq!(test_string_roundtrip("\"#", config.clone()), "r##\"\"#\"##");
    assert_eq!(
        test_string_roundtrip("#\"#", config.clone()),
        "r##\"#\"#\"##"
    );
    assert_eq!(test_string_roundtrip("#\"##", config), "r###\"#\"##\"###");
}
