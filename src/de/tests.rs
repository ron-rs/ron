use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use serde_bytes;
use serde_derive::Deserialize;

use crate::{
    error::{Error, Position, Span, SpannedError, SpannedResult},
    parse::Parser,
    value::Number,
};

#[cfg(feature = "internal-span-substring-test")]
use crate::util::span_substring::check_error_span_inclusive;

#[cfg(feature = "internal-span-substring-test")]
use crate::util::span_substring::check_error_span_exclusive;

#[derive(Debug, PartialEq, Deserialize)]
struct EmptyStruct1;

#[derive(Debug, PartialEq, Deserialize)]
struct EmptyStruct2 {}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct NewType(i32);

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename = "")]
struct UnnamedNewType(i32);

#[derive(Debug, PartialEq, Deserialize)]
struct TupleStruct(f32, f32);

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename = "")]
struct UnnamedTupleStruct(f32, f32);

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
struct MyStruct {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
enum MyEnum {
    A,
    B(bool),
    C(bool, f32),
    D { a: i32, b: i32 },
}

#[derive(Debug, Deserialize, PartialEq)]
struct BytesStruct {
    small: Vec<u8>,
    #[serde(with = "serde_bytes")]
    large: Vec<u8>,
}

#[test]
fn test_empty_struct() {
    check_from_str_bytes_reader("EmptyStruct1", Ok(EmptyStruct1));
    check_from_str_bytes_reader("EmptyStruct2()", Ok(EmptyStruct2 {}));
}

#[test]
fn test_struct() {
    let my_struct = MyStruct { x: 4.0, y: 7.0 };

    check_from_str_bytes_reader("MyStruct(x:4,y:7,)", Ok(my_struct));
    check_from_str_bytes_reader("(x:4,y:7)", Ok(my_struct));

    check_from_str_bytes_reader("NewType(42)", Ok(NewType(42)));
    check_from_str_bytes_reader("(33)", Ok(NewType(33)));

    let bogus_struct = "NewType";
    let expected_err = Err(SpannedError {
        code: Error::ExpectedNamedStructLike("NewType"),
        span: Span {
            start: Position { line: 1, col: 1 },
            end: Position { line: 1, col: 8 },
        },
    });
    check_from_str_bytes_reader::<NewType>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<NewType>(bogus_struct, expected_err, "NewType");

    check_from_str_bytes_reader::<UnnamedNewType>(
        "",
        Err(SpannedError {
            code: Error::ExpectedStructLike,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            },
        }),
    );
    check_from_str_bytes_reader("(33)", Ok(UnnamedNewType(33)));

    let bogus_struct = "NewType";
    let expected_err = Err(SpannedError {
        code: Error::ExpectedNamedStructLike(""),
        span: Span {
            start: Position { line: 1, col: 1 },
            end: Position { line: 1, col: 8 },
        },
    });

    check_from_str_bytes_reader::<UnnamedNewType>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<UnnamedNewType>(bogus_struct, expected_err, "NewType");

    check_from_str_bytes_reader("TupleStruct(2,5,)", Ok(TupleStruct(2.0, 5.0)));
    check_from_str_bytes_reader("(3,4)", Ok(TupleStruct(3.0, 4.0)));
    check_from_str_bytes_reader::<TupleStruct>(
        "",
        Err(SpannedError {
            code: Error::ExpectedNamedStructLike("TupleStruct"),
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            },
        }),
    );

    let bogus_struct = "TupleStruct(2,5,)";
    let expected_err = Err(SpannedError {
        code: Error::ExpectedNamedStructLike(""),
        span: Span {
            start: Position { line: 1, col: 1 },
            end: Position { line: 1, col: 12 },
        },
    });
    check_from_str_bytes_reader::<UnnamedTupleStruct>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<UnnamedTupleStruct>(bogus_struct, expected_err, "TupleStruct");

    check_from_str_bytes_reader("(3,4)", Ok(UnnamedTupleStruct(3.0, 4.0)));
    check_from_str_bytes_reader::<UnnamedTupleStruct>(
        "",
        Err(SpannedError {
            code: Error::ExpectedStructLike,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            },
        }),
    );
}

#[test]
fn test_unclosed_limited_seq_struct() {
    #[derive(Debug, PartialEq)]
    struct LimitedStruct;

    impl<'de> serde::Deserialize<'de> for LimitedStruct {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visitor;

            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = LimitedStruct;

                // GRCOV_EXCL_START
                fn expecting(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
                    fmt.write_str("struct LimitedStruct")
                }
                // GRCOV_EXCL_STOP

                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    _map: A,
                ) -> Result<Self::Value, A::Error> {
                    Ok(LimitedStruct)
                }
            }

            deserializer.deserialize_struct("LimitedStruct", &[], Visitor)
        }
    }

    check_from_str_bytes_reader::<LimitedStruct>(
        "(",
        Err(SpannedError {
            code: Error::ExpectedStructLikeEnd,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 2 },
            },
        }),
    )
}

