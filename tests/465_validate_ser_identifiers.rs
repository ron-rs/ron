use serde::ser::{SerializeStruct, SerializeStructVariant, Serializer};

#[test]
fn invalid_struct_name() {
    let mut ser = ron::Serializer::new(String::new(), None).unwrap();

    assert_eq!(
        ser.serialize_newtype_struct("", &true).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_tuple_struct("", 0).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_struct("", 0).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );
}

#[test]
fn invalid_enum_variant_name() {
    let mut ser = ron::Serializer::new(String::new(), None).unwrap();

    assert_eq!(
        ser.serialize_unit_variant("", 0, "A").err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_unit_variant("A", 0, "").err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_newtype_variant("", 0, "A", &true).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_newtype_variant("A", 0, "", &true).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_tuple_variant("", 0, "A", 0).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_tuple_variant("A", 0, "", 0).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_struct_variant("", 0, "A", 0).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    assert_eq!(
        ser.serialize_struct_variant("A", 0, "", 0).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );
}

#[test]
fn invalid_struct_field_name() {
    let mut ser = ron::Serializer::new(String::new(), None).unwrap();

    let mut r#struct = ser.serialize_struct("A", 2).unwrap();
    SerializeStruct::serialize_field(&mut r#struct, "A", &true).unwrap();

    assert_eq!(
        SerializeStruct::serialize_field(&mut r#struct, "", &true).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );

    std::mem::drop(r#struct);

    let mut r#struct = ser.serialize_struct_variant("A", 0, "A", 2).unwrap();
    SerializeStructVariant::serialize_field(&mut r#struct, "A", &true).unwrap();

    assert_eq!(
        SerializeStructVariant::serialize_field(&mut r#struct, "", &true).err(),
        Some(ron::Error::InvalidIdentifier(String::from(""))),
    );
}
