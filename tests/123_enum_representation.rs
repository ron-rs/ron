use std::{cmp::PartialEq, fmt::Debug};

use ron::{de::from_str, ser::to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Unit;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Tuple(u8, u8);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Inner {
    Foo,
    Bar,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum EnumStructExternally {
    VariantA {
        foo: u32,
        bar: Unit,
        #[serde(with = "ByteStr")]
        baz: Vec<u8>,
        different: Tuple,
    },
    VariantB {
        foo: u32,
        bar: Unit,
        #[serde(with = "ByteStr")]
        baz: Vec<u8>,
    },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum EnumStructInternally {
    VariantA { foo: u32, bar: u32, different: u32 },
    VariantB { foo: u32, bar: u32 },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
enum EnumStructAdjacently {
    VariantA { foo: f64, bar: (), different: Inner },
    VariantB { foo: f64, bar: () },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum EnumStructUntagged {
    VariantA { foo: u32, bar: u32, different: u32 },
    VariantB { foo: u32, bar: u32 },
}

fn test_ser<T: Serialize>(value: &T, expected: &str) {
    let actual = to_string(value).expect("Failed to serialize");
    assert_eq!(actual, expected);
}

fn test_de<T>(s: &str, expected: T)
where
    T: for<'a> Deserialize<'a> + Debug + PartialEq,
{
    let actual: Result<T, _> = from_str(s);
    assert_eq!(actual, Ok(expected));
}

fn test_roundtrip<T>(value: T)
where
    T: Serialize + for<'a> Deserialize<'a> + Debug + PartialEq,
{
    let s = to_string(&value).expect("Failed to serialize");
    let actual: Result<T, _> = from_str(&s);
    assert_eq!(actual, Ok(value));
}

#[test]
fn test_externally_a_ser() {
    let v = EnumStructExternally::VariantA {
        foo: 1,
        bar: Unit,
        baz: vec![b'a'],
        different: Tuple(2, 3),
    };
    let e = "VariantA(foo:1,bar:(),baz:b\"a\",different:(2,3))";
    test_ser(&v, e);
}

#[test]
fn test_externally_b_ser() {
    let v = EnumStructExternally::VariantB {
        foo: 1,
        bar: Unit,
        baz: vec![b'a'],
    };
    let e = "VariantB(foo:1,bar:(),baz:b\"a\")";
    test_ser(&v, e);
}

#[test]
fn test_internally_a_ser() {
    let v = EnumStructInternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    let e = "(type:\"VariantA\",foo:1,bar:2,different:3)";
    test_ser(&v, e);
}

#[test]
fn test_internally_b_ser() {
    let v = EnumStructInternally::VariantB { foo: 1, bar: 2 };
    let e = "(type:\"VariantB\",foo:1,bar:2)";
    test_ser(&v, e);
}

#[test]
fn test_adjacently_a_ser() {
    let v = EnumStructAdjacently::VariantA {
        foo: 1.0,
        bar: (),
        different: Inner::Foo,
    };
    let e = "(type:VariantA,content:(foo:1.0,bar:(),different:Foo))";
    test_ser(&v, e);
}

#[test]
fn test_adjacently_b_ser() {
    let v = EnumStructAdjacently::VariantB { foo: 1.0, bar: () };
    let e = "(type:VariantB,content:(foo:1.0,bar:()))";
    test_ser(&v, e);
}

#[test]
fn test_untagged_a_ser() {
    let v = EnumStructUntagged::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    let e = "(foo:1,bar:2,different:3)";
    test_ser(&v, e);
}

#[test]
fn test_untagged_b_ser() {
    let v = EnumStructUntagged::VariantB { foo: 1, bar: 2 };
    let e = "(foo:1,bar:2)";
    test_ser(&v, e);
}

#[test]
fn test_externally_a_de() {
    let s = "VariantA(foo:1,bar:Unit,baz:b\"a\",different:Tuple(2,3))";
    let e = EnumStructExternally::VariantA {
        foo: 1,
        bar: Unit,
        baz: vec![b'a'],
        different: Tuple(2, 3),
    };
    test_de(s, e);
}

#[test]
fn test_externally_b_de() {
    let s = "VariantB(foo:1,bar:Unit,baz:b\"a\")";
    let e = EnumStructExternally::VariantB {
        foo: 1,
        bar: Unit,
        baz: vec![b'a'],
    };
    test_de(s, e);
}

#[test]
fn test_internally_a_de() {
    let s = "(type:\"VariantA\",foo:1,bar:2,different:3)";
    let e = EnumStructInternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_de(s, e);
}

#[test]
fn test_internally_b_de() {
    let s = "(type:\"VariantB\",foo:1,bar:2)";
    let e = EnumStructInternally::VariantB { foo: 1, bar: 2 };
    test_de(s, e);
}

#[test]
fn test_adjacently_a_de() {
    let s = "(type:VariantA,content:(foo:1.0,bar:(),different:Foo))";
    let e = EnumStructAdjacently::VariantA {
        foo: 1.0,
        bar: (),
        different: Inner::Foo,
    };
    test_de(s, e);
}

#[test]
fn test_adjacently_b_de() {
    let s = "(type:VariantB,content:(foo:1.0,bar:()))";
    let e = EnumStructAdjacently::VariantB { foo: 1.0, bar: () };
    test_de(s, e);
}

#[test]
fn test_untagged_a_de() {
    let s = "(foo:1,bar:2,different:3)";
    let e = EnumStructUntagged::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_de(s, e);
}

#[test]
fn test_untagged_b_de() {
    let s = "(foo:1,bar:2)";
    let e = EnumStructUntagged::VariantB { foo: 1, bar: 2 };
    test_de(s, e);
}

#[test]
fn test_externally_a_roundtrip() {
    let v = EnumStructExternally::VariantA {
        foo: 1,
        bar: Unit,
        baz: vec![b'a'],
        different: Tuple(2, 3),
    };
    test_roundtrip(v);
}

#[test]
fn test_externally_b_roundtrip() {
    let v = EnumStructExternally::VariantB {
        foo: 1,
        bar: Unit,
        baz: vec![b'a'],
    };
    test_roundtrip(v);
}

#[test]
fn test_internally_a_roundtrip() {
    let v = EnumStructInternally::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_roundtrip(v);
}

#[test]
fn test_internally_b_roundtrip() {
    let v = EnumStructInternally::VariantB { foo: 1, bar: 2 };
    test_roundtrip(v);
}

#[test]
fn test_adjacently_a_roundtrip() {
    let v = EnumStructAdjacently::VariantA {
        foo: 1.0,
        bar: (),
        different: Inner::Foo,
    };
    test_roundtrip(v);
}

#[test]
fn test_adjacently_b_roundtrip() {
    let v = EnumStructAdjacently::VariantB { foo: 1.0, bar: () };
    test_roundtrip(v);
}

#[test]
fn test_untagged_a_roundtrip() {
    let v = EnumStructUntagged::VariantA {
        foo: 1,
        bar: 2,
        different: 3,
    };
    test_roundtrip(v);
}

#[test]
fn test_untagged_b_roundtrip() {
    let v = EnumStructUntagged::VariantB { foo: 1, bar: 2 };
    test_roundtrip(v);
}

enum ByteStr {}

impl ByteStr {
    fn serialize<S: serde::Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(data)
    }

    fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        struct ByteStrVisitor;

        impl<'de> serde::de::Visitor<'de> for ByteStrVisitor {
            type Value = Vec<u8>;

            // GRCOV_EXCL_START
            fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.write_str("a Rusty byte string")
            }
            // GRCOV_EXCL_STOP

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
                Ok(bytes.to_vec())
            }
        }

        deserializer.deserialize_bytes(ByteStrVisitor)
    }
}
