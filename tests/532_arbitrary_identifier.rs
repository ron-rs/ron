use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Event {
    #[serde(rename = "@sender")]
    sender: String,
}

#[test]
fn test_arbitary_identifier() {
    let ron = ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::ARBITRARY_IDENTIFIERS);
    let event = Event {
        sender: "test".to_string(),
    };
    let ser = ron.to_string(&event).unwrap();
    assert_eq!(ser, r#"(i"@sender":"test")"#);
    let de: Event = ron.from_str(&ser).unwrap();
    assert_eq!(de, event);
}

#[test]
fn test_arbitary_identifier_without_extension() {
    let ron = ron::Options::default();
    let event = Event {
        sender: "test".to_string(),
    };
    let ser = ron.to_string(&event).unwrap_err();
    // FIXME: assert_eq!(ser, ...);
    let de = ron
        .from_str::<Event>(r#"Event(i"@sender": "test")"#)
        .unwrap_err();
    // FIXME: assert_eq!(de, ...);
}
