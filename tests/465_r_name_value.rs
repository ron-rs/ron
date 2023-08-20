#[test]
fn value_deserialises_r_name() {
    assert_eq!(ron::from_str("r"), Ok(ron::Value::Unit));
    assert_eq!(ron::from_str("r()"), Ok(ron::Value::Seq(vec![])));
    assert_eq!(
        ron::from_str("r(42)"),
        Ok(ron::Value::Seq(vec![ron::Value::Number(
            ron::value::Number::U8(42)
        )]))
    );
    assert_eq!(
        ron::from_str("r(a:42)"),
        Ok(ron::Value::Map(
            [(
                ron::Value::String(String::from("a")),
                ron::Value::Number(ron::value::Number::U8(42))
            )]
            .into_iter()
            .collect()
        ))
    );
}
