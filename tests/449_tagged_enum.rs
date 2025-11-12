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