#[test]
fn test_unclosed_limited_seq() {
    #[derive(Debug, PartialEq)]
    struct LimitedSeq;

    impl<'de> serde::Deserialize<'de> for LimitedSeq {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visitor;

            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = LimitedSeq;

                // GRCOV_EXCL_START
                fn expecting(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
                    fmt.write_str("an empty sequence")
                }
                // GRCOV_EXCL_STOP

                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    _seq: A,
                ) -> Result<Self::Value, A::Error> {
                    Ok(LimitedSeq)
                }
            }

            deserializer.deserialize_seq(Visitor)
        }
    }

    check_from_str_bytes_reader::<LimitedSeq>(
        "[",
        Err(SpannedError {
            code: Error::ExpectedArrayEnd,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 2 },
            },
        }),
    );

    assert_eq!(
        crate::Value::from(vec![42]).into_rust::<LimitedSeq>(),
        Err(Error::ExpectedDifferentLength {
            expected: String::from("a sequence of length 0"),
            found: 1
        })
    );
}

#[test]
fn test_unclosed_limited_map() {
    #[derive(Debug, PartialEq)]
    struct LimitedMap;

    impl<'de> serde::Deserialize<'de> for LimitedMap {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visitor;

            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = LimitedMap;

                // GRCOV_EXCL_START
                fn expecting(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
                    fmt.write_str("an empty map")
                }
                // GRCOV_EXCL_STOP

                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    _map: A,
                ) -> Result<Self::Value, A::Error> {
                    Ok(LimitedMap)
                }
            }

            deserializer.deserialize_map(Visitor)
        }
    }

    check_from_str_bytes_reader::<LimitedMap>(
        "{",
        Err(SpannedError {
            code: Error::ExpectedMapEnd,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 2 },
            },
        }),
    );

    assert_eq!(
        crate::Value::Map([("a", 42)].into_iter().collect()).into_rust::<LimitedMap>(),
        Err(Error::ExpectedDifferentLength {
            expected: String::from("a map of length 0"),
            found: 1
        })
    );
}

#[test]
fn test_option() {
    check_from_str_bytes_reader("Some(1)", Ok(Some(1u8)));
    check_from_str_bytes_reader("None", Ok(None::<u8>));
}

#[test]
fn test_enum() {
    check_from_str_bytes_reader("A", Ok(MyEnum::A));
    check_from_str_bytes_reader("B(true,)", Ok(MyEnum::B(true)));
    check_from_str_bytes_reader::<MyEnum>(
        "B",
        Err(SpannedError {
            code: Error::ExpectedStructLike,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 2 },
            },
        }),
    );
    check_from_str_bytes_reader("C(true,3.5,)", Ok(MyEnum::C(true, 3.5)));
    check_from_str_bytes_reader("D(a:2,b:3,)", Ok(MyEnum::D { a: 2, b: 3 }));
}

#[test]
fn test_array() {
    check_from_str_bytes_reader::<[i32; 0]>("()", Ok([]));
    check_from_str_bytes_reader("[]", Ok(Vec::<i32>::new()));

    check_from_str_bytes_reader("(2,3,4,)", Ok([2, 3, 4i32]));
    check_from_str_bytes_reader("[2,3,4,]", Ok([2, 3, 4i32].to_vec()));
}

#[cfg(feature = "std")]
#[test]
fn test_map() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert((true, false), 4);
    map.insert((false, false), 123);

    check_from_str_bytes_reader(
        "{
        (true,false,):4,
        (false,false,):123,
    }",
        Ok(map),
    );
}

#[test]
fn test_string() {
    check_from_str_bytes_reader("\"String\"", Ok(String::from("String")));

    check_from_str_bytes_reader("r\"String\"", Ok(String::from("String")));
    check_from_str_bytes_reader("r#\"String\"#", Ok(String::from("String")));

    check_from_str_bytes_reader(
        "r#\"String with\nmultiple\nlines\n\"#",
        Ok(String::from("String with\nmultiple\nlines\n")),
    );

    check_from_str_bytes_reader(
        "r##\"String with \"#\"##",
        Ok(String::from("String with \"#")),
    );
}

#[test]
fn test_char() {
    check_from_str_bytes_reader("'c'", Ok('c'));
}

