use std::collections::HashMap;

use ron::from_str;
use serde::Deserialize;

#[derive(Deserialize)]
#[allow(dead_code)]
struct Newtype(i32);

#[derive(Deserialize)]
#[allow(dead_code)]
struct Tuple(i32, i32);

#[derive(Deserialize)]
#[allow(dead_code)]
struct Struct {
    a: i32,
    b: i32,
}

#[derive(Deserialize)]
#[allow(dead_code)]
enum Enum {
    Newtype(i32),
    Tuple(i32, i32),
    Struct { a: i32, b: i32 },
}

#[test]
fn test_trailing_comma_some() {
    assert!(from_str::<Option<i32>>("Some(1)").is_ok());
    assert!(from_str::<Option<i32>>("Some(1,)").is_ok());
    assert!(from_str::<Option<i32>>("Some(1,,)").is_err());
}

#[test]
fn test_trailing_comma_tuple() {
    assert!(from_str::<(i32, i32)>("(1,2)").is_ok());
    assert!(from_str::<(i32, i32)>("(1,2,)").is_ok());
    assert!(from_str::<(i32, i32)>("(1,2,,)").is_err());
}

#[test]
fn test_trailing_comma_list() {
    assert!(from_str::<Vec<i32>>("[1,2]").is_ok());
    assert!(from_str::<Vec<i32>>("[1,2,]").is_ok());
    assert!(from_str::<Vec<i32>>("[1,2,,]").is_err());
}

#[test]
fn test_trailing_comma_map() {
    assert!(from_str::<HashMap<i32, bool>>("{1:false,2:true}").is_ok());
    assert!(from_str::<HashMap<i32, bool>>("{1:false,2:true,}").is_ok());
    assert!(from_str::<HashMap<i32, bool>>("{1:false,2:true,,}").is_err());
}

#[test]
fn test_trailing_comma_newtype_struct() {
    assert!(from_str::<Newtype>("(1)").is_ok());
    assert!(from_str::<Newtype>("(1,)").is_ok());
    assert!(from_str::<Newtype>("(1,,)").is_err());
}

#[test]
fn test_trailing_comma_tuple_struct() {
    assert!(from_str::<Tuple>("(1,2)").is_ok());
    assert!(from_str::<Tuple>("(1,2,)").is_ok());
    assert!(from_str::<Tuple>("(1,2,,)").is_err());
}

#[test]
fn test_trailing_comma_struct() {
    assert!(from_str::<Struct>("(a:1,b:2)").is_ok());
    assert!(from_str::<Struct>("(a:1,b:2,)").is_ok());
    assert!(from_str::<Struct>("(a:1,b:2,,)").is_err());
}

#[test]
fn test_trailing_comma_enum_newtype_variant() {
    assert!(from_str::<Enum>("Newtype(1)").is_ok());
    assert!(from_str::<Enum>("Newtype(1,)").is_ok());
    assert!(from_str::<Enum>("Newtype(1,,)").is_err());
}

#[test]
fn test_trailing_comma_enum_tuple_variant() {
    assert!(from_str::<Enum>("Tuple(1,2)").is_ok());
    assert!(from_str::<Enum>("Tuple(1,2,)").is_ok());
    assert!(from_str::<Enum>("Tuple(1,2,,)").is_err());
}

#[test]
fn test_trailing_comma_enum_struct_variant() {
    assert!(from_str::<Enum>("Struct(a:1,b:2)").is_ok());
    assert!(from_str::<Enum>("Struct(a:1,b:2,)").is_ok());
    assert!(from_str::<Enum>("Struct(a:1,b:2,,)").is_err());
}

#[test]
fn test_trailing_comma_untagged_newtype_variant() {
    #[derive(Debug, PartialEq, Deserialize)]
    enum Inner {
        Newtype(i32),
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Enum(Inner),
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(untagged)]
    enum Element {
        Int(i32),
        Tuple((Untagged,)),
    }

    assert_eq!(
        from_str::<Untagged>("Newtype(1)").unwrap(),
        Untagged::Enum(Inner::Newtype(1))
    );
    assert_eq!(
        from_str::<Untagged>("Newtype(1,)").unwrap(),
        Untagged::Enum(Inner::Newtype(1))
    );
    assert_eq!(
        from_str::<Untagged>("Newtype( 1 , )").unwrap(),
        Untagged::Enum(Inner::Newtype(1))
    );
    assert!(from_str::<Untagged>("Newtype(1,,)").is_err());

    // The `)` of the newtype variant must not be skipped and then taken from
    // the enclosing tuple, which would accept unbalanced parentheses
    assert!(from_str::<Vec<Element>>("[(Newtype(1,), 2]").is_err());
    assert_eq!(
        from_str::<Vec<Element>>("[(Newtype(1,),), 2]").unwrap(),
        vec![
            Element::Tuple((Untagged::Enum(Inner::Newtype(1)),)),
            Element::Int(2)
        ]
    );
}
