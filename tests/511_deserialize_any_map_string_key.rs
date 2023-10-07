#[test]
fn test_map_custom_deserialize() {
    use std::collections::HashMap;

    #[derive(PartialEq, Debug)]
    struct CustomMap(HashMap<String, String>);

    // Use a custom deserializer for CustomMap in order to extract String
    // keys in the visit_map method.
    impl<'de> serde::de::Deserialize<'de> for CustomMap {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct CVisitor;
            impl<'de> serde::de::Visitor<'de> for CVisitor {
                type Value = CustomMap;

                // GRCOV_EXCL_START
                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(formatter, "a map with string keys and values")
                }
                // GRCOV_EXCL_STOP

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>,
                {
                    let mut inner = HashMap::new();
                    while let Some((k, v)) = map.next_entry::<String, String>()? {
                        inner.insert(k, v);
                    }
                    Ok(CustomMap(inner))
                }
            }
            // Note: This method will try to deserialize any value. In this test, it will
            // invoke the visit_map method in the visitor.
            deserializer.deserialize_any(CVisitor)
        }
    }

    let mut map = HashMap::<String, String>::new();
    map.insert("key1".into(), "value1".into());
    map.insert("key2".into(), "value2".into());

    let result: Result<CustomMap, _> = ron::from_str(
        r#"(
            key1: "value1",
            key2: "value2",
        )"#,
    );

    assert_eq!(result, Ok(CustomMap(map)));
}

#[test]
fn test_ron_struct_as_json_map() {
    let json: serde_json::Value = ron::from_str("(f1: 0, f2: 1)").unwrap();
    assert_eq!(
        json,
        serde_json::Value::Object(
            [
                (
                    String::from("f1"),
                    serde_json::Value::Number(serde_json::Number::from(0))
                ),
                (
                    String::from("f2"),
                    serde_json::Value::Number(serde_json::Number::from(1))
                ),
            ]
            .into_iter()
            .collect()
        )
    );
}