#[test]
fn test_escape_char() {
    check_from_str_bytes_reader("'\\''", Ok('\''));
}

#[test]
fn test_escape() {
    check_from_str_bytes_reader(r#""\"Quoted\"""#, Ok(String::from("\"Quoted\"")));
}

#[test]
fn test_comment() {
    check_from_str_bytes_reader(
        "(
x: 1.0, // x is just 1
// There is another comment in the very next line..
// And y is indeed
y: 2.0 // 2!
    )",
        Ok(MyStruct { x: 1.0, y: 2.0 }),
    );
}

fn err<T>(
    kind: Error,
    (line_start, col_start): (usize, usize),
    (line_end, col_end): (usize, usize),
) -> SpannedResult<T> {
    Err(SpannedError {
        code: kind,
        span: Span {
            start: Position {
                line: line_start,
                col: col_start,
            },
            end: Position {
                line: line_end,
                col: col_end,
            },
        },
    })
}

#[test]
fn test_err_wrong_value() {
    #[cfg(feature = "std")]
    use std::collections::HashMap;

    check_from_str_bytes_reader::<f32>("'c'", err(Error::ExpectedFloat, (1, 1), (1, 1)));
    check_from_str_bytes_reader::<String>("'c'", err(Error::ExpectedString, (1, 1), (1, 1)));
    #[cfg(feature = "std")]
    check_from_str_bytes_reader::<HashMap<u32, u32>>(
        "'c'",
        err(Error::ExpectedMap, (1, 1), (1, 1)),
    );
    check_from_str_bytes_reader::<[u8; 5]>("'c'", err(Error::ExpectedStructLike, (1, 1), (1, 1)));
    check_from_str_bytes_reader::<Vec<u32>>("'c'", err(Error::ExpectedArray, (1, 1), (1, 1)));
    check_from_str_bytes_reader::<MyEnum>("'c'", err(Error::ExpectedIdentifier, (1, 1), (1, 1)));
    check_from_str_bytes_reader::<MyStruct>(
        "'c'",
        err(Error::ExpectedNamedStructLike("MyStruct"), (1, 1), (1, 1)),
    );
    check_from_str_bytes_reader::<MyStruct>(
        "NotMyStruct(x: 4, y: 2)",
        err(
            Error::ExpectedDifferentStructName {
                expected: "MyStruct",
                found: String::from("NotMyStruct"),
            },
            (1, 1),
            (1, 12),
        ),
    );
    check_from_str_bytes_reader::<(u8, bool)>(
        "'c'",
        err(Error::ExpectedStructLike, (1, 1), (1, 1)),
    );
    check_from_str_bytes_reader::<bool>("notabool", err(Error::ExpectedBoolean, (1, 1), (1, 1)));

    let bogus_struct = "MyStruct(\n    x: true)";
    let expected_err = err(Error::ExpectedFloat, (2, 7), (2, 8));

    check_from_str_bytes_reader::<MyStruct>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<MyStruct>(bogus_struct, expected_err, " t");

    let bogus_struct = "MyStruct(\n    x: 3.5, \n    y:)";
    let expected_err = err(Error::ExpectedFloat, (3, 7), (3, 7));

    check_from_str_bytes_reader::<MyStruct>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<MyStruct>(bogus_struct, expected_err, ")");
}

#[test]
fn test_perm_ws() {
    check_from_str_bytes_reader(
        "\nMyStruct  \t ( \n x   : 3.5 , \t y\n: 4.5 \n ) \t\n",
        Ok(MyStruct { x: 3.5, y: 4.5 }),
    );
}

#[test]
fn untagged() {
    #[derive(Deserialize, Clone, Debug, PartialEq)]
    #[serde(untagged)]
    enum Untagged {
        U8(u8),
        Bool(bool),
        Value(crate::Value),
    }

    check_from_str_bytes_reader("true", Ok(Untagged::Bool(true)));
    check_from_str_bytes_reader("8", Ok(Untagged::U8(8)));

    // Check for a failure in Deserializer::check_struct_type
    // - untagged enum and a leading identifier trigger the serde content enum path
    // - serde content uses deserialize_any, which retriggers the struct type check
    // - struct type check inside a serde content performs a full newtype check
    // - newtype check fails on the unclosed struct
    //
    let bogus_struct = "Value(()";
    let expected_err = Err(crate::error::SpannedError {
        code: crate::Error::Eof,
        span: Span {
            start: Position { line: 1, col: 8 },
            end: crate::error::Position { line: 1, col: 9 },
        },
    });
    check_from_str_bytes_reader::<Untagged>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<Untagged>(bogus_struct, expected_err, ")");
}

