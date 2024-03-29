use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct S {
    a: i8,
    b: i16,
    c: i32,
    d: i64,
    #[cfg(feature = "integer128")]
    e: i128,
    f: u8,
    g: u16,
    h: u32,
    i: u64,
    #[cfg(feature = "integer128")]
    j: u128,
}

#[test]
fn roundtrip() {
    let s = S {
        a: std::i8::MIN,
        b: std::i16::MIN,
        c: std::i32::MIN,
        d: std::i64::MIN,
        #[cfg(feature = "integer128")]
        e: std::i128::MIN,
        f: std::u8::MAX,
        g: std::u16::MAX,
        h: std::u32::MAX,
        i: std::u64::MAX,
        #[cfg(feature = "integer128")]
        j: std::u128::MAX,
    };
    let serialized = ron::ser::to_string(&s).unwrap();
    let deserialized = ron::de::from_str(&serialized).unwrap();
    assert_eq!(s, deserialized,);
}
