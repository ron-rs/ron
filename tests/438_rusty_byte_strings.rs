use ron::{
    error::{Position, SpannedError},
    Error,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct BytesStruct {
    small: Vec<u8>,
    #[serde(with = "serde_bytes")]
    large: Vec<u8>,
}

#[test]
fn v_9_deprecated_base64_bytes_support() {
    #![allow(deprecated)]

    // Requires padding of the base64 data
    assert_eq!(
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 3, 4]
        }),
        ron::from_str("BytesStruct( small:[1, 2], large:\"AQIDBA==\" )"),
    );

    // Requires no padding of the base64 data
    assert_eq!(
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 3, 4, 5, 6]
        }),
        ron::from_str("BytesStruct( small:[1, 2], large:r\"AQIDBAUG\" )"),
    );

    // Parsing an escaped byte string is also possible
    assert_eq!(
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 3, 4, 5, 6]
        }),
        ron::from_str("BytesStruct( small:[1, 2], large:\"\\x41Q\\x49DBA\\x55G\" )"),
    );

    // Invalid base64
    assert_eq!(
        Err(SpannedError {
            code: Error::Base64Error(base64::DecodeError::InvalidByte(0, b'_')),
            position: Position { line: 1, col: 40 }
        }),
        ron::from_str::<BytesStruct>("BytesStruct( small:[1, 2], large:\"_+!!\" )"),
    );

    // Invalid last base64 symbol
    assert_eq!(
        Err(SpannedError {
            code: Error::Base64Error(base64::DecodeError::InvalidLastSymbol(1, b'x')),
            position: Position { line: 1, col: 40 }
        }),
        ron::from_str::<BytesStruct>("BytesStruct( small:[1, 2], large:\"/x==\" )"),
    );

    // Missing padding
    assert_eq!(
        Err(SpannedError {
            code: Error::Base64Error(base64::DecodeError::InvalidPadding),
            position: Position { line: 1, col: 42 }
        }),
        ron::from_str::<BytesStruct>("BytesStruct( small:[1, 2], large:\"AQIDBA\" )"),
    );

    // Too much padding
    assert_eq!(
        Err(SpannedError {
            code: Error::Base64Error(base64::DecodeError::InvalidByte(6, b'=')),
            position: Position { line: 1, col: 45 }
        }),
        ron::from_str::<BytesStruct>("BytesStruct( small:[1, 2], large:\"AQIDBA===\" )"),
    );
}

#[test]
fn rusty_byte_string() {
    assert_eq!(
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 0, 4]
        }),
        ron::from_str("BytesStruct( small:[1, 2], large: b\"\\x01\\u{2}\\0\\x04\" )"),
    );

    assert_eq!(
        ron::from_str::<String>("\"Hello \\x01 \\u{2}!\"").unwrap(),
        "Hello \x01 \u{2}!",
    );
    assert_eq!(
        &*ron::from_str::<bytes::Bytes>("b\"Hello \\x01 \\u{2}!\"").unwrap(),
        b"Hello \x01 \x02!",
    );

    rusty_byte_string_roundtrip(b"hello", "b\"hello\"", "b\"hello\"");
    rusty_byte_string_roundtrip(b"\"", "b\"\\\"\"", "br#\"\"\"#");
    rusty_byte_string_roundtrip(b"\"#", "b\"\\\"#\"", "br##\"\"#\"##");
    rusty_byte_string_roundtrip(b"\n", "b\"\\n\"", "b\"\n\"");

    assert_eq!(
        ron::from_str::<bytes::Bytes>("b\"\\xf\"").unwrap_err(),
        SpannedError {
            code: Error::InvalidEscape("Non-hex digit found"),
            position: Position { line: 1, col: 7 },
        },
    );
    assert_eq!(
        ron::from_str::<bytes::Bytes>("b\"\\xf🦀\"").unwrap_err(),
        SpannedError {
            code: Error::InvalidEscape("Non-hex digit found"),
            position: Position { line: 1, col: 7 },
        },
    );
    let err = ron::from_str::<bytes::Bytes>("br#q\"").unwrap_err();
    assert_eq!(
        err,
        SpannedError {
            code: Error::ExpectedByteString,
            position: Position { line: 1, col: 4 },
        },
    );
    assert_eq!(format!("{}", err.code), "Expected byte string",);
    assert_eq!(
        ron::from_str::<bytes::Bytes>("br#\"q").unwrap_err(),
        SpannedError {
            code: Error::ExpectedStringEnd,
            position: Position { line: 1, col: 5 },
        },
    );
    assert_eq!(
        ron::from_str::<String>("r#q\"").unwrap_err(),
        SpannedError {
            code: Error::ExpectedString,
            position: Position { line: 1, col: 3 },
        },
    );
    assert_eq!(
        ron::from_str::<String>("r#\"q").unwrap_err(),
        SpannedError {
            code: Error::ExpectedStringEnd,
            position: Position { line: 1, col: 4 },
        },
    );
}

