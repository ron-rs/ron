use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum InnerEnum {
    Unit,
    Newtype(bool),
    Tuple(bool, i32),
    Struct { field: char },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Container {
    field: InnerEnum,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum OuterEnum {
    Variant(Container),
    Sum { field: InnerEnum, value: i32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "tag")]
enum OuterEnumInternal {
    Variant(Container),
    Sum { field: InnerEnum, value: i32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "tag", content = "c")]
enum OuterEnumAdjacent {
    Variant(Container),
    Sum { field: InnerEnum, value: i32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
enum OuterEnumUntagged {
    Variant(Container),
    Sum { field: InnerEnum, value: i32 },
}

#[test]
fn test_serde_content_hack() {
    assert_eq!(
        std::any::type_name::<serde::__private::de::Content>(),
        "serde::__private::de::content::Content"
    );
}

#[test]
fn test_serde_internally_tagged_hack() {
    const SERDE_CONTENT_CANARY: &str = "serde::__private::de::content::Content";
    const SERDE_TAG_KEY_CANARY: &str = "serde::__private::de::content::TagOrContent";

    struct Deserializer {
        tag_key: Option<String>,
        tag_value: String,
        field_key: Option<String>,
        field_value: i32,
    }

    impl<'de> serde::Deserializer<'de> for Deserializer {
        type Error = ron::Error;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::Visitor<'de>,
        {
            visitor.visit_map(self)
        }

        // GRCOV_EXCL_START
        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
        // GRCOV_EXCL_STOP
    }

    impl<'de> serde::de::MapAccess<'de> for Deserializer {
        type Error = ron::Error;

        fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
        where
            K: serde::de::DeserializeSeed<'de>,
        {
            assert_eq!(std::any::type_name::<K::Value>(), SERDE_TAG_KEY_CANARY);

            if let Some(tag_key) = self.tag_key.take() {
                return seed
                    .deserialize(serde::de::value::StringDeserializer::new(tag_key))
                    .map(Some);
            }

            if let Some(field_key) = self.field_key.take() {
                return seed
                    .deserialize(serde::de::value::StringDeserializer::new(field_key))
                    .map(Some);
            }

            Ok(None)
        }

        fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where
            V: serde::de::DeserializeSeed<'de>,
        {
            if self.field_key.is_some() {
                assert_ne!(std::any::type_name::<V::Value>(), SERDE_CONTENT_CANARY);
                return seed.deserialize(serde::de::value::StrDeserializer::new(&self.tag_value));
            }

            assert_eq!(std::any::type_name::<V::Value>(), SERDE_CONTENT_CANARY);

            seed.deserialize(serde::de::value::I32Deserializer::new(self.field_value))
        }
    }

    #[derive(PartialEq, Debug, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        A { hi: i32 },
    }

    assert_eq!(
        InternallyTagged::deserialize(Deserializer {
            tag_key: Some(String::from("tag")),
            tag_value: String::from("A"),
            field_key: Some(String::from("hi")),
            field_value: 42,
        }),
        Ok(InternallyTagged::A { hi: 42 })
    );
}

#[test]
fn test_enum_in_enum_roundtrip() {
    let outer = OuterEnum::Variant(Container {
        field: InnerEnum::Unit,
    });

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "Variant((field:Unit))");

    let de = ron::from_str::<OuterEnum>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnum::Sum {
        field: InnerEnum::Newtype(true),
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "Sum(field:Newtype(true),value:42)");

    let de = ron::from_str::<OuterEnum>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnum::Sum {
        field: InnerEnum::Tuple(true, 24),
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "Sum(field:Tuple(true,24),value:42)");

    let de = ron::from_str::<OuterEnum>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnum::Sum {
        field: InnerEnum::Struct { field: '🦀' },
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "Sum(field:Struct(field:'🦀'),value:42)");

    let de = ron::from_str::<OuterEnum>(&ron);

    assert_eq!(de, Ok(outer));
}

#[test]
fn test_enum_in_internally_tagged_roundtrip() {
    let outer = OuterEnumInternal::Variant(Container {
        field: InnerEnum::Unit,
    });

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:\"Variant\",field:Unit)");

    let de = ron::from_str::<OuterEnumInternal>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumInternal::Sum {
        field: InnerEnum::Newtype(true),
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:\"Sum\",field:Newtype(true),value:42)");

    let de = ron::from_str::<OuterEnumInternal>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumInternal::Sum {
        field: InnerEnum::Tuple(true, 24),
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:\"Sum\",field:Tuple(true,24),value:42)");

    let de = ron::from_str::<OuterEnumInternal>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumInternal::Sum {
        field: InnerEnum::Struct { field: '🦀' },
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:\"Sum\",field:Struct(field:'🦀'),value:42)");

    let de = ron::from_str::<OuterEnumInternal>(&ron);

    assert_eq!(de, Ok(outer));
}

#[test]
fn test_enum_in_adjacently_tagged_roundtrip() {
    let outer = OuterEnumAdjacent::Variant(Container {
        field: InnerEnum::Unit,
    });

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:Variant,c:(field:Unit))");

    let de = ron::from_str::<OuterEnumAdjacent>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumAdjacent::Sum {
        field: InnerEnum::Newtype(true),
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:Sum,c:(field:Newtype(true),value:42))");

    let de = ron::from_str::<OuterEnumAdjacent>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumAdjacent::Sum {
        field: InnerEnum::Tuple(true, 24),
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:Sum,c:(field:Tuple(true,24),value:42))");

    let de = ron::from_str::<OuterEnumAdjacent>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumAdjacent::Sum {
        field: InnerEnum::Struct { field: '🦀' },
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:Sum,c:(field:Struct(field:'🦀'),value:42))");

    let de = ron::from_str::<OuterEnumAdjacent>(&ron);

    assert_eq!(de, Ok(outer));
}

#[test]
fn test_enum_in_untagged_roundtrip() {
    let outer = OuterEnumUntagged::Variant(Container {
        field: InnerEnum::Unit,
    });

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(field:Unit)");

    let de = ron::from_str::<OuterEnumUntagged>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumUntagged::Sum {
        field: InnerEnum::Newtype(true),
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(field:Newtype(true),value:42)");

    let de = ron::from_str::<OuterEnumUntagged>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumUntagged::Sum {
        field: InnerEnum::Tuple(true, 24),
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(field:Tuple(true,24),value:42)");

    let de = ron::from_str::<OuterEnumUntagged>(&ron);

    assert_eq!(de, Ok(outer));

    let outer = OuterEnumUntagged::Sum {
        field: InnerEnum::Struct { field: '🦀' },
        value: 42,
    };

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(field:Struct(field:'🦀'),value:42)");

    let de = ron::from_str::<OuterEnumUntagged>(&ron);

    assert_eq!(de, Ok(outer));
}
