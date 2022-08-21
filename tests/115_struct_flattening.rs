use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
struct Main {
    #[serde(flatten)]
    required: Required,
    #[serde(flatten)]
    optional: Optional,

    some_other_field: u32,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
struct Required {
    first: u32,
    second: u32,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
struct Optional {
    third: Option<u32>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
struct MyType {
    first: u32,
    second: u32,
    #[serde(flatten)]
    everything_else: HashMap<String, ron::Value>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
struct AllOptional {
    #[serde(flatten)]
    everything_else: HashMap<String, ron::Value>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
enum Newtype {
    Main(Main),
    MyType(MyType),
    AllOptional(AllOptional),
}

#[test]
fn test_flatten_struct_into_struct() {
    let ron = "Main(
        first: 1,
        second: 2,
        third: Some(3),
        some_other_field: 1337,
    )";

    let de: Main = ron::from_str(ron).unwrap();

    assert_eq!(
        de,
        Main {
            required: Required {
                first: 1,
                second: 2
            },
            optional: Optional { third: Some(3) },
            some_other_field: 1337,
        }
    );

    let ron = "
    #![enable(unwrap_variant_newtypes)]
    Main(
        first: 1,
        second: 2,
        third: Some(3),
        some_other_field: 1337,
    )";

    let de: Newtype = ron::from_str(ron).unwrap();

    assert_eq!(
        de,
        Newtype::Main(Main {
            required: Required {
                first: 1,
                second: 2
            },
            optional: Optional { third: Some(3) },
            some_other_field: 1337,
        })
    );
}

#[test]
fn test_flatten_rest() {
    let ron = "MyType(
        first: 1,
        second: 2,
        third: 3,
    )";

    let de: MyType = ron::from_str(ron).unwrap();

    assert_eq!(
        de,
        MyType {
            first: 1,
            second: 2,
            everything_else: {
                let mut map = HashMap::new();
                map.insert(
                    String::from("third"),
                    ron::Value::Number(ron::value::Number::from(3)),
                );
                map
            },
        }
    );

    let ron = "
    #![enable(unwrap_variant_newtypes)]
    MyType(
        first: 1,
        second: 2,
        third: 3,
    )";

    let de: Newtype = ron::from_str(ron).unwrap();

    assert_eq!(
        de,
        Newtype::MyType(MyType {
            first: 1,
            second: 2,
            everything_else: {
                let mut map = HashMap::new();
                map.insert(
                    String::from("third"),
                    ron::Value::Number(ron::value::Number::from(3)),
                );
                map
            },
        })
    )
}

#[test]
fn test_flatten_only_rest() {
    let ron = "AllOptional()";

    let de: AllOptional = ron::from_str(ron).unwrap();

    assert_eq!(
        de,
        AllOptional {
            everything_else: HashMap::new(),
        }
    );

    let ron = "#![enable(unwrap_variant_newtypes)] AllOptional()";

    let de: Newtype = ron::from_str(ron).unwrap();

    assert_eq!(
        de,
        Newtype::AllOptional(AllOptional {
            everything_else: HashMap::new(),
        })
    )
}