#[test]
fn rename() {
    #[derive(Deserialize, Debug, PartialEq)]
    enum Foo {
        #[serde(rename = "2d")]
        D2,
        #[serde(rename = "triangle-list")]
        TriangleList,
    }

    check_from_str_bytes_reader("r#2d", Ok(Foo::D2));
    check_from_str_bytes_reader("r#triangle-list", Ok(Foo::TriangleList));
}

#[test]
fn forgot_apostrophes() {
    let bogus_struct = "(4, \"Hello)";
    let expected_err = Err(SpannedError {
        code: Error::ExpectedStringEnd,
        span: Span {
            start: Position { line: 1, col: 5 },
            end: Position { line: 1, col: 6 },
        },
    });

    check_from_str_bytes_reader::<(i32, String)>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<(i32, String)>(bogus_struct, expected_err, "\"");
}

#[test]
fn expected_attribute() {
    check_from_str_bytes_reader::<String>(
        "#\"Hello\"",
        err(Error::ExpectedAttribute, (1, 2), (1, 2)),
    );
}

#[test]
fn expected_attribute_end() {
    let bogus_struct = "#![enable(unwrap_newtypes) \"Hello\"";
    let expected_err = err(Error::ExpectedAttributeEnd, (1, 27), (1, 28));
    check_from_str_bytes_reader::<String>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_inclusive::<String>(bogus_struct, expected_err, " \"");
}

#[test]
fn invalid_attribute() {
    let bogus_struct = "#![enable(invalid)] \"Hello\"";
    let expected_err = err(
        Error::NoSuchExtension("invalid".to_string()),
        (1, 11),
        (1, 18),
    );
    check_from_str_bytes_reader::<String>(bogus_struct, expected_err.clone());

    #[cfg(feature = "internal-span-substring-test")]
    check_error_span_exclusive::<String>(bogus_struct, expected_err, "invalid");
}

#[test]
fn multiple_attributes() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct New(String);

    check_from_str_bytes_reader(
        "#![enable(unwrap_newtypes)] #![enable(unwrap_newtypes)] \"Hello\"",
        Ok(New("Hello".to_owned())),
    );
}

#[test]
fn type_schema_attributes() {
    check_from_str_bytes_reader::<String>(
        "#![type = \"my::Type\"] \"Hello\"",
        Ok("Hello".to_owned()),
    );
    check_from_str_bytes_reader::<String>(
        "#![schema = \"./schemas/app.schema.ron\"] \"Hello\"",
        Ok("Hello".to_owned()),
    );
}

#[test]
fn mixed_attributes() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct New(String);

    check_from_str_bytes_reader(
        "#![type = \"my::Type\"] #![enable(unwrap_newtypes)] \"Hello\"",
        Ok(New("Hello".to_owned())),
    );
}

#[test]
fn uglified_attribute() {
    check_from_str_bytes_reader(
        "#   !\
    // We definitely want to add a comment here
    [\t\tenable( // best style ever
            unwrap_newtypes  ) ] ()",
        Ok(()),
    );
}

#[test]
fn implicit_some() {
    use serde::de::DeserializeOwned;

    fn de<T: DeserializeOwned>(s: &str) -> Option<T> {
        let enable = "#![enable(implicit_some)]\n".to_string();

        super::from_str::<Option<T>>(&(enable + s)).unwrap()
    }

    assert_eq!(de("'c'"), Some('c'));
    assert_eq!(de("5"), Some(5));
    assert_eq!(de("\"Hello\""), Some("Hello".to_owned()));
    assert_eq!(de("false"), Some(false));
    assert_eq!(
        de("MyStruct(x: .4, y: .5)"),
        Some(MyStruct { x: 0.4, y: 0.5 })
    );

    assert_eq!(de::<char>("None"), None);

    // Not concise
    assert_eq!(de::<Option<Option<char>>>("None"), None);
}

#[test]
fn ws_tuple_newtype_variant() {
    check_from_str_bytes_reader("B  ( \n true \n ) ", Ok(MyEnum::B(true)));
}

#[test]
fn test_byte_stream() {
    check_from_str_bytes_reader(
        "BytesStruct( small:[1, 2], large:b\"\\x01\\x02\\x03\\x04\" )",
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 3, 4],
        }),
    );
}

#[test]
fn test_numbers() {
    check_from_str_bytes_reader(
        "[1_234, 12_345, 1_2_3_4_5_6, 1_234_567, 5_55_55_5]",
        Ok(vec![1234, 12345, 123_456, 1_234_567, 555_555]),
    );
}

fn check_de_any_number<
    T: Copy + PartialEq + core::fmt::Debug + Into<Number> + serde::de::DeserializeOwned,
