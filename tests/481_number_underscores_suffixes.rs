use ron::Number;

#[test]
#[allow(clippy::unusual_byte_groupings)]
#[allow(clippy::inconsistent_digit_grouping)]
#[allow(clippy::zero_prefixed_literal)]
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
#[allow(clippy::inconsistent_digit_grouping)]
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

    check_number_roundtrip(f32::NAN, "f32", f64::NAN);
    check_number_roundtrip(-f32::NAN, "f32", -f64::NAN);
    check_number_roundtrip(f32::INFINITY, "f32", f64::INFINITY);
    check_number_roundtrip(f32::NEG_INFINITY, "f32", f64::NEG_INFINITY);

    check_number_roundtrip(f64::NAN, "f64", f64::NAN);
    check_number_roundtrip(-f64::NAN, "f64", -f64::NAN);
    check_number_roundtrip(f64::INFINITY, "f64", f64::INFINITY);
    check_number_roundtrip(f64::NEG_INFINITY, "f64", f64::NEG_INFINITY);

    macro_rules! test_min_max {
        ($($ty:ty),*) => {
            $(
                check_number_roundtrip(<$ty>::MIN, stringify!($ty), <$ty>::MIN as f64);
                check_number_roundtrip(<$ty>::MAX, stringify!($ty), <$ty>::MAX as f64);
            )*
        };
    }

    test_min_max! { i8, i16, i32, i64, u8, u16, u32, u64, f32, f64 }
    #[cfg(feature = "integer128")]
    test_min_max! { i128, u128 }
}

fn check_number_roundtrip<
    T: Copy
        + Into<Number>
        + serde::Serialize
        + serde::de::DeserializeOwned
        + PartialEq
        + std::fmt::Debug,
>(
    n: T,
    suffix: &str,
    n_f64: f64,
) {
    let number: Number = n.into();
    let ron = ron::ser::to_string_pretty(
        &number,
        ron::ser::PrettyConfig::default().number_suffixes(true),
    )
    .unwrap();
    assert!(ron.ends_with(suffix));

    let ron =
        ron::ser::to_string_pretty(&n, ron::ser::PrettyConfig::default().number_suffixes(true))
            .unwrap();
    assert!(ron.ends_with(suffix));

    let de: ron::Value = ron::from_str(&ron).unwrap();
    assert_eq!(de, ron::Value::Number(number));

    let de: T = ron::from_str(&ron).unwrap();
    let de_number: Number = de.into();
    assert_eq!(de_number, number);

    assert_eq!(Number::from(de_number.into_f64()), Number::from(n_f64));
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

    assert_eq!(
        ron::from_str::<u8>("-1u8"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 5 },
        })
    );
    assert_eq!(
        ron::from_str::<u16>("-1u16"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 6 },
        })
    );
    assert_eq!(
        ron::from_str::<u32>("-1u32"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 6 },
        })
    );
    assert_eq!(
        ron::from_str::<u64>("-1u64"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 6 },
        })
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        ron::from_str::<u128>("-1u128"),
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

    assert_eq!(
        ron::from_str::<u8>("1u7"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );
    assert_eq!(
        ron::from_str::<f32>("1f17"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );
    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        ron::from_str::<u64>("1u128"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );
    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        ron::from_str::<i64>("1i128"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );
}

#[test]
fn number_type_mismatch() {
    assert_eq!(
        ron::from_str::<u8>("1i32"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidValueForType {
                expected: String::from("an 8-bit unsigned integer"),
                found: String::from("1i32")
            },
            position: ron::error::Position { line: 1, col: 5 },
        })
    );

    assert_eq!(
        ron::from_str::<i64>("-1u8"),
        Err(ron::error::SpannedError {
            code: ron::Error::IntegerOutOfBounds,
            position: ron::error::Position { line: 1, col: 5 },
        })
    );

    assert_eq!(
        ron::from_str::<f32>("1f64"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidValueForType {
                expected: String::from("a 32-bit floating point number"),
                found: String::from("1f64")
            },
            position: ron::error::Position { line: 1, col: 5 },
        })
    );

    assert_eq!(
        ron::from_str::<f64>("1f32"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidValueForType {
                expected: String::from("a 64-bit floating point number"),
                found: String::from("1f32")
            },
            position: ron::error::Position { line: 1, col: 5 },
        })
    );

    macro_rules! test_mismatch {
        ($($ty:ty),*) => {
            $(
                check_number_type_mismatch::<$ty>("i8");
                check_number_type_mismatch::<$ty>("i16");
                check_number_type_mismatch::<$ty>("i32");
                check_number_type_mismatch::<$ty>("i64");
                #[cfg(feature = "integer128")]
                check_number_type_mismatch::<$ty>("i128");
                check_number_type_mismatch::<$ty>("u8");
                check_number_type_mismatch::<$ty>("u16");
                check_number_type_mismatch::<$ty>("u32");
                check_number_type_mismatch::<$ty>("u64");
                #[cfg(feature = "integer128")]
                check_number_type_mismatch::<$ty>("u128");
            )*
        };
    }

    test_mismatch! { i8, i16, i32, i64, u8, u16, u32, u64 }
    #[cfg(feature = "integer128")]
    test_mismatch! { i128, u128 }
}

