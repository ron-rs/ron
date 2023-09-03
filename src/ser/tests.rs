use serde_derive::Serialize;

use crate::Number;

#[derive(Serialize)]
struct EmptyStruct1;

#[derive(Serialize)]
struct EmptyStruct2 {}

#[derive(Serialize)]
struct NewType(i32);

#[derive(Serialize)]
struct TupleStruct(f32, f32);

#[derive(Serialize)]
struct MyStruct {
    x: f32,
    y: f32,
}

#[derive(Serialize)]
enum MyEnum {
    A,
    B(bool),
    C(bool, f32),
    D { a: i32, b: i32 },
}

#[test]
fn test_empty_struct() {
    check_to_string_writer(&EmptyStruct1, "()", "EmptyStruct1");
    check_to_string_writer(&EmptyStruct2 {}, "()", "EmptyStruct2()");
}

#[test]
fn test_struct() {
    let my_struct = MyStruct { x: 4.0, y: 7.0 };

    check_to_string_writer(&my_struct, "(x:4.0,y:7.0)", "MyStruct(x: 4.0, y: 7.0)");

    check_to_string_writer(&NewType(42), "(42)", "NewType(42)");

    check_to_string_writer(&TupleStruct(2.0, 5.0), "(2.0,5.0)", "TupleStruct(2.0, 5.0)");
}

#[test]
fn test_option() {
    check_to_string_writer(&Some(1u8), "Some(1)", "Some(1)");
    check_to_string_writer(&None::<u8>, "None", "None");
}

#[test]
fn test_enum() {
    check_to_string_writer(&MyEnum::A, "A", "A");
    check_to_string_writer(&MyEnum::B(true), "B(true)", "B(true)");
    check_to_string_writer(&MyEnum::C(true, 3.5), "C(true,3.5)", "C(true, 3.5)");
    check_to_string_writer(&MyEnum::D { a: 2, b: 3 }, "D(a:2,b:3)", "D(a: 2, b: 3)");
}

#[test]
fn test_array() {
    let empty: [i32; 0] = [];
    check_to_string_writer(&empty, "()", "()");
    let empty_ref: &[i32] = &empty;
    check_to_string_writer(&empty_ref, "[]", "[]");

    check_to_string_writer(&[2, 3, 4i32], "(2,3,4)", "(2, 3, 4)");
    check_to_string_writer(
        &(&[2, 3, 4i32] as &[i32]),
        "[2,3,4]",
        "[\n    2,\n    3,\n    4,\n]",
    );
}

#[test]
fn test_slice() {
    check_to_string_writer(
        &[0, 1, 2, 3, 4, 5][..],
        "[0,1,2,3,4,5]",
        "[\n    0,\n    1,\n    2,\n    3,\n    4,\n    5,\n]",
    );
    check_to_string_writer(
        &[0, 1, 2, 3, 4, 5][1..4],
        "[1,2,3]",
        "[\n    1,\n    2,\n    3,\n]",
    );
}

#[test]
fn test_vec() {
    check_to_string_writer(
        &vec![0, 1, 2, 3, 4, 5],
        "[0,1,2,3,4,5]",
        "[\n    0,\n    1,\n    2,\n    3,\n    4,\n    5,\n]",
    );
}

#[test]
fn test_map() {
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    map.insert((true, false), 4);
    map.insert((false, false), 123);

    check_to_string_writer(
        &map,
        "{(false,false):123,(true,false):4}",
        "{\n    (false, false): 123,\n    (true, false): 4,\n}",
    );
}

#[test]
fn test_string() {
    check_to_string_writer(&"Some string", "\"Some string\"", "\"Some string\"");
}

#[test]
fn test_char() {
    check_to_string_writer(&'c', "'c'", "'c'");
}

