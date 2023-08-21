use ron::Number;

#[test]
fn de_integer_underscores() {
    assert_eq!(ron::from_str("0b10_10___101_"), Ok(0b10_10___101__u8));
    assert_eq!(
        ron::from_str::<u8>("_0b1"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("_0b1_u8"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("0b2"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidIntegerDigit {
                digit: '2',
                base: 2
            },
            position: ron::error::Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        ron::from_str::<i32>("-0b2_i32"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidIntegerDigit {
                digit: '2',
                base: 2
            },
            position: ron::error::Position { line: 1, col: 4 },
        })
    );

    assert_eq!(ron::from_str("0o71_32___145_"), Ok(0o71_32___145_));
    assert_eq!(
        ron::from_str::<u8>("_0o5"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("0oA"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidIntegerDigit {
                digit: 'A',
                base: 8
            },
            position: ron::error::Position { line: 1, col: 3 },
        })
    );

    assert_eq!(ron::from_str("0xa1_fe___372_"), Ok(0xa1_fe___372_));
    assert_eq!(
        ron::from_str::<u8>("_0xF"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("0xZ"),
        Err(ron::error::SpannedError {
            code: ron::Error::ExpectedInteger,
            position: ron::error::Position { line: 1, col: 3 },
        })
    );

    assert_eq!(ron::from_str("0_6_163_810___17"), Ok(0_6_163_810___17));
    assert_eq!(
        ron::from_str::<u8>("_123"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("12a"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidIntegerDigit {
                digit: 'a',
                base: 10
            },
            position: ron::error::Position { line: 1, col: 3 },
        })
    );
}

#[test]
fn de_float_underscores() {
    assert_eq!(ron::from_str("2_18__6_"), Ok(2_18__6__f32));
    assert_eq!(
        ron::from_str::<f32>("_286"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<f32>("2a86"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );

    assert_eq!(ron::from_str("2_18__6_."), Ok(2_18__6__f32));
    assert_eq!(
        ron::from_str::<f32>("2_18__6_._"),
        Err(ron::error::SpannedError {
            code: ron::Error::FloatUnderscore,
            position: ron::error::Position { line: 1, col: 10 },
        })
    );
    assert_eq!(
        ron::from_str::<f32>("2_18__6_.3__7_"),
        Ok(2_18__6_.3__7__f32)
    );

    assert_eq!(ron::from_str::<f32>(".3__7_"), Ok(0.3__7__f32));
    assert_eq!(
        ron::from_str::<f32>("._3__7_"),
        Err(ron::error::SpannedError {
            code: ron::Error::FloatUnderscore,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );

    assert_eq!(
        ron::from_str::<f64>("2_18__6_.3__7_e____7_3__"),
        Ok(2_18__6_.3__7_e____7_3___f64)
    );
    assert_eq!(
        ron::from_str::<f64>("2_18__6_.3__7_e+____"),
        Err(ron::error::SpannedError {
            code: ron::Error::ExpectedFloat,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
}

#[test]
fn value_number_suffix_roundtrip() {
    assert_eq!(
        ron::from_str::<ron::Value>("1_f32").unwrap(),
        ron::Value::Number(ron::value::Number::new(1_f32))
    );
    assert_eq!(
        ron::from_str::<ron::Value>("-1_f32").unwrap(),
        ron::Value::Number(ron::value::Number::new(-1_f32))
    );

    check_number_roundtrip(f32::NAN, "_f32");
    check_number_roundtrip(-f32::NAN, "_f32");
    check_number_roundtrip(f32::INFINITY, "_f32");
    check_number_roundtrip(f32::NEG_INFINITY, "_f32");

    check_number_roundtrip(f64::NAN, "_f64");
    check_number_roundtrip(-f64::NAN, "_f64");
    check_number_roundtrip(f64::INFINITY, "_f64");
    check_number_roundtrip(f64::NEG_INFINITY, "_f64");

    macro_rules! test_min_max {
        ($($ty:ty),*) => {
            $(
                check_number_roundtrip(<$ty>::MIN, concat!("_", stringify!($ty)));
                check_number_roundtrip(<$ty>::MAX, concat!("_", stringify!($ty)));
            )*
        };
    }

    test_min_max! { i8, i16, i32, i64, u8, u16, u32, u64, f32, f64 }
    #[cfg(feature = "integer128")]
    test_min_max! { i128, u128 }
}

fn check_number_roundtrip<T: Into<Number>>(n: T, suffix: &str) {
    let number = n.into();
    let ron = ron::ser::to_string_pretty(
        &number,
        ron::ser::PrettyConfig::default().number_suffixes(true),
    )
    .unwrap();
    assert!(ron.ends_with(suffix));
    let de: ron::Value = ron::from_str(&ron).unwrap();
    assert_eq!(de, ron::Value::Number(number));
}

#[test]
fn negative_unsigned() {
    assert_eq!(
        ron::from_str::<ron::Value>("-1u8"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 5 },
        })
    );
    assert_eq!(
        ron::from_str::<ron::Value>("-1u16"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 6 },
        })
    );
    assert_eq!(
        ron::from_str::<ron::Value>("-1u32"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 6 },
        })
    );
    assert_eq!(
        ron::from_str::<ron::Value>("-1u64"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 6 },
        })
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        ron::from_str::<ron::Value>("-1u128"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 7 },
        })
    );
}

#[test]
fn invalid_suffix() {
    assert_eq!(
        ron::from_str::<ron::Value>("1u7"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );
    assert_eq!(
        ron::from_str::<ron::Value>("1f17"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );
    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        ron::from_str::<ron::Value>("1u128"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );
    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        ron::from_str::<ron::Value>("1i128"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );
}
