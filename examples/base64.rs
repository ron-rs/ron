//! ron initially encoded byte-slices and byte-bufs as base64-encoded strings.
//! However, since v0.9, ron now uses Rusty byte string literals instead.
//!
//! This example shows how the previous behaviour can be restored by serialising
//! bytes with strongly-typed base64-encoded strings, or accepting both Rusty
//! byte strings and the legacy base64-encoded string syntax.

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Config {
    #[serde(with = "ByteStr")]
    bytes: Vec<u8>,
    #[serde(with = "Base64")]
    base64: Vec<u8>,
    #[serde(with = "ByteStrOrBase64")]
    bytes_or_base64: Vec<u8>,
}

enum ByteStr {}

impl ByteStr {
    fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(data)
    }

    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        struct ByteStrVisitor;

        impl<'de> Visitor<'de> for ByteStrVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.write_str("a Rusty byte string")
            }

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
                Ok(bytes.to_vec())
            }

            fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<Self::Value, E> {
                Ok(bytes)
            }
        }

        deserializer.deserialize_byte_buf(ByteStrVisitor)
    }
}

enum Base64 {}

impl Base64 {
    fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&BASE64.encode(data))
    }

    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let base64_str = <&str>::deserialize(deserializer)?;
        BASE64.decode(base64_str).map_err(serde::de::Error::custom)
    }
}

enum ByteStrOrBase64 {}

impl ByteStrOrBase64 {
    fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        if cfg!(all()) {
            // either of these would work
            serializer.serialize_str(&BASE64.encode(data))
        } else {
            serializer.serialize_bytes(data)
        }
    }

    fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        struct ByteStrOrBase64Visitor;

        impl<'de> Visitor<'de> for ByteStrOrBase64Visitor {
            type Value = Vec<u8>;

            fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.write_str("a Rusty byte string or a base64-encoded string")
            }

            fn visit_str<E: serde::de::Error>(self, base64_str: &str) -> Result<Self::Value, E> {
                BASE64.decode(base64_str).map_err(serde::de::Error::custom)
            }

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
                Ok(bytes.to_vec())
            }

            fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<Self::Value, E> {
                Ok(bytes)
            }
        }

        deserializer.deserialize_any(ByteStrOrBase64Visitor)
    }
}

fn main() {
    let ron = r#"Config(
        bytes: b"only byte strings are allowed",
        base64: "b25seSBiYXNlNjQtZW5jb2RlZCBzdHJpbmdzIGFyZSBhbGxvd2Vk",
        bytes_or_base64: b"both byte strings and base64-encoded strings work",
    )"#;

    assert_eq!(
        ron::from_str::<Config>(ron).unwrap(),
        Config {
            bytes: b"only byte strings are allowed".to_vec(),
            base64: b"only base64-encoded strings are allowed".to_vec(),
            bytes_or_base64: b"both byte strings and base64-encoded strings work".to_vec()
        }
    );

    let ron = r#"Config(
        bytes: b"only byte strings are allowed",
        base64: "b25seSBiYXNlNjQtZW5jb2RlZCBzdHJpbmdzIGFyZSBhbGxvd2Vk",
        bytes_or_base64: "Ym90aCBieXRlIHN0cmluZ3MgYW5kIGJhc2U2NC1lbmNvZGVkIHN0cmluZ3Mgd29yaw==",
    )"#;

    assert_eq!(
        ron::from_str::<Config>(ron).unwrap(),
        Config {
            bytes: b"only byte strings are allowed".to_vec(),
            base64: b"only base64-encoded strings are allowed".to_vec(),
            bytes_or_base64: b"both byte strings and base64-encoded strings work".to_vec()
        }
    );

    println!(
        "{}",
        ron::ser::to_string_pretty(
            &Config {
                bytes: b"only byte strings are allowed".to_vec(),
                base64: b"only base64-encoded strings are allowed".to_vec(),
                bytes_or_base64: b"both byte strings and base64-encoded strings work".to_vec()
            },
            ron::ser::PrettyConfig::default().struct_names(true)
        )
        .unwrap()
    );
}