fn check_number_type_mismatch<T: std::fmt::Debug + serde::de::DeserializeOwned>(suffix: &str) {
    let ron = format!("0{suffix}");

    if suffix.starts_with(std::any::type_name::<T>()) {
        assert!(ron::from_str::<T>(&ron).is_ok());
        return;
    }

    let err = ron::from_str::<T>(&ron).unwrap_err();

    println!("{:?} {}", err, suffix);

    assert_eq!(
        err.position,
        ron::error::Position {
            line: 1,
            col: 2 + suffix.len()
        }
    );

    if !matches!(&err.code, ron::Error::InvalidValueForType { found, .. } if found == &ron ) {
        panic!("{:?}", err.code); // GRCOV_EXCL_LINE
    }
}

#[test]
fn float_const_prefix() {
    assert_eq!(
        ron::from_str::<f32>("NaNf32a").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::ExpectedFloat,
            position: ron::error::Position { line: 1, col: 1 },
        }
    );

    assert_eq!(
        ron::from_str::<f64>("-inff64a").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::ExpectedFloat,
            position: ron::error::Position { line: 1, col: 1 },
        }
    );

    assert_eq!(
        ron::from_str::<f32>("+NaNf17").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::ExpectedFloat,
            position: ron::error::Position { line: 1, col: 1 },
        }
    );
}

#[test]
fn invalid_float() {
    assert_eq!(
        ron::from_str::<f32>("1ee3").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::ExpectedFloat,
            position: ron::error::Position { line: 1, col: 1 },
        }
    );
    assert_eq!(
        ron::from_str::<f32>("1ee3f32").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::ExpectedFloat,
            position: ron::error::Position { line: 1, col: 1 },
        }
    );
    assert_eq!(
        ron::from_str::<f64>("1ee3f64").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::ExpectedFloat,
            position: ron::error::Position { line: 1, col: 1 },
        }
    );
}

#[test]
fn fuzzer_found_issues() {
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    enum A {
        #[serde(rename = "true")]
        True(bool),
        #[serde(rename = "false")]
        False(bool),
        #[serde(rename = "Some")]
        Some(bool),
        #[serde(rename = "None")]
        None(bool),
        #[serde(rename = "inf")]
        Inf(bool),
        #[serde(rename = "inff32")]
        InfF32(bool),
        #[serde(rename = "inff64")]
        InfF64(bool),
        #[serde(rename = "NaN")]
        NaN(bool),
        #[serde(rename = "NaNf32")]
        NaNF32(bool),
        #[serde(rename = "NaNf64")]
        NaNF64(bool),
    }

    assert_eq!(ron::to_string(&A::True(false)).unwrap(), "r#true(false)");
    assert_eq!(ron::from_str::<A>("r#true(false)").unwrap(), A::True(false));
    assert_eq!(
        ron::from_str::<ron::Value>("true(false)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 5 },
        }
    );

    assert_eq!(ron::to_string(&A::False(true)).unwrap(), "r#false(true)");
    assert_eq!(ron::from_str::<A>("r#false(true)").unwrap(), A::False(true));
    assert_eq!(
        ron::from_str::<ron::Value>("false(true)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 6 },
        }
    );

    assert_eq!(ron::to_string(&A::Some(false)).unwrap(), "r#Some(false)");
    assert_eq!(ron::from_str::<A>("r#Some(false)").unwrap(), A::Some(false));
    assert_eq!(
        ron::from_str::<ron::Value>("Some(false)").unwrap(),
        ron::Value::Option(Some(Box::new(ron::Value::Bool(false)))),
    );

    assert_eq!(ron::to_string(&A::None(true)).unwrap(), "r#None(true)");
    assert_eq!(ron::from_str::<A>("r#None(true)").unwrap(), A::None(true));
    assert_eq!(
        ron::from_str::<ron::Value>("None(true)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 5 },
        }
    );

    assert_eq!(ron::to_string(&A::Inf(false)).unwrap(), "r#inf(false)");
    assert_eq!(ron::from_str::<A>("r#inf(false)").unwrap(), A::Inf(false));
    assert_eq!(
        ron::from_str::<ron::Value>("inf(false)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 4 },
        }
    );

    assert_eq!(
        ron::to_string(&A::InfF32(false)).unwrap(),
        "r#inff32(false)"
    );
    assert_eq!(
        ron::from_str::<A>("r#inff32(false)").unwrap(),
        A::InfF32(false)
    );
    assert_eq!(
        ron::from_str::<ron::Value>("inff32(false)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 7 },
        }
    );

    assert_eq!(
        ron::to_string(&A::InfF64(false)).unwrap(),
        "r#inff64(false)"
    );
    assert_eq!(
        ron::from_str::<A>("r#inff64(false)").unwrap(),
        A::InfF64(false)
    );
    assert_eq!(
        ron::from_str::<ron::Value>("inff64(false)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 7 },
        }
    );

    assert_eq!(ron::to_string(&A::NaN(true)).unwrap(), "r#NaN(true)");
    assert_eq!(ron::from_str::<A>("r#NaN(true)").unwrap(), A::NaN(true));
    assert_eq!(
        ron::from_str::<ron::Value>("NaN(true)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 4 },
        }
    );

    assert_eq!(ron::to_string(&A::NaNF32(true)).unwrap(), "r#NaNf32(true)");
    assert_eq!(
        ron::from_str::<A>("r#NaNf32(true)").unwrap(),
        A::NaNF32(true)
    );
    assert_eq!(
        ron::from_str::<ron::Value>("NaNf32(true)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 7 },
        }
    );

    assert_eq!(ron::to_string(&A::NaNF64(true)).unwrap(), "r#NaNf64(true)");
    assert_eq!(
        ron::from_str::<A>("r#NaNf64(true)").unwrap(),
        A::NaNF64(true)
    );
    assert_eq!(
        ron::from_str::<ron::Value>("NaNf64(true)").unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 7 },
        }
    );
}
