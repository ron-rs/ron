#[test]
fn extreme_floats_use_exponent_notation() {
    assert_eq!(ron::to_string(&1e300_f64).unwrap(), "1e300");
    assert_eq!(ron::to_string(&5e-324_f64).unwrap(), "5e-324");
    assert_eq!(ron::to_string(&f64::MAX).unwrap(), "1.7976931348623157e308");
    assert_eq!(ron::to_string(&1e30_f32).unwrap(), "1e30");
    assert_eq!(ron::to_string(&f32::MAX).unwrap(), "3.4028235e38");
}

#[test]
fn ordinary_floats_keep_decimal_notation() {
    assert_eq!(ron::to_string(&1.0_f64).unwrap(), "1.0");
    assert_eq!(ron::to_string(&-0.0_f64).unwrap(), "-0.0");
    assert_eq!(ron::to_string(&0.1_f64).unwrap(), "0.1");
    assert_eq!(ron::to_string(&123.456_f64).unwrap(), "123.456");
    assert_eq!(ron::to_string(&3.0_f32).unwrap(), "3.0");
}

#[test]
fn exponent_floats_roundtrip() {
    for v in [
        1e300_f64,
        5e-324,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        1e16,
        1e-5,
    ] {
        let ron = ron::to_string(&v).unwrap();
        assert_eq!(ron::from_str::<f64>(&ron).unwrap().to_bits(), v.to_bits());

        // Untyped values must still be parsed as floats, not integers
        assert_eq!(
            ron::from_str::<ron::Value>(&ron).unwrap(),
            ron::Value::Number(ron::value::Number::F64(v.into()))
        );
    }

    for v in [1e30_f32, 1e-40, f32::MAX, f32::MIN_POSITIVE] {
        let ron = ron::to_string(&v).unwrap();
        assert_eq!(ron::from_str::<f32>(&ron).unwrap().to_bits(), v.to_bits());
    }
}

#[test]
fn exponent_floats_with_number_suffixes() {
    let config = ron::ser::PrettyConfig::default().number_suffixes(true);

    let ron = ron::ser::to_string_pretty(&1e300_f64, config.clone()).unwrap();
    assert_eq!(ron, "1e300f64");
    assert_eq!(ron::from_str::<f64>(&ron).unwrap(), 1e300);

    let ron = ron::ser::to_string_pretty(&1e30_f32, config).unwrap();
    assert_eq!(ron, "1e30f32");
    assert_eq!(ron::from_str::<f32>(&ron).unwrap(), 1e30);
}
