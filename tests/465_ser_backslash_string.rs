#[test]
fn serialize_backslash_string() {
    check_roundtrip('\\', r"'\\'", r"'\\'");
    check_roundtrip(String::from("\\"), r#""\\""#, "r#\"\\\"#");
}

fn check_roundtrip<
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
>(
    val: T,
    cmp: &str,
    cmp_raw: &str,
) {
    let ron = ron::to_string(&val).unwrap();
    assert_eq!(ron, cmp);

    let ron_escaped =
        ron::ser::to_string_pretty(&val, ron::ser::PrettyConfig::default().escape_strings(true))
            .unwrap();
    assert_eq!(ron_escaped, cmp);

    let ron_raw = ron::ser::to_string_pretty(
        &val,
        ron::ser::PrettyConfig::default().escape_strings(false),
    )
    .unwrap();
    assert_eq!(ron_raw, cmp_raw);

    let de = ron::from_str::<T>(&ron).unwrap();
    assert_eq!(de, val);

    let de_raw = ron::from_str::<T>(&ron_raw).unwrap();
    assert_eq!(de_raw, val);
}
