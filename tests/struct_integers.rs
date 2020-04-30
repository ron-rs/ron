use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct S {
    a: i8,
    b: i16,
    c: i32,
    d: i64,
    e: i128,
    f: u8,
    g: u16,
    h: u32,
    i: u64,
    j: u128,
}

#[test]
fn roundtrip() {
    let s = S {
        a: i8::MIN,
        b: i16::MIN,
        c: i32::MIN,
        d: i64::MIN,
        e: i128::MIN,
        f: u8::MAX,
        g: u16::MAX,
        h: u32::MAX,
        i: u64::MAX,
        j: u128::MAX,
    };
    let serialized = ron::ser::to_string(&s).unwrap();
    dbg!(&serialized);
    let deserialized = ron::de::from_str(&serialized).unwrap();
    assert_eq!(s, deserialized,);
}
