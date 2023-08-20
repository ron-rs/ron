use serde_bytes;
use serde_derive::Deserialize;

use crate::{
    de::from_str,
    error::{Error, Position, SpannedError, SpannedResult},
    parse::Bytes,
    value::Number,
};

#[derive(Debug, PartialEq, Deserialize)]
struct EmptyStruct1;

#[derive(Debug, PartialEq, Deserialize)]
struct EmptyStruct2 {}

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
    assert_eq!(Ok(EmptyStruct1), from_str("EmptyStruct1"));
    assert_eq!(Ok(EmptyStruct2 {}), from_str("EmptyStruct2()"));
}

#[test]
fn test_struct() {
    let my_struct = MyStruct { x: 4.0, y: 7.0 };

    assert_eq!(Ok(my_struct), from_str("MyStruct(x:4,y:7,)"));
    assert_eq!(Ok(my_struct), from_str("(x:4,y:7)"));

    #[derive(Debug, PartialEq, Deserialize)]
    struct NewType(i32);

    assert_eq!(Ok(NewType(42)), from_str("NewType(42)"));
    assert_eq!(Ok(NewType(33)), from_str("(33)"));

    #[derive(Debug, PartialEq, Deserialize)]
    struct TupleStruct(f32, f32);

    assert_eq!(Ok(TupleStruct(2.0, 5.0)), from_str("TupleStruct(2,5,)"));
    assert_eq!(Ok(TupleStruct(3.0, 4.0)), from_str("(3,4)"));
}

#[test]
fn test_option() {
    assert_eq!(Ok(Some(1u8)), from_str("Some(1)"));
    assert_eq!(Ok(None::<u8>), from_str("None"));
}

#[test]
fn test_enum() {
    assert_eq!(Ok(MyEnum::A), from_str("A"));
    assert_eq!(Ok(MyEnum::B(true)), from_str("B(true,)"));
    assert_eq!(Ok(MyEnum::C(true, 3.5)), from_str("C(true,3.5,)"));
    assert_eq!(Ok(MyEnum::D { a: 2, b: 3 }), from_str("D(a:2,b:3,)"));
}

#[test]
fn test_array() {
    let empty: [i32; 0] = [];
    assert_eq!(Ok(empty), from_str("()"));
    let empty_array = empty.to_vec();
    assert_eq!(Ok(empty_array), from_str("[]"));

    assert_eq!(Ok([2, 3, 4i32]), from_str("(2,3,4,)"));
    assert_eq!(Ok([2, 3, 4i32].to_vec()), from_str("[2,3,4,]"));
}

#[test]
fn test_map() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert((true, false), 4);
    map.insert((false, false), 123);

    assert_eq!(
        Ok(map),
        from_str(
            "{
        (true,false,):4,
        (false,false,):123,
    }"
        )
    );
}

#[test]
fn test_string() {
    let s: String = from_str("\"String\"").unwrap();
    assert_eq!("String", s);

    let raw: String = from_str("r\"String\"").unwrap();
    assert_eq!("String", raw);

    let raw_hashes: String = from_str("r#\"String\"#").unwrap();
    assert_eq!("String", raw_hashes);

    let raw_hashes_multiline: String = from_str("r#\"String with\nmultiple\nlines\n\"#").unwrap();
    assert_eq!("String with\nmultiple\nlines\n", raw_hashes_multiline);

    let raw_hashes_quote: String = from_str("r##\"String with \"#\"##").unwrap();
    assert_eq!("String with \"#", raw_hashes_quote);
}

#[test]
fn test_char() {
    assert_eq!(Ok('c'), from_str("'c'"));
}

#[test]
fn test_escape_char() {
    assert_eq!('\'', from_str::<char>("'\\''").unwrap());
}

