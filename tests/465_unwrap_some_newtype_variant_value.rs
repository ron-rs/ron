#[test]
fn deserialise_value_with_unwrap_some_newtype_variant() {
    assert_eq!(
        ron::from_str::<ron::Value>("Some(a: 42)"),
        Err(ron::error::SpannedError {
            code: ron::Error::ExpectedOptionEnd,
            position: ron::error::Position { line: 1, col: 7 },
        }),
    );
    assert_eq!(
        ron::from_str("#![enable(unwrap_variant_newtypes)] Some(a: 42)"),
        Ok(ron::Value::Option(Some(Box::new(ron::Value::Map(
            [(
                ron::Value::String(String::from("a")),
                ron::Value::Number(42.into())
            )]
            .into_iter()
            .collect()
        ))))),
    );
}
