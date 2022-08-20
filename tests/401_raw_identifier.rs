use ron::error::{Error, Position, SpannedError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "Hello World")]
struct InvalidStruct;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "")]
struct EmptyStruct;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename = "Hello+World")]
struct RawStruct {
    #[serde(rename = "ab.cd-ef")]
    field: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
enum RawEnum {
    #[serde(rename = "Hello-World")]
    RawVariant,
}

#[test]
fn test_invalid_identifiers() {
    let ser = ron::ser::to_string_pretty(
        &InvalidStruct,
        ron::ser::PrettyConfig::default().struct_names(true),
    );
    assert_eq!(
        ser,
        Err(Error::InvalidIdentifier(String::from("Hello World")))
    );

    let ser = ron::ser::to_string_pretty(
        &EmptyStruct,
        ron::ser::PrettyConfig::default().struct_names(true),
    );
    assert_eq!(ser, Err(Error::InvalidIdentifier(String::from(""))));

    let de = ron::from_str::<InvalidStruct>("Hello World").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: Error::ExpectedDifferentStructName {
                expected: "Hello World",
                found: String::from("Hello"),
            },
            position: Position { line: 1, col: 6 },
        }
    );

    let de = ron::from_str::<EmptyStruct>("").unwrap_err();
    assert_eq!(
        de,
        SpannedError {
            code: Error::ExpectedUnit,
            position: Position { line: 1, col: 1 },
        }
    );
}

#[test]
fn test_raw_identifier_roundtrip() {
    let val = RawStruct { field: true };

    let ser =
        ron::ser::to_string_pretty(&val, ron::ser::PrettyConfig::default().struct_names(true))
            .unwrap();
    assert_eq!(ser, "r#Hello+World(\n    r#ab.cd-ef: true,\n)");

    let de: RawStruct = ron::from_str(&ser).unwrap();
    assert_eq!(de, val);

    let val = RawEnum::RawVariant;

    let ser = ron::ser::to_string(&val).unwrap();
    assert_eq!(ser, "r#Hello-World");

    let de: RawEnum = ron::from_str(&ser).unwrap();
    assert_eq!(de, val);
}
