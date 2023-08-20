use ron::value::{Number, Value};

#[test]
fn test_large_number() {
    let test_var = Value::Number(Number::new(10000000000000000000000.0f64));
    let test_ser = ron::ser::to_string(&test_var).unwrap();
    let test_deser = ron::de::from_str::<Value>(&test_ser);

    assert_eq!(
        test_deser.unwrap(),
        Value::Number(Number::new(10000000000000000000000.0))
    );
}

#[test]
fn test_large_integer_to_float() {
    use ron::value::Number;
    let test_var = std::u64::MAX as u128 + 1;
    let test_ser = test_var.to_string();
    let test_deser = ron::de::from_str::<Value>(&test_ser);

    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        test_deser.unwrap(),
        Value::Number(Number::F32((test_var as f32).into())), // f64 representation matches f32
    );
    #[cfg(feature = "integer128")]
    assert_eq!(test_deser.unwrap(), Value::Number(Number::U128(test_var)),);
}
