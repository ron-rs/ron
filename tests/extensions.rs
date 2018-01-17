extern crate ron;
#[macro_use]
extern crate serde;

use std::collections::HashMap;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct UnitStruct;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct NewType(f32);

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct TupleStruct(UnitStruct, i8);

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
struct Key(u32);

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
enum Enum {
    Unit,
    Bool(bool),
    Chars(char, String),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Struct {
    tuple: ((), NewType, TupleStruct),
    vec: Vec<Option<UnitStruct>>,
    map: HashMap<Key, Enum>,
}

const CONFIG_U_NT: &str = "
#![enable(unwrap_newtypes)]

(
    tuple: ((), 0.5, ((), -5)),
    vec: [
        None,
        Some(()),
    ],
    map: {
        7: Bool(true),
        9: Chars('x', \"\"),
        6: Bool(false),
        5: Unit,
    },
)
";

#[test]
fn unwrap_newtypes() {
    let d: Struct = ron::de::from_str(&CONFIG_U_NT).expect("Failed to deserialize");

    println!("unwrap_newtypes: {:#?}", d);
}
