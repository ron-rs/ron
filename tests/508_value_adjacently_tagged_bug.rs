use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
enum TheEnum {
    Variant([f32; 3]),
}

#[test]
fn roundtrip_through_value() {
    let value = TheEnum::Variant([0.1, 0.1, 0.1]);

    let ron = ron::to_string(&value).unwrap();
    assert_eq!(ron, "(type:Variant,value:(0.1,0.1,0.1))");

    let de = ron::from_str::<TheEnum>(&ron).unwrap();
    assert_eq!(de, value);

    let ron_value = ron::from_str::<ron::Value>(&ron).unwrap();

    // Known bug: ron::Value only stores a unit, cannot find a variant
    let err = ron_value.into_rust::<TheEnum>().unwrap_err();
    assert_eq!(
        err,
        ron::Error::InvalidValueForType {
            expected: String::from("variant of enum TheEnum"),
            found: String::from("a unit value")
        }
    );

    let old_serde_ron: &str = "(type:\"Variant\",value:(0.1,0.1,0.1))";

    // Known bug: serde no longer uses strings in > v1.0.180 to deserialize the variant
    let err = ron::from_str::<TheEnum>(&old_serde_ron).unwrap_err();
    assert_eq!(
        err,
        ron::error::SpannedError {
            code: ron::Error::ExpectedIdentifier,
            position: ron::error::Position { line: 1, col: 7 },
        }
    );

    let ron_value = ron::from_str::<ron::Value>(&old_serde_ron).unwrap();

    // Known bug: ron::Value is asked for an enum but has no special handling for it (yet)
    let err = ron_value.into_rust::<TheEnum>().unwrap_err();
    assert_eq!(
        err,
        ron::Error::InvalidValueForType {
            expected: String::from("variant of enum TheEnum"),
            found: String::from("the string \"Variant\"")
        }
    );

    // This still works, but is a bug as well
    let ron_value = ron::from_str::<ron::Value>("(\"Variant\",(0.1,0.1,0.1))").unwrap();
    let de: TheEnum = ron_value.into_rust::<TheEnum>().unwrap();
    assert_eq!(de, value);
}
