use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Other {
    Env(String),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum MaybeEnv {
    Value(String),
    Other(Other),
}

#[test]
fn enum_in_untagged_enum() {
    check_roundtrip(&MaybeEnv::Value(String::from("foo")), "\"foo\"");
    check_roundtrip(
        &MaybeEnv::Other(Other::Env(String::from("bar"))),
        "Env(\"bar\")",
    );
}

fn check_roundtrip<T: Serialize + serde::de::DeserializeOwned + std::fmt::Debug + PartialEq>(
    val: &T,
    check: &str,
) {
    let ron = ron::to_string(&val).unwrap();
    assert_eq!(ron, check);

    let de = ron::from_str::<T>(&ron).unwrap();
    assert_eq!(&de, val);
}