>(
    s: &str,
    cmp: T,
) {
    let mut parser = Parser::new(s).unwrap();
    let number = parser.any_number().unwrap();

    assert_eq!(number, Number::new(cmp));
    assert_eq!(
        Number::new(super::from_str::<T>(s).unwrap()),
        Number::new(cmp)
    );
}

#[test]
fn test_any_number_precision() {
    check_de_any_number("1", 1_u8);
    check_de_any_number("+1", 1_u8);
    check_de_any_number("-1", -1_i8);
    check_de_any_number("-1.0", -1.0_f32);
    check_de_any_number("1.", 1.0_f32);
    check_de_any_number("-1.", -1.0_f32);
    check_de_any_number(".3", 0.3_f64);
    check_de_any_number("-.3", -0.3_f64);
    check_de_any_number("+.3", 0.3_f64);
    check_de_any_number("0.3", 0.3_f64);
    check_de_any_number("NaN", f32::NAN);
    check_de_any_number("-NaN", -f32::NAN);
    check_de_any_number("inf", f32::INFINITY);
    check_de_any_number("-inf", f32::NEG_INFINITY);

    macro_rules! test_min {
        ($($ty:ty),*) => {
            $(check_de_any_number(&format!("{}", <$ty>::MIN), <$ty>::MIN);)*
        };
    }

    macro_rules! test_max {
        ($($ty:ty),*) => {
            $(check_de_any_number(&format!("{}", <$ty>::MAX), <$ty>::MAX);)*
        };
    }

    test_min! { i8, i16, i32, i64, f64 }
    test_max! { u8, u16, u32, u64, f64 }
    #[cfg(feature = "integer128")]
    test_min! { i128 }
    #[cfg(feature = "integer128")]
    test_max! { u128 }
}

#[test]
fn test_value_special_floats() {
    use crate::{from_str, value::Number, Value};

    assert_eq!(
        from_str("NaN"),
        Ok(Value::Number(Number::F32(f32::NAN.into())))
    );
    assert_eq!(
        from_str("+NaN"),
        Ok(Value::Number(Number::F32(f32::NAN.into())))
    );
    assert_eq!(
        from_str("-NaN"),
        Ok(Value::Number(Number::F32((-f32::NAN).into())))
    );

    assert_eq!(
        from_str("inf"),
        Ok(Value::Number(Number::F32(f32::INFINITY.into())))
    );
    assert_eq!(
        from_str("+inf"),
        Ok(Value::Number(Number::F32(f32::INFINITY.into())))
    );
    assert_eq!(
        from_str("-inf"),
        Ok(Value::Number(Number::F32(f32::NEG_INFINITY.into())))
    );
}

#[test]
fn test_leading_whitespace() {
    check_from_str_bytes_reader("  +1", Ok(1_u8));
    check_from_str_bytes_reader("  EmptyStruct1", Ok(EmptyStruct1));
}

fn check_from_str_bytes_reader<T: serde::de::DeserializeOwned + PartialEq + core::fmt::Debug>(
    ron: &str,
    check: SpannedResult<T>,
) {
    let res_str = super::from_str::<T>(ron);
    assert_eq!(res_str, check);

    let res_bytes = super::from_bytes::<T>(ron.as_bytes());
    assert_eq!(res_bytes, check);

    #[cfg(feature = "std")]
    {
        let res_reader = super::from_reader::<&[u8], T>(ron.as_bytes());
        assert_eq!(res_reader, check);
    }
}

#[test]
fn test_remainder() {
    let mut deserializer = super::Deserializer::from_str("  42  ").unwrap();
    assert_eq!(
        <u8 as serde::Deserialize>::deserialize(&mut deserializer).unwrap(),
        42
    );
    assert_eq!(deserializer.remainder(), "  ");
    assert_eq!(deserializer.end(), Ok(()));

    let mut deserializer = super::Deserializer::from_str("  42 37 ").unwrap();
    assert_eq!(
        <u8 as serde::Deserialize>::deserialize(&mut deserializer).unwrap(),
        42
    );
    assert_eq!(deserializer.remainder(), " 37 ");
    assert_eq!(deserializer.end(), Err(Error::TrailingCharacters));
}

#[test]
fn boolean_struct_name() {
    check_from_str_bytes_reader::<bool>(
        "true_",
        Err(SpannedError {
            code: Error::ExpectedBoolean,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            },
        }),
    );
    check_from_str_bytes_reader::<bool>(
        "false_",
        Err(SpannedError {
            code: Error::ExpectedBoolean,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            },
        }),
    );
}