#[test]
fn test_escape() {
    assert_eq!("\"Quoted\"", from_str::<String>(r#""\"Quoted\"""#).unwrap());
}

#[test]
fn test_comment() {
    assert_eq!(
        MyStruct { x: 1.0, y: 2.0 },
        from_str(
            "(
x: 1.0, // x is just 1
// There is another comment in the very next line..
// And y is indeed
y: 2.0 // 2!
    )"
        )
        .unwrap()
    );
}

fn err<T>(kind: Error, line: usize, col: usize) -> SpannedResult<T> {
    Err(SpannedError {
        code: kind,
        position: Position { line, col },
    })
}

#[test]
fn test_err_wrong_value() {
    use std::collections::HashMap;

    use self::Error::*;

    assert_eq!(from_str::<f32>("'c'"), err(ExpectedFloat, 1, 1));
    assert_eq!(from_str::<String>("'c'"), err(ExpectedString, 1, 1));
    assert_eq!(from_str::<HashMap<u32, u32>>("'c'"), err(ExpectedMap, 1, 1));
    assert_eq!(from_str::<[u8; 5]>("'c'"), err(ExpectedStructLike, 1, 1));
    assert_eq!(from_str::<Vec<u32>>("'c'"), err(ExpectedArray, 1, 1));
    assert_eq!(from_str::<MyEnum>("'c'"), err(ExpectedIdentifier, 1, 1));
    assert_eq!(
        from_str::<MyStruct>("'c'"),
        err(ExpectedNamedStructLike("MyStruct"), 1, 1)
    );
    assert_eq!(
        from_str::<MyStruct>("NotMyStruct(x: 4, y: 2)"),
        err(
            ExpectedDifferentStructName {
                expected: "MyStruct",
                found: String::from("NotMyStruct")
            },
            1,
            12
        )
    );
    assert_eq!(from_str::<(u8, bool)>("'c'"), err(ExpectedStructLike, 1, 1));
    assert_eq!(from_str::<bool>("notabool"), err(ExpectedBoolean, 1, 1));

    assert_eq!(
        from_str::<MyStruct>("MyStruct(\n    x: true)"),
        err(ExpectedFloat, 2, 8)
    );
    assert_eq!(
        from_str::<MyStruct>("MyStruct(\n    x: 3.5, \n    y:)"),
        err(ExpectedFloat, 3, 7)
    );
}

#[test]
fn test_perm_ws() {
    assert_eq!(
        from_str::<MyStruct>("\nMyStruct  \t ( \n x   : 3.5 , \t y\n: 4.5 \n ) \t\n"),
        Ok(MyStruct { x: 3.5, y: 4.5 })
    );
}

#[test]
fn untagged() {
    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(untagged)]
    enum Untagged {
        U8(u8),
        Bool(bool),
    }

    assert_eq!(from_str::<Untagged>("true").unwrap(), Untagged::Bool(true));
    assert_eq!(from_str::<Untagged>("8").unwrap(), Untagged::U8(8));
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
    assert_eq!(from_str::<Foo>("r#2d").unwrap(), Foo::D2);
    assert_eq!(
        from_str::<Foo>("r#triangle-list").unwrap(),
        Foo::TriangleList
    );
}

#[test]
fn forgot_apostrophes() {
    let de: SpannedResult<(i32, String)> = from_str("(4, \"Hello)");

    assert!(matches!(
        de,
        Err(SpannedError {
            code: Error::ExpectedStringEnd,
            position: _,
        })
    ));
}

#[test]
fn expected_attribute() {
    let de: SpannedResult<String> = from_str("#\"Hello\"");

    assert_eq!(de, err(Error::ExpectedAttribute, 1, 2));
}

#[test]
fn expected_attribute_end() {
    let de: SpannedResult<String> = from_str("#![enable(unwrap_newtypes) \"Hello\"");

    assert_eq!(de, err(Error::ExpectedAttributeEnd, 1, 28));
}

#[test]
fn invalid_attribute() {
    let de: SpannedResult<String> = from_str("#![enable(invalid)] \"Hello\"");

    assert_eq!(
        de,
        err(Error::NoSuchExtension("invalid".to_string()), 1, 18)
    );
}

#[test]
fn multiple_attributes() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct New(String);
    let de: SpannedResult<New> =
        from_str("#![enable(unwrap_newtypes)] #![enable(unwrap_newtypes)] \"Hello\"");

    assert_eq!(de, Ok(New("Hello".to_owned())));
}

#[test]
fn uglified_attribute() {
    let de: SpannedResult<()> = from_str(
        "#   !\
    // We definitely want to add a comment here
    [\t\tenable( // best style ever
            unwrap_newtypes  ) ] ()",
    );

    assert_eq!(de, Ok(()));
}

#[test]
fn implicit_some() {
    use serde::de::DeserializeOwned;

    fn de<T: DeserializeOwned>(s: &str) -> Option<T> {
        let enable = "#![enable(implicit_some)]\n".to_string();

        from_str::<Option<T>>(&(enable + s)).unwrap()
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
    assert_eq!(Ok(MyEnum::B(true)), from_str("B  ( \n true \n ) "));
}

#[test]
fn test_byte_stream() {
    assert_eq!(
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 3, 4]
        }),
        from_str("BytesStruct( small:[1, 2], large:\"AQIDBA==\" )"),
    );
}

#[test]
fn test_numbers() {
    assert_eq!(
        Ok(vec![1234, 12345, 123456, 1234567, 555_555]),
        from_str("[1_234, 12_345, 1_2_3_4_5_6, 1_234_567, 5_55_55_5]"),
    );
}

fn check_de_any_number<
    T: Copy + PartialEq + std::fmt::Debug + Into<Number> + serde::de::DeserializeOwned,
>(
    s: &str,
    cmp: T,
) {
    let mut bytes = Bytes::new(s.as_bytes()).unwrap();
    let number = bytes.any_number().unwrap();

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
