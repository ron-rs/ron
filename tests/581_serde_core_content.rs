use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
enum Message {
    Request {
        id: u32,
        resource: String,
        operation: String,
    },
    Response {
        id: u32,
        value: String,
    },
}

#[test]
fn internally_tagged_enum_serde_core_content_detection() {
    let value = Message::Response {
        id: 60069,
        value: "Foobar".into(),
    };
    let serialized = ron::to_string(&value).unwrap();
    let deserialized: Message = ron::from_str(&serialized).unwrap();
    assert_eq!(deserialized, value);
}
