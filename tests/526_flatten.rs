use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SomeCollection {
    inner: Vec<SomeItem>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SomeItem {
    #[serde(flatten)]
    foo: Foo,
    #[serde(flatten)]
    bar: Bar,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Bar {
    name: String,
    some_enum: Option<SomeEnum>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Foo {
    something: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum SomeEnum {
    A,
    B,
}

#[test]
fn roundtrip() {
    let scene = SomeCollection {
        inner: vec![SomeItem {
            foo: Foo {
                something: "something".to_string(),
            },
            bar: Bar {
                name: "name".to_string(),
                some_enum: Some(SomeEnum::A),
            },
        }],
    };

    let ron = ron::ser::to_string(&scene).unwrap();
    let de: SomeCollection = ron::de::from_str(&ron).unwrap();
    assert_eq!(de, scene);

    let ron = ron::ser::to_string_pretty(&scene, Default::default()).unwrap();
    let _deser_scene: SomeCollection = ron::de::from_str(&ron).unwrap();
    assert_eq!(de, scene);
}