fn rusty_byte_string_roundtrip(bytes: &[u8], ron: &str, ron_raw: &str) {
    let ser_list = ron::to_string(bytes).unwrap();
    let de_list: Vec<u8> = ron::from_str(&ser_list).unwrap();
    assert_eq!(de_list, bytes);

    let ser = ron::to_string(&bytes::Bytes::copy_from_slice(bytes)).unwrap();
    assert_eq!(ser, ron);

    let ser_non_raw = ron::ser::to_string_pretty(
        &bytes::Bytes::copy_from_slice(bytes),
        ron::ser::PrettyConfig::default(),
    )
    .unwrap();
    assert_eq!(ser_non_raw, ron);

    let ser_raw = ron::ser::to_string_pretty(
        &bytes::Bytes::copy_from_slice(bytes),
        ron::ser::PrettyConfig::default().escape_strings(false),
    )
    .unwrap();
    assert_eq!(ser_raw, ron_raw);

    let de: bytes::Bytes = ron::from_str(&ser).unwrap();
    assert_eq!(de, bytes);

    let de_raw: bytes::Bytes = ron::from_str(&ser_raw).unwrap();
    assert_eq!(de_raw, bytes);
}

#[test]
fn fuzzer_failures() {
    assert_eq!(
        ron::to_string(&bytes::Bytes::copy_from_slice(&[
            123, 0, 0, 0, 0, 214, 214, 214, 214, 214
        ]))
        .unwrap(),
        r#"b"{\x00\x00\x00\x00\xd6\xd6\xd6\xd6\xd6""#
    );
    // Need to fall back to escaping so no invalid UTF-8 is produced
    assert_eq!(
        ron::ser::to_string_pretty(
            &bytes::Bytes::copy_from_slice(&[123, 0, 0, 0, 0, 214, 214, 214, 214, 214]),
            ron::ser::PrettyConfig::default().escape_strings(false)
        )
        .unwrap(),
        r#"b"{\x00\x00\x00\x00\xd6\xd6\xd6\xd6\xd6""#
    );

    assert_eq!(
        ron::to_string(&bytes::Bytes::copy_from_slice(&[123, 0, 0, 0, 0])).unwrap(),
        r#"b"{\x00\x00\x00\x00""#
    );
    assert_eq!(
        ron::ser::to_string_pretty(
            &bytes::Bytes::copy_from_slice(&[123, 0, 0, 0, 0]),
            ron::ser::PrettyConfig::default().escape_strings(false)
        )
        .unwrap(),
        "b\"{\x00\x00\x00\x00\""
    );

    // `br#` should be parsed as the start of a byte string, not the identifier `br` and a `#`
    assert_eq!(
        ron::from_str(r##"br#"""#"##),
        Ok(ron::Value::Bytes(vec![34]))
    );
    assert_eq!(
        ron::from_str(r##"{"error": br#"""#}"##),
        Ok(ron::Value::Map(
            [(
                ron::Value::String(String::from("error")),
                ron::Value::Bytes(vec![34])
            )]
            .into_iter()
            .collect()
        ))
    );
    assert_eq!(
        ron::from_str(
            r##"#![enable(unwrap_newtypes)]
    #![enable(unwrap_variant_newtypes)]
    Some({"error": br#"""#})
    "##
        ),
        Ok(ron::Value::Option(Some(Box::new(ron::Value::Map(
            [(
                ron::Value::String(String::from("error")),
                ron::Value::Bytes(vec![34])
            )]
            .into_iter()
            .collect()
        )))))
    );

    // `br"` should be parsed as the start of a byte string, not the identifier `br` and a `"`
    assert_eq!(ron::from_str(r#"br"""#), Ok(ron::Value::Bytes(vec![])));
    assert_eq!(
        ron::from_str(r#"{"error": br""}"#),
        Ok(ron::Value::Map(
            [(
                ron::Value::String(String::from("error")),
                ron::Value::Bytes(vec![])
            )]
            .into_iter()
            .collect()
        ))
    );
    assert_eq!(
        ron::from_str(
            r#"#![enable(unwrap_newtypes)]
    #![enable(unwrap_variant_newtypes)]
    Some({"error": br""})
    "#
        ),
        Ok(ron::Value::Option(Some(Box::new(ron::Value::Map(
            [(
                ron::Value::String(String::from("error")),
                ron::Value::Bytes(vec![])
            )]
            .into_iter()
            .collect()
        )))))
    );

    // Test that the struct type check for newtype variant unwrapping does
    //  not enter inside a byte string to find a bad comment start
    assert_eq!(
        ron::from_str::<Option<ron::Value>>(
            r#"#![enable(unwrap_variant_newtypes)] Some(b"\xff/not a comment")"#
        )
        .unwrap(),
        Some(ron::Value::Bytes(b"\xff/not a comment".to_vec()))
    );

    // `b'` should be parsed as the start of a byte literal, not the identifier `b` and a `'`
    assert_eq!(
        ron::from_str(r"b'\xff'"),
        Ok(ron::Value::Number(ron::value::Number::U8(b'\xff')))
    );

    // `b`, `br`, `bq`, and `brq` should all be parsed as identifiers
    for id in ["b", "br", "bq", "brq"] {
        assert_eq!(ron::from_str(id), Ok(ron::Value::Unit));
    }
}

#[test]
fn serialize_backslash_byte_string() {
    check_roundtrip('\\', r"'\\'", r"'\\'");
    check_roundtrip(
        bytes::Bytes::copy_from_slice(b"\\"),
        r#"b"\\""#,
        "br#\"\\\"#",
    );
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

#[test]
fn test_weird_escapes() {
    assert_eq!(
        ron::from_str::<String>(r#""\u{1F980}""#),
        Ok(String::from("\u{1F980}"))
    );
    assert_eq!(
        ron::from_str::<bytes::Bytes>(r#"b"\xf0\x9f\xa6\x80""#),
        Ok(bytes::Bytes::copy_from_slice("\u{1F980}".as_bytes()))
    );
    assert_eq!(
        ron::from_str::<String>(r#""\xf0\x9f\xa6\x80""#),
        Ok(String::from("\u{1F980}"))
    );
    assert_eq!(
        ron::from_str::<String>(r#""\xf0""#),
        Err(SpannedError {
            code: Error::InvalidEscape("Not a valid byte-escaped Unicode character"),
            position: Position { line: 1, col: 6 }
        })
    );
    assert_eq!(
        ron::from_str::<String>(r#""\xf0\x9f""#),
        Err(SpannedError {
            code: Error::InvalidEscape("Not a valid byte-escaped Unicode character"),
            position: Position { line: 1, col: 10 }
        })
    );
    assert_eq!(
        ron::from_str::<String>(r#""\xf0\x9f\x40""#),
        Err(SpannedError {
            code: Error::InvalidEscape("Not a valid byte-escaped Unicode character"),
            position: Position { line: 1, col: 14 }
        })
    );
    assert_eq!(
        ron::from_str::<String>(r#""\xf0\x9f\xa6""#),
        Err(SpannedError {
            code: Error::InvalidEscape("Not a valid byte-escaped Unicode character"),
            position: Position { line: 1, col: 14 }
        })
    );
    assert_eq!(
        ron::from_str::<String>(r#""\xff\xff\xff\xff""#),
        Err(SpannedError {
            code: Error::InvalidEscape("Not a valid byte-escaped Unicode character"),
            position: Position { line: 1, col: 18 }
        })
    );

    assert_eq!(ron::from_str::<char>(r"'\u{1F980}'"), Ok('\u{1F980}'));
    assert_eq!(
        ron::from_str::<char>(r"'\xf0\x9f\xa6\x80'"),
        Err(SpannedError {
            code: Error::InvalidEscape("Not a valid byte-escaped Unicode character"),
            position: Position { line: 1, col: 6 }
        })
    );
}

#[test]
fn byte_literal() {
    assert_eq!(
        ron::from_str("b'\0'"),
        Ok(ron::Value::Number(ron::value::Number::U8(0)))
    );
    assert_eq!(
        ron::from_str("b'\\0'"),
        Ok(ron::Value::Number(ron::value::Number::U8(0)))
    );

    for b in 0..=255_u8 {
        let default = std::ascii::escape_default(b)
            .map(char::from)
            .collect::<String>();
        let lower = format!(r"\x{:02x}", b);
        let upper = format!(r"\x{:02X}", b);

        assert_eq!(
            ron::from_str(&format!("b'{}'", default)),
            Ok(ron::Value::Number(ron::value::Number::U8(b)))
        );
        assert_eq!(
            ron::from_str(&format!("b'{}'", lower)),
            Ok(ron::Value::Number(ron::value::Number::U8(b)))
        );
        assert_eq!(
            ron::from_str(&format!("b'{}'", upper)),
            Ok(ron::Value::Number(ron::value::Number::U8(b)))
        );
    }

    assert_eq!(
        ron::from_str::<u8>(r#"b'\u{0}'"#),
        Err(SpannedError {
            code: Error::InvalidEscape("Unexpected Unicode escape in byte literal"),
            position: Position { line: 1, col: 8 },
        })
    );

    let err = ron::from_str::<u8>(r#"b'🦀'"#).unwrap_err();
    assert_eq!(
        err,
        SpannedError {
            code: Error::ExpectedByteLiteral,
            position: Position { line: 1, col: 4 },
        }
    );
    assert_eq!(format!("{}", err.code), "Expected byte literal");

    assert_eq!(
        ron::from_str::<u8>(r#"b'qq'"#).unwrap_err(),
        SpannedError {
            code: Error::ExpectedByteLiteral,
            position: Position { line: 1, col: 4 },
        }
    );

    assert_eq!(
        ron::from_str::<i8>(r#"b'9'"#),
        Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("an 8-bit signed integer"),
                found: String::from(r#"b'9'"#)
            },
            position: Position { line: 1, col: 5 },
        })
    );
}

#[test]
fn invalid_identifier() {
    #[allow(dead_code)]
    #[derive(Debug, Deserialize)] // GRCOV_EXCL_LINE
    struct Test {
        a: i32,
    }

    for id in ["b\"", "b'", "br#", "br\"", "r\"", "r#\"", "r##"] {
        assert_eq!(
            ron::from_str::<Test>(&format!("({}: 42)", id)).unwrap_err(),
            SpannedError {
                code: Error::ExpectedIdentifier,
                position: Position { line: 1, col: 2 },
            }
        );
    }
}