#[test]
fn test_escape() {
    check_to_string_writer(&r#""Quoted""#, r#""\"Quoted\"""#, r#""\"Quoted\"""#);
}

#[test]
fn test_byte_stream() {
    use serde_bytes;

    let small: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    check_to_string_writer(
        &small,
        "(0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15)",
        "(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15)",
    );

    let large = vec![0x01, 0x02, 0x03, 0x04];
    let large = serde_bytes::Bytes::new(&large);
    check_to_string_writer(
        &large,
        "b\"\\x01\\x02\\x03\\x04\"",
        "b\"\\x01\\x02\\x03\\x04\"",
    );

    let large = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
    let large = serde_bytes::Bytes::new(&large);
    check_to_string_writer(
        &large,
        "b\"\\x01\\x02\\x03\\x04\\x05\\x06\"",
        "b\"\\x01\\x02\\x03\\x04\\x05\\x06\"",
    );

    let large = vec![255u8; 64];
    let large = serde_bytes::Bytes::new(&large);
    check_to_string_writer(
        &large,
        concat!(
            "b\"\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff",
            "\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff",
            "\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff",
            "\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff",
            "\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\""
        ),
        concat!(
            "b\"\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff",
            "\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff",
            "\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff",
            "\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff",
            "\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\""
        ),
    );
}

#[test]
fn rename() {
    #[derive(Serialize, Debug, PartialEq)]
    enum Foo {
        #[serde(rename = "2d")]
        D2,
        #[serde(rename = "triangle-list")]
        TriangleList,
    }

    check_to_string_writer(&Foo::D2, "r#2d", "r#2d");
    check_to_string_writer(&Foo::TriangleList, "r#triangle-list", "r#triangle-list");
}

#[test]
fn test_any_number_precision() {
    check_ser_any_number(1_u8);
    check_ser_any_number(-1_i8);
    check_ser_any_number(1_f32);
    check_ser_any_number(-1_f32);
    check_ser_any_number(0.3_f64);

    check_to_string_writer(&Number::new(f32::NAN), "NaN", "NaN");
    check_to_string_writer(&f32::NAN, "NaN", "NaN");
    check_to_string_writer(&Number::new(-f32::NAN), "-NaN", "-NaN");
    check_to_string_writer(&(-f32::NAN), "-NaN", "-NaN");
    check_to_string_writer(&Number::new(f32::INFINITY), "inf", "inf");
    check_to_string_writer(&f32::INFINITY, "inf", "inf");
    check_to_string_writer(&Number::new(f32::NEG_INFINITY), "-inf", "-inf");
    check_to_string_writer(&f32::NEG_INFINITY, "-inf", "-inf");

    macro_rules! test_min_max {
        ($ty:ty) => {
            check_ser_any_number(<$ty>::MIN);
            check_ser_any_number(<$ty>::MAX);
        };
        ($($ty:ty),*) => {
            $(test_min_max! { $ty })*
        };
    }

    test_min_max! { i8, i16, i32, i64, u8, u16, u32, u64, f32, f64 }
    #[cfg(feature = "integer128")]
    test_min_max! { i128, u128 }
}

fn check_ser_any_number<T: Copy + Into<Number> + std::fmt::Display + serde::Serialize>(n: T) {
    let mut fmt = format!("{}", n);
    if !fmt.contains('.') && std::any::type_name::<T>().contains('f') {
        fmt.push_str(".0");
    }

    check_to_string_writer(&n.into(), &fmt, &fmt);
    check_to_string_writer(&n, &fmt, &fmt);
}

#[test]
fn recursion_limit() {
    assert_eq!(
        crate::Options::default()
            .with_recursion_limit(0)
            .to_string(&[42]),
        Err(crate::Error::ExceededRecursionLimit),
    );
    assert_eq!(
        crate::Options::default()
            .with_recursion_limit(1)
            .to_string(&[42])
            .as_deref(),
        Ok("(42)"),
    );
    assert_eq!(
        crate::Options::default()
            .without_recursion_limit()
            .to_string(&[42])
            .as_deref(),
        Ok("(42)"),
    );

    assert_eq!(
        crate::Options::default()
            .with_recursion_limit(1)
            .to_string(&[[42]]),
        Err(crate::Error::ExceededRecursionLimit),
    );
    assert_eq!(
        crate::Options::default()
            .with_recursion_limit(2)
            .to_string(&[[42]])
            .as_deref(),
        Ok("((42))"),
    );
    assert_eq!(
        crate::Options::default()
            .without_recursion_limit()
            .to_string(&[[42]])
            .as_deref(),
        Ok("((42))"),
    );
}

fn check_to_string_writer<T: ?Sized + serde::Serialize>(val: &T, check: &str, check_pretty: &str) {
    let ron_str = super::to_string(val).unwrap();
    assert_eq!(ron_str, check);

    let ron_str_pretty = super::to_string_pretty(
        val,
        super::PrettyConfig::default()
            .struct_names(true)
            .compact_structs(true),
    )
    .unwrap();
    assert_eq!(ron_str_pretty, check_pretty);

    let mut ron_writer = std::ffi::OsString::new();
    super::to_writer(&mut ron_writer, val).unwrap();
    assert_eq!(ron_writer, check);

    let mut ron_writer_pretty = std::ffi::OsString::new();
    super::to_writer_pretty(
        &mut ron_writer_pretty,
        val,
        super::PrettyConfig::default()
            .struct_names(true)
            .compact_structs(true),
    )
    .unwrap();
    assert_eq!(ron_writer_pretty, check_pretty);
}
