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

#[test]
fn test_escaped_string_matches_escape_debug() {
    // Mix runs of plain ASCII with every kind of escaped character.
    let inputs = [
        "plain ascii text, no escapes",
        "tab\there, quote\" apostrophe' backslash\\ end",
        "\0\u{7}\u{7f}\u{80}\u{a0} control and latin-1",
        "é中😀 multi-byte between ascii",
        "e\u{301} combining, \u{200b}\u{feff}\u{2028}\u{e000} invisible",
        "~ !} edges of the printable range",
    ];

    for s in inputs {
        let expected = format!(
            "\"{}\"",
            s.chars().flat_map(char::escape_debug).collect::<String>()
        );
        assert_eq!(test_string_roundtrip(s, None), expected);
        assert_eq!(
            test_string_roundtrip(s, Some(PrettyConfig::default())),
            expected
        );
    }
}
