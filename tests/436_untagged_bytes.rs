#[test]
fn test_serde_bytes() {
    #[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    #[serde(rename = "b")]
    struct BytesVal {
        pub b: serde_bytes::ByteBuf,
    }

    #[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    #[serde(untagged)]
    enum Bad {
        Bytes(BytesVal),
    }

    let s = ron::to_string(&serde_bytes::Bytes::new(b"test")).unwrap();

    assert_eq!(s, r#"b"test""#);

    let v: Bad = ron::from_str(r#"(b: b"test")"#).unwrap();

    assert_eq!(
        format!("{:?}", v),
        "Bytes(BytesVal { b: [116, 101, 115, 116] })"
    );

    let s = ron::to_string(&v).unwrap();

    assert_eq!(s, r#"(b:b"test")"#);
}

#[test]
fn test_bytes() {
    #[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    #[serde(rename = "b")]
    struct BytesVal {
        pub b: bytes::Bytes,
    }

    #[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    #[serde(untagged)]
    enum Bad {
        Bytes(BytesVal),
    }

    let s = ron::to_string(&bytes::Bytes::from("test")).unwrap();

    assert_eq!(s, r#"b"test""#);

    let v: Bad = ron::from_str(r#"(b: b"test")"#).unwrap();

    assert_eq!(format!("{:?}", v), r#"Bytes(BytesVal { b: b"test" })"#);

    let s = ron::to_string(&v).unwrap();

    assert_eq!(s, r#"(b:b"test")"#);
}

#[test]
fn test_strongly_typed_base64() {
    use base64::engine::{general_purpose::STANDARD as BASE64, Engine};

    enum Base64 {}

    impl Base64 {
        fn serialize<S: serde::Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&BASE64.encode(data))
        }

        fn deserialize<'de, D: serde::Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Vec<u8>, D::Error> {
            let base64_str: &str = serde::Deserialize::deserialize(deserializer)?;
            BASE64.decode(base64_str).map_err(serde::de::Error::custom)
        }
    }

    #[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    #[serde(rename = "b")]
    struct BytesVal {
        #[serde(with = "Base64")]
        pub b: Vec<u8>,
    }

    #[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    #[serde(untagged)]
    enum Bad {
        Bytes(BytesVal),
    }

    let v: Bad = ron::from_str(r#"(b: "dGVzdA==")"#).unwrap();

    assert_eq!(
        v,
        Bad::Bytes(BytesVal {
            b: b"test".to_vec()
        })
    );

    let s = ron::to_string(&v).unwrap();

    assert_eq!(s, r#"(b:"dGVzdA==")"#);
}
