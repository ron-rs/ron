use std::collections::BTreeMap;

use ron::ser::{to_string, to_string_pretty, PrettyConfig};

#[derive(serde_derive::Serialize)]
struct Struct {
    a: u8,
    b: u8,
}

#[test]
fn compact_structs() {
    let s = Struct { a: 4, b: 2 };

    assert_eq!(to_string(&s).unwrap(), "(a:4,b:2)");
    assert_eq!(
        to_string_pretty(&s, PrettyConfig::default()).unwrap(),
        "(\n    a: 4,\n    b: 2,\n)"
    );
    assert_eq!(
        to_string_pretty(&s, PrettyConfig::default().compact_structs(true)).unwrap(),
        "(a: 4, b: 2)"
    );
}

#[test]
fn compact_maps() {
    let m: BTreeMap<&str, i32> = BTreeMap::from_iter([("a", 4), ("b", 2)]);

    assert_eq!(to_string(&m).unwrap(), "{\"a\":4,\"b\":2}");
    assert_eq!(
        to_string_pretty(&m, PrettyConfig::default()).unwrap(),
        "{\n    \"a\": 4,\n    \"b\": 2,\n}"
    );
    assert_eq!(
        to_string_pretty(&m, PrettyConfig::default().compact_maps(true)).unwrap(),
        "{\"a\": 4, \"b\": 2}"
    );
}
