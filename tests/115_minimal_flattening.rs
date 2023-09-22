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
    let val = Main {
        required: Required {
            first: 1,
            second: 2,
        },
        optional: Optional { third: Some(3) },
        some_other_field: 1337,
    };

    let ron = ron::ser::to_string_pretty(&val, ron::ser::PrettyConfig::default()).unwrap();

    assert_eq!(
        ron,
        "{
    \"first\": 1,
    \"second\": 2,
    \"third\": Some(3),
    \"some_other_field\": 1337,
}"
    );

    let de: Main = ron::from_str(&ron).unwrap();

    assert_eq!(de, val);

    let val = Newtype::Main(Main {
        required: Required {
            first: 1,
            second: 2,
        },
        optional: Optional { third: Some(3) },
        some_other_field: 1337,
    });

    let ron = ron::ser::to_string_pretty(
        &val,
        ron::ser::PrettyConfig::default()
            .extensions(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES),
    )
    .unwrap();

    assert_eq!(
        ron,
        "#![enable(unwrap_variant_newtypes)]
Main({
    \"first\": 1,
    \"second\": 2,
    \"third\": Some(3),
    \"some_other_field\": 1337,
})"
    );

    let ron = ron::ser::to_string_pretty(&val, ron::ser::PrettyConfig::default()).unwrap();

    let de: Newtype = ron::from_str(&ron).unwrap();

    assert_eq!(de, val);

    assert_eq!(
        ron::from_str::<Main>(
            "{
        first\": 1,
        \"second\": 2,
        \"third\": Some(3),
        \"some_other_field\": 1337,
    }"
        ),
        Err(ron::error::SpannedError {
            code: ron::error::Error::ExpectedString,
            position: ron::error::Position { line: 2, col: 9 },
        })
    );

    assert_eq!(
        ron::from_str::<Main>(
            "{
        \"first\": 1,
        \"second: 2,
        \"third\": Some(3),
        \"some_other_field\": 1337,
    }"
        ),
        Err(ron::error::SpannedError {
            code: ron::error::Error::ExpectedMapColon,
            position: ron::error::Position { line: 4, col: 10 },
        })
    );

    assert_eq!(
        ron::from_str::<Main>(
            "{
        \"first\": 1,
        \"second\": 2,
        third\": Some(3),
        \"some_other_field\": 1337,
    }"
        ),
        Err(ron::error::SpannedError {
            code: ron::error::Error::ExpectedString,
            position: ron::error::Position { line: 4, col: 9 },
        })
    );

    assert_eq!(
        ron::from_str::<Main>(
            "{
        \"first\": 1,
        \"second\": 2,
        \"third\": Some(3),
        \"some_other_field: 1337,
    }"
        ),
        Err(ron::error::SpannedError {
            code: ron::error::Error::ExpectedStringEnd,
            position: ron::error::Position { line: 5, col: 10 },
        })
    );
}

#[test]
fn test_flatten_rest() {
    let val = MyType {
        first: 1,
        second: 2,
        everything_else: {
            let mut map = HashMap::new();
            map.insert(
                String::from("third"),
                ron::Value::Number(ron::value::Number::U8(3)),
            );
            map
        },
    };

    let ron = ron::ser::to_string_pretty(&val, ron::ser::PrettyConfig::default()).unwrap();

    assert_eq!(
        ron,
        "{
    \"first\": 1,
    \"second\": 2,
    \"third\": 3,
}"
    );

    let de: MyType = ron::from_str(&ron).unwrap();

    assert_eq!(de, val);

    let val = Newtype::MyType(MyType {
        first: 1,
        second: 2,
        everything_else: {
            let mut map = HashMap::new();
            map.insert(
                String::from("third"),
                ron::Value::Number(ron::value::Number::U8(3)),
            );
            map
        },
    });

    let ron = ron::ser::to_string_pretty(
        &val,
        ron::ser::PrettyConfig::default()
            .extensions(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES),
    )
    .unwrap();

    assert_eq!(
        ron,
        "#![enable(unwrap_variant_newtypes)]
MyType({
    \"first\": 1,
    \"second\": 2,
    \"third\": 3,
})"
    );

    let de: Newtype = ron::from_str(&ron).unwrap();

    assert_eq!(de, val);

    assert_eq!(
        ron::from_str::<MyType>(
            "{
        first\": 1,
        \"second\": 2,
        \"third\": 3,
    }"
        ),
        Err(ron::error::SpannedError {
            code: ron::error::Error::ExpectedString,
            position: ron::error::Position { line: 2, col: 9 },
        })
    );

    assert_eq!(
        ron::from_str::<MyType>(
            "{
        \"first\": 1,
        \"second: 2,
        \"third\": 3,
    }"
        ),
        Err(ron::error::SpannedError {
            code: ron::error::Error::ExpectedMapColon,
            position: ron::error::Position { line: 4, col: 10 },
        })
    );

    assert_eq!(
        ron::from_str::<MyType>(
            "{
        \"first\": 1,
        \"second\": 2,
        third\": 3,
    }"
        ),
        Err(ron::error::SpannedError {
            code: ron::error::Error::ExpectedString,
            position: ron::error::Position { line: 4, col: 9 },
        })
    );

    assert_eq!(
        ron::from_str::<MyType>(
            "{
        \"first\": 1,
        \"second\": 2,
        \"third: 3,
    }"
        ),
        Err(ron::error::SpannedError {
            code: ron::error::Error::ExpectedStringEnd,
            position: ron::error::Position { line: 4, col: 10 },
        })
    );
}

#[test]
fn test_flatten_only_rest() {
    let val = AllOptional {
        everything_else: HashMap::new(),
    };

    let ron = ron::ser::to_string(&val).unwrap();

    assert_eq!(ron, "{}");

    let de: AllOptional = ron::from_str(&ron).unwrap();

    assert_eq!(de, val);

    let val = Newtype::AllOptional(AllOptional {
        everything_else: HashMap::new(),
    });

    let ron = ron::ser::to_string_pretty(
        &val,
        ron::ser::PrettyConfig::default()
            .extensions(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES),
    )
    .unwrap();

    assert_eq!(
        ron,
        "#![enable(unwrap_variant_newtypes)]
AllOptional({
})"
    );

    let de: Newtype = ron::from_str(&ron).unwrap();

    assert_eq!(de, val);
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AvailableCards {
    pub left: u8,
    pub right: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct MapProperties {
    #[serde(flatten)]
    cards: AvailableCards,
}

#[test]
fn test_issue_456() {
    let map_properties = MapProperties {
        cards: AvailableCards {
            ..Default::default()
        },
    };
    let ron = ron::to_string(&map_properties).unwrap();

    let de: MapProperties = ron::from_str(&ron).unwrap();

    assert_eq!(map_properties, de);
}
