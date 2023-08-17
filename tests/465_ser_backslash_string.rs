#[test]
fn serialize_backslash_string() {
    assert_eq!(ron::to_string(&'\\').unwrap(), r"'\\'");
    assert_eq!(ron::to_string(&"\\").unwrap(), r#""\\""#);
    assert_eq!(
        ron::ser::to_string_pretty(
            &"\\",
            ron::ser::PrettyConfig::default().escape_strings(true)
        )
        .unwrap(),
        r#""\\""#
    );
    assert_eq!(
        ron::ser::to_string_pretty(
            &"\\",
            ron::ser::PrettyConfig::default().escape_strings(false)
        )
        .unwrap(),
        "r#\"\\\"#"
    );
}
