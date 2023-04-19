use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum InnerEnum {
    UnitVariant,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Container {
    field: InnerEnum,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum OuterEnum {
    Variant(Container),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "tag")]
enum OuterEnumInternal {
    Variant(Container),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "tag", content = "c")]
enum OuterEnumAdjacent {
    Variant(Container),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
enum OuterEnumUntagged {
    Variant(Container),
}

#[test]
fn test_serde_content_hack() {
    assert_eq!(
        std::any::type_name::<serde::__private::de::Content>(),
        "serde::__private::de::content::Content"
    )
}

#[test]
fn test_enum_in_enum_roundtrip() {
    let outer = OuterEnum::Variant(Container {
        field: InnerEnum::UnitVariant,
    });

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "Variant((field:UnitVariant))");

    let de = ron::from_str::<OuterEnum>(&ron);

    assert_eq!(de, Ok(outer));
}

#[test]
fn test_enum_in_internally_tagged_roundtrip() {
    let outer = OuterEnumInternal::Variant(Container {
        field: InnerEnum::UnitVariant,
    });

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:\"Variant\",field:UnitVariant)");

    // Wrong JSONy RON would correctly deserialise here
    assert_eq!(
        ron::from_str::<OuterEnumInternal>("(tag:\"Variant\",field:\"UnitVariant\")").as_ref(),
        Ok(&outer)
    );

    let de = ron::from_str::<OuterEnumInternal>(&ron);

    assert_eq!(de, Ok(outer));
}

#[test]
fn test_enum_in_adjacently_tagged_roundtrip() {
    let outer = OuterEnumAdjacent::Variant(Container {
        field: InnerEnum::UnitVariant,
    });

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(tag:\"Variant\",c:(field:UnitVariant))");

    let de = ron::from_str::<OuterEnumAdjacent>(&ron);

    assert_eq!(de, Ok(outer));
}

#[test]
fn test_enum_in_untagged_roundtrip() {
    let outer = OuterEnumUntagged::Variant(Container {
        field: InnerEnum::UnitVariant,
    });

    let ron = ron::to_string(&outer).unwrap();

    assert_eq!(ron, "(field:UnitVariant)");

    // Wrong JSONy RON would correctly deserialise here
    assert_eq!(
        ron::from_str::<OuterEnumUntagged>("(field:\"UnitVariant\")").as_ref(),
        Ok(&outer)
    );

    let de = ron::from_str::<OuterEnumUntagged>(&ron);

    assert_eq!(de, Ok(outer));
}
