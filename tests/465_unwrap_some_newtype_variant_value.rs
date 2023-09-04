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
                ron::Value::Number(ron::value::Number::U8(42))
            )]
            .into_iter()
            .collect()
        ))))),
    );
    assert_eq!(
        ron::from_str("#![enable(unwrap_variant_newtypes)] Some(42, true)"),
        Ok(ron::Value::Option(Some(Box::new(ron::Value::Seq(vec![
            ron::Value::Number(ron::value::Number::U8(42)),
            ron::Value::Bool(true)
        ]))))),
    );
    assert_eq!(
        ron::from_str("#![enable(unwrap_variant_newtypes)] Some(42,)"),
        Ok(ron::Value::Option(Some(Box::new(ron::Value::Number(
            ron::value::Number::U8(42)
        ))))),
    );
    assert_eq!(
        ron::from_str("#![enable(unwrap_variant_newtypes)] Some()"),
        Ok(ron::Value::Option(Some(Box::new(ron::Value::Unit)))),
    );
    assert_eq!(
        ron::from_str("#![enable(unwrap_variant_newtypes)] Some(42)"),
        Ok(ron::Value::Option(Some(Box::new(ron::Value::Number(
            ron::value::Number::U8(42)
        ))))),
    );
}
