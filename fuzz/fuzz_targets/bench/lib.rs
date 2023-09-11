use std::convert::TryFrom;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use arbitrary::{Arbitrary, Unstructured};
use serde::de::{DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserializer;
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Deserialize, Serialize, Serializer,
};

use ron::{extensions::Extensions, ser::PrettyConfig};

const RECURSION_LIMIT: usize = 32_usize;
const LONG_NAME_COST_THRESHOLD: usize = 8_usize;

const ARRAY_UNINIT_LEN: usize = usize::MAX;

pub fn roundtrip_arbitrary_typed_ron_or_panic(data: &[u8]) -> Option<TypedSerdeData> {
    if let Ok(typed_value) = TypedSerdeData::arbitrary(&mut Unstructured::new(data)) {
        let options = ron::Options::default().with_recursion_limit(RECURSION_LIMIT);

        let ron = match options.to_string_pretty(&typed_value, typed_value.pretty_config()) {
            Ok(ron) => ron,
            // Erroring on deep recursion is better than crashing on a stack overflow
            Err(ron::error::Error::ExceededRecursionLimit) => return None,
            Err(ron::error::Error::Message(msg))
                if msg == format!("{}", ron::error::Error::ExceededRecursionLimit) =>
            {
                return None
            }
            // We want the fuzzer to try to generate valid identifiers
            Err(ron::error::Error::InvalidIdentifier(_)) => return None,
            Err(ron::error::Error::Message(msg))
                if msg.starts_with("Invalid identifier \"") && msg.ends_with('"') =>
            {
                return None
            }
            // The fuzzer can find this code path (lol) but give the wrong data
            Err(ron::error::Error::ExpectedRawValue) => return None,
            Err(ron::error::Error::Message(msg))
                if msg == format!("{}", ron::error::Error::ExpectedRawValue) =>
            {
                return None
            }
            // Internally tagged newtype variants have requirements only checked at serialize time
            Err(ron::error::Error::Message(msg))
                if msg.starts_with("cannot serialize tagged newtype variant ") =>
            {
                return None
            }
            // Everything else is actually a bug we want to find
            Err(err) => panic!("{:?} -! {:?}", typed_value, err),
        };

        if let Err(err) = options.from_str::<ron::Value>(&ron) {
            match err.code {
                // Erroring on deep recursion is better than crashing on a stack overflow
                ron::error::Error::ExceededRecursionLimit => return None,
                // Everything else is actually a bug we want to find
                _ => panic!("{:?} -> {} -! {:?}", typed_value, ron, err),
            }
        };

        if let Err(err) = options.from_str_seed(&ron, &typed_value) {
            match err.code {
                // Erroring on deep recursion is better than crashing on a stack overflow
                ron::error::Error::ExceededRecursionLimit => return None,
                // Duplicate struct fields only cause issues inside internally tagged
                //  or untagged enums, so we allow them otherwise
                ron::error::Error::DuplicateStructField { .. } => return None,
                // Everything else is actually a bug we want to find
                _ => panic!("{:?} -> {} -! {:?}", typed_value, ron, err),
            }
        };

        Some(typed_value)
    } else {
        None
    }
}

// NOTE: Keep synchronised with ron::value::raw::RAW_VALUE_TOKEN
const RAW_VALUE_TOKEN: &str = "$ron::private::RawValue";

#[derive(Debug, PartialEq, Arbitrary)]
struct ArbitraryPrettyConfig {
    /// Limit the pretty-ness up to the given depth.
    depth_limit: u8,
    // Whether to emit struct names
    struct_names: bool,
    /// Separate tuple members with indentation
    separate_tuple_members: bool,
    /// Enumerate array items in comments
    enumerate_arrays: bool,
    #[arbitrary(with = arbitrary_ron_extensions)]
    /// Enable extensions. Only configures 'implicit_some',
    ///  'unwrap_newtypes', and 'unwrap_variant_newtypes' for now.
    extensions: Extensions,
    /// Enable compact arrays, which do not insert new lines and indentation
    ///  between the elements of an array
    compact_arrays: bool,
    /// Whether to serialize strings as escaped strings,
    ///  or fall back onto raw strings if necessary.
    escape_strings: bool,
    /// Enable compact structs, which do not insert new lines and indentation
    ///  between the fields of a struct
    compact_structs: bool,
    /// Enable compact maps, which do not insert new lines and indentation
    ///  between the entries of a struct
    compact_maps: bool,
    /// Enable explicit number type suffixes like `1u16`
    number_suffixes: bool,
}

fn arbitrary_ron_extensions(u: &mut Unstructured) -> arbitrary::Result<Extensions> {
    Extensions::from_bits(usize::arbitrary(u)?).ok_or(arbitrary::Error::IncorrectFormat)
}

impl From<ArbitraryPrettyConfig> for PrettyConfig {
    fn from(arbitrary: ArbitraryPrettyConfig) -> Self {
        Self::default()
            .depth_limit((arbitrary.depth_limit % 16).into())
            .indentor(String::from(" ")) // conserve some memory and time
            .struct_names(arbitrary.struct_names)
            .separate_tuple_members(arbitrary.separate_tuple_members)
            .enumerate_arrays(arbitrary.enumerate_arrays)
            .extensions(arbitrary.extensions)
            .compact_arrays(arbitrary.compact_arrays)
            .escape_strings(arbitrary.escape_strings)
            .compact_structs(arbitrary.compact_structs)
            .compact_maps(arbitrary.compact_maps)
            .number_suffixes(arbitrary.number_suffixes)
    }
}

#[derive(Debug, PartialEq)]
pub struct TypedSerdeData<'a> {
    pretty_config: PrettyConfig,
    ty: SerdeDataType<'a>,
    value: SerdeDataValue<'a>,
}

impl<'a> TypedSerdeData<'a> {
    #[allow(dead_code)]
    pub fn pretty_config(&self) -> PrettyConfig {
        self.pretty_config.clone()
    }

    pub fn ty(&self) -> &SerdeDataType<'a> {
        &self.ty
    }

    pub fn value(&self) -> &SerdeDataValue<'a> {
        &self.value
    }
}

struct BorrowedTypedSerdeData<'a> {
    ty: &'a SerdeDataType<'a>,
    value: &'a SerdeDataValue<'a>,
}

impl<'a> Serialize for TypedSerdeData<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        BorrowedTypedSerdeData {
            ty: &self.ty,
            value: &self.value,
        }
        .serialize(serializer)
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &TypedSerdeData<'a> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        BorrowedTypedSerdeData {
            ty: &self.ty,
            value: &self.value,
        }
        .deserialize(deserializer)
    }
}

unsafe fn to_static_str(s: &str) -> &'static str {
    std::mem::transmute(s)
}

unsafe fn to_static_str_slice(s: &[&str]) -> &'static [&'static str] {
    std::mem::transmute(s)
}

impl<'a> Serialize for BorrowedTypedSerdeData<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match (self.ty, self.value) {
            (SerdeDataType::Unit, SerdeDataValue::Unit) => ().serialize(serializer),
            (SerdeDataType::Bool, SerdeDataValue::Bool(v)) => v.serialize(serializer),
            (SerdeDataType::I8, SerdeDataValue::I8(v)) => v.serialize(serializer),
            (SerdeDataType::I16, SerdeDataValue::I16(v)) => v.serialize(serializer),
            (SerdeDataType::I32, SerdeDataValue::I32(v)) => v.serialize(serializer),
            (SerdeDataType::I64, SerdeDataValue::I64(v)) => v.serialize(serializer),
            (SerdeDataType::I128, SerdeDataValue::I128(v)) => v.serialize(serializer),
            (SerdeDataType::ISize, SerdeDataValue::ISize(v)) => v.serialize(serializer),
            (SerdeDataType::U8, SerdeDataValue::U8(v)) => v.serialize(serializer),
            (SerdeDataType::U16, SerdeDataValue::U16(v)) => v.serialize(serializer),
            (SerdeDataType::U32, SerdeDataValue::U32(v)) => v.serialize(serializer),
            (SerdeDataType::U64, SerdeDataValue::U64(v)) => v.serialize(serializer),
            (SerdeDataType::U128, SerdeDataValue::U128(v)) => v.serialize(serializer),
            (SerdeDataType::USize, SerdeDataValue::USize(v)) => v.serialize(serializer),
            (SerdeDataType::F32, SerdeDataValue::F32(v)) => v.serialize(serializer),
            (SerdeDataType::F64, SerdeDataValue::F64(v)) => v.serialize(serializer),
            (SerdeDataType::Char, SerdeDataValue::Char(v)) => v.serialize(serializer),
            (SerdeDataType::String, SerdeDataValue::String(v)) => v.serialize(serializer),
            (SerdeDataType::ByteBuf, SerdeDataValue::ByteBuf(v)) => serializer.serialize_bytes(v),
            (SerdeDataType::Option { inner: ty }, SerdeDataValue::Option { inner: value }) => {
                if let Some(value) = value {
                    serializer.serialize_some(&BorrowedTypedSerdeData { ty, value })
                } else {
                    serializer.serialize_none()
                }
            }
            (SerdeDataType::Array { kind, len }, SerdeDataValue::Seq { elems }) => {
                if elems.len() != *len {
                    return Err(serde::ser::Error::custom("mismatch array len"));
                }

                let mut array = serializer.serialize_tuple(*len)?;
                for elem in elems {
                    array.serialize_element(&BorrowedTypedSerdeData {
                        ty: kind,
                        value: elem,
                    })?;
                }
                array.end()
            }
            (SerdeDataType::Tuple { elems: tys }, SerdeDataValue::Seq { elems: values }) => {
                if values.len() != tys.len() {
                    return Err(serde::ser::Error::custom("mismatch tuple len"));
                }

                let mut tuple = serializer.serialize_tuple(tys.len())?;
                for (ty, data) in tys.iter().zip(values.iter()) {
                    tuple.serialize_element(&BorrowedTypedSerdeData { ty, value: data })?;
                }
                tuple.end()
            }
            (SerdeDataType::Vec { item: ty }, SerdeDataValue::Seq { elems }) => {
                let mut vec = serializer.serialize_seq(Some(elems.len()))?;
                for elem in elems {
                    vec.serialize_element(&BorrowedTypedSerdeData { ty, value: elem })?;
                }
                vec.end()
            }
            (SerdeDataType::Map { key, value }, SerdeDataValue::Map { elems }) => {
                let mut map = serializer.serialize_map(Some(elems.len()))?;
                for (k, v) in elems {
                    map.serialize_entry(
                        &BorrowedTypedSerdeData { ty: key, value: k },
                        &BorrowedTypedSerdeData {
                            ty: value,
                            value: v,
                        },
                    )?;
                }
                map.end()
            }
            (SerdeDataType::UnitStruct { name }, SerdeDataValue::UnitStruct) => {
                serializer.serialize_unit_struct(unsafe { to_static_str(name) })
            }
            (SerdeDataType::Newtype { name, inner }, SerdeDataValue::Newtype { inner: value }) => {
                serializer.serialize_newtype_struct(
                    unsafe { to_static_str(name) },
                    &BorrowedTypedSerdeData { ty: inner, value },
                )
            }
            (
                SerdeDataType::TupleStruct { name, fields },
                SerdeDataValue::Struct { fields: values },
            ) => {
                if values.len() != fields.len() {
                    return Err(serde::ser::Error::custom(
                        "mismatch tuple struct fields len",
                    ));
                }

                let mut tuple = serializer
                    .serialize_tuple_struct(unsafe { to_static_str(name) }, fields.len())?;
                for (ty, data) in fields.iter().zip(values.iter()) {
                    tuple.serialize_field(&BorrowedTypedSerdeData { ty, value: data })?;
                }
                tuple.end()
            }
            (SerdeDataType::Struct { name, fields }, SerdeDataValue::Struct { fields: values }) => {
                if values.len() != fields.0.len() || values.len() != fields.1.len() {
                    return Err(serde::ser::Error::custom("mismatch struct fields len"));
                }

                let mut r#struct =
                    serializer.serialize_struct(unsafe { to_static_str(name) }, values.len())?;
                for ((field, ty), data) in fields.0.iter().zip(fields.1.iter()).zip(values.iter()) {
                    r#struct.serialize_field(
                        unsafe { to_static_str(field) },
                        &BorrowedTypedSerdeData { ty, value: data },
                    )?;
                }
                r#struct.end()
            }
            (
                SerdeDataType::Enum {
                    name,
                    variants,
                    representation,
                },
                SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                },
            ) => {
                let (variant, ty) = match (
                    variants.0.get(*variant_index as usize),
                    variants.1.get(*variant_index as usize),
                ) {
                    (Some(variant), Some(ty)) => (variant, ty),
                    _ => return Err(serde::ser::Error::custom("out of bounds variant index")),
                };

                match (ty, value) {
                    (SerdeDataVariantType::Unit, SerdeDataVariantValue::Unit) => {
                        match representation {
                            SerdeEnumRepresentation::ExternallyTagged => serializer
                                .serialize_unit_variant(
                                    unsafe { to_static_str(name) },
                                    *variant_index,
                                    unsafe { to_static_str(variant) },
                                ),
                            SerdeEnumRepresentation::Untagged => serializer.serialize_unit(),
                            SerdeEnumRepresentation::AdjacentlyTagged { tag, content: _ } => {
                                let mut r#struct = serializer
                                    .serialize_struct(unsafe { to_static_str(name) }, 1)?;
                                r#struct.serialize_field(
                                    unsafe { to_static_str(tag) },
                                    &serde::__private::ser::AdjacentlyTaggedEnumVariant {
                                        enum_name: unsafe { to_static_str(name) },
                                        variant_index: *variant_index,
                                        variant_name: unsafe { to_static_str(variant) },
                                    },
                                )?;
                                r#struct.end()
                            }
                            SerdeEnumRepresentation::InternallyTagged { tag } => {
                                let mut r#struct = serializer
                                    .serialize_struct(unsafe { to_static_str(name) }, 1)?;
                                r#struct.serialize_field(unsafe { to_static_str(tag) }, variant)?;
                                r#struct.end()
                            }
                        }
                    }
                    (
                        SerdeDataVariantType::Newtype { inner: ty },
                        SerdeDataVariantValue::Newtype { inner: value },
                    ) => match representation {
                        SerdeEnumRepresentation::ExternallyTagged => serializer
                            .serialize_newtype_variant(
                                unsafe { to_static_str(name) },
                                *variant_index,
                                unsafe { to_static_str(variant) },
                                &BorrowedTypedSerdeData { ty, value },
                            ),
                        SerdeEnumRepresentation::Untagged => {
                            Serialize::serialize(&BorrowedTypedSerdeData { ty, value }, serializer)
                        }
                        SerdeEnumRepresentation::AdjacentlyTagged { tag, content } => {
                            let mut r#struct =
                                serializer.serialize_struct(unsafe { to_static_str(name) }, 2)?;
                            r#struct.serialize_field(
                                unsafe { to_static_str(tag) },
                                &serde::__private::ser::AdjacentlyTaggedEnumVariant {
                                    enum_name: unsafe { to_static_str(name) },
                                    variant_index: *variant_index,
                                    variant_name: unsafe { to_static_str(variant) },
                                },
                            )?;
                            r#struct.serialize_field(
                                unsafe { to_static_str(content) },
                                &BorrowedTypedSerdeData { ty, value },
                            )?;
                            r#struct.end()
                        }
                        SerdeEnumRepresentation::InternallyTagged { tag } => {
                            if matches!(
                                (&**ty, &**value),
                                (
                                    SerdeDataType::Enum {
                                        name: _,
                                        variants: _,
                                        representation: SerdeEnumRepresentation::Untagged
                                    },
                                    SerdeDataValue::Enum {
                                        variant: _,
                                        value: SerdeDataVariantValue::Unit
                                    },
                                )
                            ) || matches!(
                                (&**ty, &**value),
                                (
                                    SerdeDataType::Enum {
                                        name: _,
                                        variants: _,
                                        representation: SerdeEnumRepresentation::Untagged
                                    },
                                    SerdeDataValue::Enum {
                                        variant: _,
                                        value: SerdeDataVariantValue::Newtype { inner }
                                    },
                                ) if matches!(&**inner, SerdeDataValue::Unit)
                            ) || matches!(
                                (&**ty, &**value),
                                (
                                    SerdeDataType::Enum {
                                        name: _,
                                        variants: _,
                                        representation: SerdeEnumRepresentation::Untagged
                                    },
                                    SerdeDataValue::Enum {
                                        variant: _,
                                        value: SerdeDataVariantValue::Struct { fields }
                                    },
                                ) if fields.is_empty()
                            ) {
                                // BUG: these look like units to ron, which are not allowed in here
                                return Err(serde::ser::Error::custom(
                                    "cannot serialize tagged newtype variant : SERDE BUG",
                                ));
                            }

                            serde::__private::ser::serialize_tagged_newtype(
                                serializer,
                                unsafe { to_static_str(name) },
                                unsafe { to_static_str(variant) },
                                unsafe { to_static_str(tag) },
                                unsafe { to_static_str(variant) },
                                // directly serialising BorrowedTypedSerdeData with
                                //  TaggedSerializer creates a type recursion limit overflow
                                //  since the value type could in theory be infinite and
                                //  one level of TaggedSerializer is added on at every step
                                // erasing the Serialize impl breaks this cycle
                                &Box::new(&BorrowedTypedSerdeData { ty, value }
                                    as &dyn erased_serde::Serialize),
                            )
                        }
                    },
                    (
                        SerdeDataVariantType::Tuple { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        if values.len() != fields.len() {
                            return Err(serde::ser::Error::custom(
                                "mismatch tuple struct variant fields len",
                            ));
                        }

                        struct UntaggedTuple<'a> {
                            fields: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a> Serialize for UntaggedTuple<'a> {
                            fn serialize<S: Serializer>(
                                &self,
                                serializer: S,
                            ) -> Result<S::Ok, S::Error> {
                                let mut tuple = serializer.serialize_tuple(self.fields.len())?;
                                for (ty, data) in self.fields.iter().zip(self.values.iter()) {
                                    tuple.serialize_element(&BorrowedTypedSerdeData {
                                        ty,
                                        value: data,
                                    })?;
                                }
                                tuple.end()
                            }
                        }

                        match representation {
                            SerdeEnumRepresentation::ExternallyTagged => {
                                let mut tuple = serializer.serialize_tuple_variant(
                                    unsafe { to_static_str(name) },
                                    *variant_index,
                                    unsafe { to_static_str(variant) },
                                    fields.len(),
                                )?;
                                for (ty, data) in fields.iter().zip(values.iter()) {
                                    tuple.serialize_field(&BorrowedTypedSerdeData {
                                        ty,
                                        value: data,
                                    })?;
                                }
                                tuple.end()
                            }
                            SerdeEnumRepresentation::Untagged => {
                                UntaggedTuple { fields, values }.serialize(serializer)
                            }
                            SerdeEnumRepresentation::AdjacentlyTagged { tag, content } => {
                                let mut r#struct = serializer
                                    .serialize_struct(unsafe { to_static_str(name) }, 2)?;
                                r#struct.serialize_field(
                                    unsafe { to_static_str(tag) },
                                    &serde::__private::ser::AdjacentlyTaggedEnumVariant {
                                        enum_name: unsafe { to_static_str(name) },
                                        variant_index: *variant_index,
                                        variant_name: unsafe { to_static_str(variant) },
                                    },
                                )?;
                                r#struct.serialize_field(
                                    unsafe { to_static_str(content) },
                                    &UntaggedTuple { fields, values },
                                )?;
                                r#struct.end()
                            }
                            SerdeEnumRepresentation::InternallyTagged { tag: _ } => {
                                Err(serde::ser::Error::custom(
                                    "invalid serde internally tagged tuple variant",
                                ))
                            }
                        }
                    }
                    (
                        SerdeDataVariantType::Struct { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        if values.len() != fields.0.len() || values.len() != fields.1.len() {
                            return Err(serde::ser::Error::custom(
                                "mismatch struct variant fields len",
                            ));
                        }

                        struct UntaggedStruct<'a> {
                            name: &'a str,
                            fields: &'a (Vec<&'a str>, Vec<SerdeDataType<'a>>),
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a> Serialize for UntaggedStruct<'a> {
                            fn serialize<S: Serializer>(
                                &self,
                                serializer: S,
                            ) -> Result<S::Ok, S::Error> {
                                let mut r#struct = serializer.serialize_struct(
                                    unsafe { to_static_str(self.name) },
                                    self.values.len(),
                                )?;
                                for ((field, ty), data) in self
                                    .fields
                                    .0
                                    .iter()
                                    .zip(self.fields.1.iter())
                                    .zip(self.values.iter())
                                {
                                    r#struct.serialize_field(
                                        unsafe { to_static_str(field) },
                                        &BorrowedTypedSerdeData { ty, value: data },
                                    )?;
                                }
                                r#struct.end()
                            }
                        }

                        match representation {
                            SerdeEnumRepresentation::ExternallyTagged => {
                                let mut r#struct = serializer.serialize_struct_variant(
                                    unsafe { to_static_str(name) },
                                    *variant_index,
                                    unsafe { to_static_str(variant) },
                                    values.len(),
                                )?;
                                for ((field, ty), data) in
                                    fields.0.iter().zip(fields.1.iter()).zip(values.iter())
                                {
                                    r#struct.serialize_field(
                                        unsafe { to_static_str(field) },
                                        &BorrowedTypedSerdeData { ty, value: data },
                                    )?;
                                }
                                r#struct.end()
                            }
                            SerdeEnumRepresentation::Untagged => UntaggedStruct {
                                name,
                                fields,
                                values,
                            }
                            .serialize(serializer),
                            SerdeEnumRepresentation::AdjacentlyTagged { tag, content } => {
                                let mut r#struct = serializer
                                    .serialize_struct(unsafe { to_static_str(name) }, 2)?;
                                r#struct.serialize_field(
                                    unsafe { to_static_str(tag) },
                                    &serde::__private::ser::AdjacentlyTaggedEnumVariant {
                                        enum_name: unsafe { to_static_str(name) },
                                        variant_index: *variant_index,
                                        variant_name: unsafe { to_static_str(variant) },
                                    },
                                )?;
                                r#struct.serialize_field(
                                    unsafe { to_static_str(content) },
                                    &UntaggedStruct {
                                        name,
                                        fields,
                                        values,
                                    },
                                )?;
                                r#struct.end()
                            }
                            SerdeEnumRepresentation::InternallyTagged { tag } => {
                                let mut r#struct = serializer.serialize_struct(
                                    unsafe { to_static_str(name) },
                                    values.len() + 1,
                                )?;
                                r#struct.serialize_field(unsafe { to_static_str(tag) }, variant)?;
                                for ((field, ty), data) in
                                    fields.0.iter().zip(fields.1.iter()).zip(values.iter())
                                {
                                    r#struct.serialize_field(
                                        unsafe { to_static_str(field) },
                                        &BorrowedTypedSerdeData { ty, value: data },
                                    )?;
                                }
                                r#struct.end()
                            }
                        }
                    }
                    _ => Err(serde::ser::Error::custom("invalid serde enum data")),
                }
            }
            _ => Err(serde::ser::Error::custom("invalid serde data")),
        }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for BorrowedTypedSerdeData<'a> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        fn deserialize_matching<
            'de,
            T: Deserialize<'de> + fmt::Debug + PartialEq,
            D: Deserializer<'de>,
        >(
            deserializer: D,
            check: &T,
        ) -> Result<(), D::Error> {
            let value = T::deserialize(deserializer)?;

            if value == *check {
                Ok(())
            } else {
                Err(serde::de::Error::custom(format!(
                    "expected {:?} found {:?}",
                    check, value
                )))
            }
        }

        match (self.ty, self.value) {
            (SerdeDataType::Unit, SerdeDataValue::Unit) => deserialize_matching(deserializer, &()),
            (SerdeDataType::Bool, SerdeDataValue::Bool(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::I8, SerdeDataValue::I8(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::I16, SerdeDataValue::I16(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::I32, SerdeDataValue::I32(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::I64, SerdeDataValue::I64(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::I128, SerdeDataValue::I128(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::ISize, SerdeDataValue::ISize(v)) => {
                deserialize_matching(deserializer, v)
            }
            (SerdeDataType::U8, SerdeDataValue::U8(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::U16, SerdeDataValue::U16(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::U32, SerdeDataValue::U32(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::U64, SerdeDataValue::U64(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::U128, SerdeDataValue::U128(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::USize, SerdeDataValue::USize(v)) => {
                deserialize_matching(deserializer, v)
            }
            (SerdeDataType::F32, SerdeDataValue::F32(v)) => {
                let value = f32::deserialize(deserializer)?;

                if (v.is_nan() && value.is_nan()) || (value == *v) {
                    Ok(())
                } else {
                    Err(serde::de::Error::custom(format!(
                        "expected {:?} found {:?}",
                        v, value
                    )))
                }
            }
            (SerdeDataType::F64, SerdeDataValue::F64(v)) => {
                let value = f64::deserialize(deserializer)?;

                if (v.is_nan() && value.is_nan()) || (value == *v) {
                    Ok(())
                } else {
                    Err(serde::de::Error::custom(format!(
                        "expected {:?} found {:?}",
                        v, value
                    )))
                }
            }
            (SerdeDataType::Char, SerdeDataValue::Char(v)) => deserialize_matching(deserializer, v),
            (SerdeDataType::String, SerdeDataValue::String(v)) => {
                struct StringVisitor<'a> {
                    value: &'a str,
                }

                impl<'a, 'de> Visitor<'de> for StringVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a string")
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        if v == self.value {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected {:?} found {:?}",
                                self.value, v
                            )))
                        }
                    }
                }

                deserializer.deserialize_str(StringVisitor { value: v })
            }
            (SerdeDataType::ByteBuf, SerdeDataValue::ByteBuf(v)) => {
                struct BytesVisitor<'a> {
                    value: &'a [u8],
                }

                impl<'a, 'de> Visitor<'de> for BytesVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a byte array")
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        self.visit_bytes(v.as_bytes())
                    }

                    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                        if v == self.value {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected {:?} found {:?}",
                                self.value, v
                            )))
                        }
                    }
                }

                deserializer.deserialize_bytes(BytesVisitor { value: v })
            }
            (SerdeDataType::Option { inner: ty }, SerdeDataValue::Option { inner: value }) => {
                struct OptionVisitor<'a> {
                    ty: &'a SerdeDataType<'a>,
                    value: Option<&'a SerdeDataValue<'a>>,
                }

                impl<'a, 'de> Visitor<'de> for OptionVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("an option")
                    }

                    fn visit_some<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error> {
                        if let Some(expected) = self.value {
                            BorrowedTypedSerdeData {
                                ty: self.ty,
                                value: expected,
                            }
                            .deserialize(deserializer)
                        } else {
                            Err(serde::de::Error::custom("expected None found Some(...)"))
                        }
                    }

                    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                        if self.value.is_none() {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected {:?} found None",
                                self.value
                            )))
                        }
                    }
                }

                deserializer.deserialize_option(OptionVisitor {
                    ty,
                    value: value.as_deref(),
                })
            }
            (SerdeDataType::Array { kind, len }, SerdeDataValue::Seq { elems }) => {
                struct ArrayVisitor<'a> {
                    kind: &'a SerdeDataType<'a>,
                    elems: &'a [SerdeDataValue<'a>],
                }

                impl<'a, 'de> Visitor<'de> for ArrayVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_fmt(format_args!("an array of length {}", self.elems.len()))
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        for expected in self.elems {
                            seq.next_element_seed(BorrowedTypedSerdeData {
                                ty: self.kind,
                                value: expected,
                            })?;
                        }
                        Ok(())
                    }
                }

                if elems.len() != *len {
                    return Err(serde::de::Error::custom("mismatch array len"));
                }

                deserializer.deserialize_tuple(*len, ArrayVisitor { kind, elems })
            }
            (SerdeDataType::Tuple { elems: tys }, SerdeDataValue::Seq { elems: values }) => {
                struct TupleVisitor<'a> {
                    tys: &'a [SerdeDataType<'a>],
                    values: &'a [SerdeDataValue<'a>],
                }

                impl<'a, 'de> Visitor<'de> for TupleVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_fmt(format_args!("a tuple of size {}", self.values.len()))
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        for (ty, expected) in self.tys.iter().zip(self.values.iter()) {
                            seq.next_element_seed(BorrowedTypedSerdeData {
                                ty,
                                value: expected,
                            })?;
                        }
                        Ok(())
                    }
                }

                struct NewtypeVisitor<'a> {
                    inner: &'a SerdeDataType<'a>,
                    value: &'a SerdeDataValue<'a>,
                }

                impl<'a, 'de> Visitor<'de> for NewtypeVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a newtype tuple")
                    }

                    fn visit_newtype_struct<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error> {
                        BorrowedTypedSerdeData {
                            ty: self.inner,
                            value: self.value,
                        }
                        .deserialize(deserializer)
                    }
                }

                if values.len() != tys.len() {
                    return Err(serde::de::Error::custom("mismatch tuple len"));
                }

                deserializer.deserialize_tuple(tys.len(), TupleVisitor { tys, values })
            }
            (SerdeDataType::Vec { item }, SerdeDataValue::Seq { elems }) => {
                struct VecVisitor<'a> {
                    item: &'a SerdeDataType<'a>,
                    elems: &'a [SerdeDataValue<'a>],
                }

                impl<'a, 'de> Visitor<'de> for VecVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a sequence")
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        for expected in self.elems {
                            seq.next_element_seed(BorrowedTypedSerdeData {
                                ty: self.item,
                                value: expected,
                            })?;
                        }
                        Ok(())
                    }
                }

                deserializer.deserialize_seq(VecVisitor { item, elems })
            }
            (SerdeDataType::Map { key, value }, SerdeDataValue::Map { elems }) => {
                struct MapVisitor<'a> {
                    key: &'a SerdeDataType<'a>,
                    value: &'a SerdeDataType<'a>,
                    elems: &'a [(SerdeDataValue<'a>, SerdeDataValue<'a>)],
                }

                impl<'a, 'de> Visitor<'de> for MapVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a map")
                    }

                    fn visit_map<A: MapAccess<'de>>(
                        self,
                        mut map: A,
                    ) -> Result<Self::Value, A::Error> {
                        for (ekey, eval) in self.elems {
                            map.next_entry_seed(
                                BorrowedTypedSerdeData {
                                    ty: self.key,
                                    value: ekey,
                                },
                                BorrowedTypedSerdeData {
                                    ty: self.value,
                                    value: eval,
                                },
                            )?;
                        }
                        Ok(())
                    }
                }

                deserializer.deserialize_map(MapVisitor { key, value, elems })
            }
            (SerdeDataType::UnitStruct { name }, SerdeDataValue::UnitStruct) => {
                struct UnitStructVisitor<'a> {
                    name: &'a str,
                }

                impl<'a, 'de> Visitor<'de> for UnitStructVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_fmt(format_args!("the unit struct {}", self.name))
                    }

                    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                        Ok(())
                    }
                }

                deserializer.deserialize_unit_struct(
                    unsafe { to_static_str(name) },
                    UnitStructVisitor { name },
                )
            }
            (SerdeDataType::Newtype { name, inner }, SerdeDataValue::Newtype { inner: value }) => {
                struct NewtypeVisitor<'a> {
                    name: &'a str,
                    inner: &'a SerdeDataType<'a>,
                    value: &'a SerdeDataValue<'a>,
                }

                impl<'a, 'de> Visitor<'de> for NewtypeVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_fmt(format_args!("the newtype struct {}", self.name))
                    }

                    fn visit_newtype_struct<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error> {
                        BorrowedTypedSerdeData {
                            ty: self.inner,
                            value: self.value,
                        }
                        .deserialize(deserializer)
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        // ron::value::RawValue expects a visit_str call
                        //  even though it's disguised as a newtype
                        if self.name == RAW_VALUE_TOKEN {
                            if let SerdeDataValue::String(ron) = &self.value {
                                // pretty serialising can add whitespace and comments
                                //  before and after the raw value
                                if v.contains(ron) {
                                    return Ok(());
                                }
                            }
                        }

                        // Fall back to the default implementation of visit_str
                        Err(serde::de::Error::invalid_type(
                            serde::de::Unexpected::Str(v),
                            &self,
                        ))
                    }
                }

                deserializer.deserialize_newtype_struct(
                    unsafe { to_static_str(name) },
                    NewtypeVisitor { name, inner, value },
                )
            }
            (
                SerdeDataType::TupleStruct { name, fields },
                SerdeDataValue::Struct { fields: values },
            ) => {
                struct TupleStructVisitor<'a> {
                    name: &'a str,
                    fields: &'a [SerdeDataType<'a>],
                    values: &'a [SerdeDataValue<'a>],
                }

                impl<'a, 'de> Visitor<'de> for TupleStructVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_fmt(format_args!("the tuple struct {}", self.name))
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        for (ty, expected) in self.fields.iter().zip(self.values.iter()) {
                            seq.next_element_seed(BorrowedTypedSerdeData {
                                ty,
                                value: expected,
                            })?;
                        }
                        Ok(())
                    }
                }

                if values.len() != fields.len() {
                    return Err(serde::de::Error::custom("mismatch tuple struct fields len"));
                }

                deserializer.deserialize_tuple_struct(
                    unsafe { to_static_str(name) },
                    fields.len(),
                    TupleStructVisitor {
                        name,
                        fields,
                        values,
                    },
                )
            }
            (SerdeDataType::Struct { name, fields }, SerdeDataValue::Struct { fields: values }) => {
                struct FieldIdentifierVisitor<'a> {
                    field: &'a str,
                    index: u64,
                }

                impl<'a, 'de> Visitor<'de> for FieldIdentifierVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a field identifier")
                    }

                    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                        if v == self.index {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected field index {} found {}",
                                self.index, v
                            )))
                        }
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        if v == self.field {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected field identifier {} found {}",
                                self.field, v
                            )))
                        }
                    }

                    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                        if v == self.field.as_bytes() {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected field identifier {:?} found {:?}",
                                self.field.as_bytes(),
                                v
                            )))
                        }
                    }
                }

                impl<'a, 'de> DeserializeSeed<'de> for FieldIdentifierVisitor<'a> {
                    type Value = ();

                    fn deserialize<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error> {
                        deserializer.deserialize_identifier(self)
                    }
                }

                struct StructVisitor<'a> {
                    name: &'a str,
                    fields: &'a [&'a str],
                    tys: &'a [SerdeDataType<'a>],
                    values: &'a [SerdeDataValue<'a>],
                }

                impl<'a, 'de> Visitor<'de> for StructVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_fmt(format_args!("the struct {}", self.name))
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        for (ty, expected) in self.tys.iter().zip(self.values.iter()) {
                            seq.next_element_seed(BorrowedTypedSerdeData {
                                ty,
                                value: expected,
                            })?;
                        }
                        Ok(())
                    }

                    fn visit_map<A: MapAccess<'de>>(
                        self,
                        mut map: A,
                    ) -> Result<Self::Value, A::Error> {
                        for (((index, field), ty), expected) in (0..)
                            .zip(self.fields.iter())
                            .zip(self.tys.iter())
                            .zip(self.values.iter())
                        {
                            map.next_entry_seed(
                                FieldIdentifierVisitor { field, index },
                                BorrowedTypedSerdeData {
                                    ty,
                                    value: expected,
                                },
                            )?;
                        }
                        Ok(())
                    }
                }

                if values.len() != fields.0.len() || values.len() != fields.1.len() {
                    return Err(serde::de::Error::custom("mismatch struct fields len"));
                }

                deserializer.deserialize_struct(
                    unsafe { to_static_str(name) },
                    unsafe { to_static_str_slice(&fields.0) },
                    StructVisitor {
                        name,
                        fields: &fields.0,
                        tys: &fields.1,
                        values,
                    },
                )
            }
            (
                SerdeDataType::Enum {
                    name,
                    variants,
                    representation: SerdeEnumRepresentation::ExternallyTagged,
                },
                SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                },
            ) => {
                struct VariantIdentifierVisitor<'a> {
                    variant: &'a str,
                    index: u32,
                }

                impl<'a, 'de> Visitor<'de> for VariantIdentifierVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a variant identifier")
                    }

                    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                        if v == u64::from(self.index) {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected variant index {} found {}",
                                self.index, v
                            )))
                        }
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        if v == self.variant {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected variant identifier {} found {}",
                                self.variant, v
                            )))
                        }
                    }

                    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                        if v == self.variant.as_bytes() {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected variant identifier {:?} found {:?}",
                                self.variant.as_bytes(),
                                v
                            )))
                        }
                    }
                }

                impl<'a, 'de> DeserializeSeed<'de> for VariantIdentifierVisitor<'a> {
                    type Value = ();

                    fn deserialize<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error> {
                        deserializer.deserialize_identifier(self)
                    }
                }

                struct EnumVisitor<'a> {
                    name: &'a str,
                    variant: &'a str,
                    index: u32,
                    ty: &'a SerdeDataVariantType<'a>,
                    value: &'a SerdeDataVariantValue<'a>,
                }

                impl<'a, 'de> Visitor<'de> for EnumVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_fmt(format_args!(
                            "the variant {} of the enum {}",
                            self.variant, self.name
                        ))
                    }

                    fn visit_enum<A: EnumAccess<'de>>(
                        self,
                        data: A,
                    ) -> Result<Self::Value, A::Error> {
                        let ((), variant) = data.variant_seed(VariantIdentifierVisitor {
                            variant: self.variant,
                            index: self.index,
                        })?;

                        match (self.ty, self.value) {
                            (SerdeDataVariantType::Unit, SerdeDataVariantValue::Unit) => {
                                variant.unit_variant()
                            }
                            (
                                SerdeDataVariantType::Newtype { inner: ty },
                                SerdeDataVariantValue::Newtype { inner: value },
                            ) => variant.newtype_variant_seed(BorrowedTypedSerdeData { ty, value }),
                            (
                                SerdeDataVariantType::Tuple { fields },
                                SerdeDataVariantValue::Struct { fields: values },
                            ) => {
                                struct TupleVariantVisitor<'a> {
                                    variant: &'a str,
                                    fields: &'a [SerdeDataType<'a>],
                                    values: &'a [SerdeDataValue<'a>],
                                }

                                impl<'a, 'de> Visitor<'de> for TupleVariantVisitor<'a> {
                                    type Value = ();

                                    fn expecting(
                                        &self,
                                        formatter: &mut fmt::Formatter,
                                    ) -> fmt::Result {
                                        formatter.write_fmt(format_args!(
                                            "the tuple variant {}",
                                            self.variant
                                        ))
                                    }

                                    fn visit_seq<A: SeqAccess<'de>>(
                                        self,
                                        mut seq: A,
                                    ) -> Result<Self::Value, A::Error>
                                    {
                                        for (ty, expected) in
                                            self.fields.iter().zip(self.values.iter())
                                        {
                                            seq.next_element_seed(BorrowedTypedSerdeData {
                                                ty,
                                                value: expected,
                                            })?;
                                        }
                                        Ok(())
                                    }
                                }

                                if values.len() != fields.len() {
                                    return Err(serde::de::Error::custom(
                                        "mismatch tuple struct variant fields len",
                                    ));
                                }

                                variant.tuple_variant(
                                    fields.len(),
                                    TupleVariantVisitor {
                                        variant: self.variant,
                                        fields,
                                        values,
                                    },
                                )
                            }
                            (
                                SerdeDataVariantType::Struct { fields },
                                SerdeDataVariantValue::Struct { fields: values },
                            ) => {
                                struct FieldIdentifierVisitor<'a> {
                                    field: &'a str,
                                    index: u64,
                                }

                                impl<'a, 'de> Visitor<'de> for FieldIdentifierVisitor<'a> {
                                    type Value = ();

                                    fn expecting(
                                        &self,
                                        formatter: &mut fmt::Formatter,
                                    ) -> fmt::Result {
                                        formatter.write_str("a field identifier")
                                    }

                                    fn visit_u64<E: serde::de::Error>(
                                        self,
                                        v: u64,
                                    ) -> Result<Self::Value, E>
                                    {
                                        if v == self.index {
                                            Ok(())
                                        } else {
                                            Err(serde::de::Error::custom(format!(
                                                "expected field index {} found {}",
                                                self.index, v
                                            )))
                                        }
                                    }

                                    fn visit_str<E: serde::de::Error>(
                                        self,
                                        v: &str,
                                    ) -> Result<Self::Value, E>
                                    {
                                        if v == self.field {
                                            Ok(())
                                        } else {
                                            Err(serde::de::Error::custom(format!(
                                                "expected field identifier {} found {}",
                                                self.field, v
                                            )))
                                        }
                                    }

                                    fn visit_bytes<E: serde::de::Error>(
                                        self,
                                        v: &[u8],
                                    ) -> Result<Self::Value, E>
                                    {
                                        if v == self.field.as_bytes() {
                                            Ok(())
                                        } else {
                                            Err(serde::de::Error::custom(format!(
                                                "expected field identifier {:?} found {:?}",
                                                self.field.as_bytes(),
                                                v
                                            )))
                                        }
                                    }
                                }

                                impl<'a, 'de> DeserializeSeed<'de> for FieldIdentifierVisitor<'a> {
                                    type Value = ();

                                    fn deserialize<D: Deserializer<'de>>(
                                        self,
                                        deserializer: D,
                                    ) -> Result<Self::Value, D::Error>
                                    {
                                        deserializer.deserialize_identifier(self)
                                    }
                                }

                                struct StructVariantVisitor<'a> {
                                    variant: &'a str,
                                    fields: &'a [&'a str],
                                    tys: &'a [SerdeDataType<'a>],
                                    values: &'a [SerdeDataValue<'a>],
                                }

                                impl<'a, 'de> Visitor<'de> for StructVariantVisitor<'a> {
                                    type Value = ();

                                    fn expecting(
                                        &self,
                                        formatter: &mut fmt::Formatter,
                                    ) -> fmt::Result {
                                        formatter.write_fmt(format_args!(
                                            "the struct variant {}",
                                            self.variant
                                        ))
                                    }

                                    fn visit_seq<A: SeqAccess<'de>>(
                                        self,
                                        mut seq: A,
                                    ) -> Result<Self::Value, A::Error>
                                    {
                                        for (ty, expected) in
                                            self.tys.iter().zip(self.values.iter())
                                        {
                                            seq.next_element_seed(BorrowedTypedSerdeData {
                                                ty,
                                                value: expected,
                                            })?;
                                        }
                                        Ok(())
                                    }

                                    fn visit_map<A: MapAccess<'de>>(
                                        self,
                                        mut map: A,
                                    ) -> Result<Self::Value, A::Error>
                                    {
                                        for (((index, field), ty), expected) in (0..)
                                            .zip(self.fields.iter())
                                            .zip(self.tys.iter())
                                            .zip(self.values.iter())
                                        {
                                            map.next_entry_seed(
                                                FieldIdentifierVisitor { field, index },
                                                BorrowedTypedSerdeData {
                                                    ty,
                                                    value: expected,
                                                },
                                            )?;
                                        }
                                        Ok(())
                                    }
                                }

                                if values.len() != fields.0.len() || values.len() != fields.1.len()
                                {
                                    return Err(serde::de::Error::custom(
                                        "mismatch struct fields len",
                                    ));
                                }

                                variant.struct_variant(
                                    unsafe { to_static_str_slice(&fields.0) },
                                    StructVariantVisitor {
                                        variant: self.variant,
                                        fields: &fields.0,
                                        tys: &fields.1,
                                        values,
                                    },
                                )
                            }
                            _ => Err(serde::de::Error::custom("invalid serde enum data")),
                        }
                    }
                }

                let (variant, ty) = match (
                    variants.0.get(*variant_index as usize),
                    variants.1.get(*variant_index as usize),
                ) {
                    (Some(variant), Some(ty)) => (variant, ty),
                    _ => return Err(serde::de::Error::custom("out of bounds variant index")),
                };

                deserializer.deserialize_enum(
                    unsafe { to_static_str(name) },
                    unsafe { to_static_str_slice(&variants.0) },
                    EnumVisitor {
                        name,
                        variant,
                        index: *variant_index,
                        ty,
                        value,
                    },
                )
            }
            (
                SerdeDataType::Enum {
                    name,
                    variants,
                    representation: SerdeEnumRepresentation::Untagged,
                },
                SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                },
            ) => {
                let content = serde::__private::de::Content::deserialize(deserializer)?;
                let deserializer =
                    serde::__private::de::ContentRefDeserializer::<D::Error>::new(&content);

                let (variant, ty) = match (
                    variants.0.get(*variant_index as usize),
                    variants.1.get(*variant_index as usize),
                ) {
                    (Some(variant), Some(ty)) => (variant, ty),
                    _ => return Err(serde::de::Error::custom("out of bounds variant index")),
                };

                match (ty, value) {
                    (SerdeDataVariantType::Unit, SerdeDataVariantValue::Unit) => deserializer
                        .deserialize_any(serde::__private::de::UntaggedUnitVisitor::new(
                            name, variant,
                        )),
                    (
                        SerdeDataVariantType::Newtype { inner: ty },
                        SerdeDataVariantValue::Newtype { inner: value },
                    ) => BorrowedTypedSerdeData { ty, value }.deserialize(deserializer),
                    (
                        SerdeDataVariantType::Tuple { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        struct TupleVariantVisitor<'a> {
                            variant: &'a str,
                            fields: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for TupleVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the tuple variant {}", self.variant))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (ty, expected) in self.fields.iter().zip(self.values.iter()) {
                                    seq.next_element_seed(BorrowedTypedSerdeData {
                                        ty,
                                        value: expected,
                                    })?;
                                }
                                Ok(())
                            }
                        }

                        if values.len() != fields.len() {
                            return Err(serde::de::Error::custom(
                                "mismatch tuple struct variant fields len",
                            ));
                        }

                        deserializer.deserialize_tuple(
                            fields.len(),
                            TupleVariantVisitor {
                                variant,
                                fields,
                                values,
                            },
                        )
                    }
                    (
                        SerdeDataVariantType::Struct { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        struct FieldIdentifierVisitor<'a> {
                            field: &'a str,
                            index: u64,
                        }

                        impl<'a, 'de> Visitor<'de> for FieldIdentifierVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("a field identifier")
                            }

                            fn visit_u64<E: serde::de::Error>(
                                self,
                                v: u64,
                            ) -> Result<Self::Value, E> {
                                if v == self.index {
                                    Ok(())
                                } else {
                                    Err(serde::de::Error::custom(format!(
                                        "expected field index {} found {}",
                                        self.index, v
                                    )))
                                }
                            }

                            fn visit_str<E: serde::de::Error>(
                                self,
                                v: &str,
                            ) -> Result<Self::Value, E> {
                                if v == self.field {
                                    Ok(())
                                } else {
                                    Err(serde::de::Error::custom(format!(
                                        "expected field identifier {} found {}",
                                        self.field, v
                                    )))
                                }
                            }

                            fn visit_bytes<E: serde::de::Error>(
                                self,
                                v: &[u8],
                            ) -> Result<Self::Value, E> {
                                if v == self.field.as_bytes() {
                                    Ok(())
                                } else {
                                    Err(serde::de::Error::custom(format!(
                                        "expected field identifier {:?} found {:?}",
                                        self.field.as_bytes(),
                                        v
                                    )))
                                }
                            }
                        }

                        impl<'a, 'de> DeserializeSeed<'de> for FieldIdentifierVisitor<'a> {
                            type Value = ();

                            fn deserialize<D: Deserializer<'de>>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error> {
                                deserializer.deserialize_identifier(self)
                            }
                        }

                        struct StructVariantVisitor<'a> {
                            variant: &'a str,
                            fields: &'a [&'a str],
                            tys: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for StructVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the struct variant {}", self.variant))
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (((index, field), ty), expected) in (0..)
                                    .zip(self.fields.iter())
                                    .zip(self.tys.iter())
                                    .zip(self.values.iter())
                                {
                                    map.next_entry_seed(
                                        FieldIdentifierVisitor { field, index },
                                        BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        },
                                    )?;
                                }
                                Ok(())
                            }
                        }

                        if values.len() != fields.0.len() || values.len() != fields.1.len() {
                            return Err(serde::de::Error::custom("mismatch struct fields len"));
                        }

                        deserializer.deserialize_struct(
                            unsafe { to_static_str(name) },
                            unsafe { to_static_str_slice(&fields.0) },
                            StructVariantVisitor {
                                variant,
                                fields: &fields.0,
                                tys: &fields.1,
                                values,
                            },
                        )
                    }
                    _ => Err(serde::de::Error::custom("invalid serde enum data")),
                }
            }
            (
                SerdeDataType::Enum {
                    name,
                    variants,
                    representation: SerdeEnumRepresentation::AdjacentlyTagged { tag, content },
                },
                SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                },
            ) => {
                struct FieldIdentifierVisitor<'a> {
                    field: &'a str,
                    index: u64,
                }

                impl<'a, 'de> Visitor<'de> for FieldIdentifierVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a field identifier")
                    }

                    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                        if v == self.index {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected field index {} found {}",
                                self.index, v
                            )))
                        }
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        if v == self.field {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected field identifier {} found {}",
                                self.field, v
                            )))
                        }
                    }

                    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                        if v == self.field.as_bytes() {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected field identifier {:?} found {:?}",
                                self.field.as_bytes(),
                                v
                            )))
                        }
                    }
                }

                impl<'a, 'de> DeserializeSeed<'de> for FieldIdentifierVisitor<'a> {
                    type Value = ();

                    fn deserialize<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error> {
                        deserializer.deserialize_identifier(self)
                    }
                }

                let (variant, ty) = match (
                    variants.0.get(*variant_index as usize),
                    variants.1.get(*variant_index as usize),
                ) {
                    (Some(variant), Some(ty)) => (variant, ty),
                    _ => return Err(serde::de::Error::custom("out of bounds variant index")),
                };

                enum DelayedVariantIdentifier {
                    Index(u64),
                    Str(String),
                    Bytes(Vec<u8>),
                }

                impl DelayedVariantIdentifier {
                    fn check_variant<E: serde::de::Error>(
                        self,
                        variant: &str,
                        variant_index: u32,
                    ) -> Result<(), E> {
                        match self {
                            DelayedVariantIdentifier::Index(v) if v == u64::from(variant_index) => {
                                Ok(())
                            }
                            DelayedVariantIdentifier::Index(v) => Err(serde::de::Error::custom(
                                format!("expected variant index {} found {}", variant_index, v),
                            )),
                            DelayedVariantIdentifier::Str(ref v) if v == variant => Ok(()),
                            DelayedVariantIdentifier::Str(ref v) => Err(serde::de::Error::custom(
                                format!("expected variant identifier {} found {}", variant, v),
                            )),
                            DelayedVariantIdentifier::Bytes(ref v) if v == variant.as_bytes() => {
                                Ok(())
                            }
                            DelayedVariantIdentifier::Bytes(ref v) => {
                                Err(serde::de::Error::custom(format!(
                                    "expected variant identifier {:?} found {:?}",
                                    variant.as_bytes(),
                                    v
                                )))
                            }
                        }
                    }
                }

                impl<'de> Deserialize<'de> for DelayedVariantIdentifier {
                    fn deserialize<D: Deserializer<'de>>(
                        deserializer: D,
                    ) -> Result<Self, D::Error> {
                        struct DelayedVariantIdentifierVisitor;

                        impl<'de> Visitor<'de> for DelayedVariantIdentifierVisitor {
                            type Value = DelayedVariantIdentifier;

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("a variant identifier")
                            }

                            fn visit_u64<E: serde::de::Error>(
                                self,
                                v: u64,
                            ) -> Result<Self::Value, E> {
                                Ok(DelayedVariantIdentifier::Index(v))
                            }

                            fn visit_str<E: serde::de::Error>(
                                self,
                                v: &str,
                            ) -> Result<Self::Value, E> {
                                Ok(DelayedVariantIdentifier::Str(String::from(v)))
                            }

                            fn visit_bytes<E: serde::de::Error>(
                                self,
                                v: &[u8],
                            ) -> Result<Self::Value, E> {
                                Ok(DelayedVariantIdentifier::Bytes(Vec::from(v)))
                            }
                        }

                        deserializer.deserialize_identifier(DelayedVariantIdentifierVisitor)
                    }
                }

                match (ty, value) {
                    (SerdeDataVariantType::Unit, SerdeDataVariantValue::Unit) => {
                        struct AdjacentlyTaggedUnitVariantVisitor<'a> {
                            tag: &'a str,
                            variant: &'a str,
                            variant_index: u32,
                            content: &'a str,
                        }

                        impl<'a, 'de> Visitor<'de> for AdjacentlyTaggedUnitVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the unit variant {}", self.variant))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                if let Some(tag) = seq.next_element::<DelayedVariantIdentifier>()? {
                                    tag.check_variant(self.variant, self.variant_index)
                                } else {
                                    Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }))
                                }
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                let Some(serde::__private::de::TagOrContentField::Tag) = map.next_key_seed(serde::__private::de::TagOrContentFieldVisitor {
                                    tag: unsafe { to_static_str(self.tag) },
                                    content: unsafe { to_static_str(self.content) },
                                })? else {
                                    return Err(serde::de::Error::missing_field(unsafe { to_static_str(self.tag) }))
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.variant, self.variant_index)
                            }
                        }

                        deserializer.deserialize_struct(
                            unsafe { to_static_str(name) },
                            unsafe { to_static_str_slice(std::slice::from_ref(tag)) },
                            AdjacentlyTaggedUnitVariantVisitor {
                                tag,
                                variant,
                                variant_index: *variant_index,
                                content,
                            },
                        )
                    }
                    (
                        SerdeDataVariantType::Newtype { inner: ty },
                        SerdeDataVariantValue::Newtype { inner: value },
                    ) => {
                        struct AdjacentlyTaggedNewtypeVariantVisitor<'a> {
                            tag: &'a str,
                            variant: &'a str,
                            variant_index: u32,
                            content: &'a str,
                            ty: &'a SerdeDataType<'a>,
                            value: &'a SerdeDataValue<'a>,
                        }

                        impl<'a, 'de> Visitor<'de> for AdjacentlyTaggedNewtypeVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the newtype variant {}", self.variant))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                if let Some(tag) = seq.next_element::<DelayedVariantIdentifier>()? {
                                    tag.check_variant(self.variant, self.variant_index)?;
                                } else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                }
                                let Some(()) = seq.next_element_seed(BorrowedTypedSerdeData {
                                    ty: self.ty,
                                    value: self.value,
                                })? else {
                                    return Err(serde::de::Error::missing_field(unsafe { to_static_str(self.content) }))
                                };
                                Ok(())
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                let Some(serde::__private::de::TagOrContentField::Tag) = map.next_key_seed(serde::__private::de::TagOrContentFieldVisitor {
                                    tag: unsafe { to_static_str(self.tag) },
                                    content: unsafe { to_static_str(self.content) },
                                })? else {
                                    return Err(serde::de::Error::missing_field(unsafe { to_static_str(self.tag) }))
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.variant, self.variant_index)?;
                                let Some(serde::__private::de::TagOrContentField::Content) = map.next_key_seed(serde::__private::de::TagOrContentFieldVisitor {
                                    tag: unsafe { to_static_str(self.tag) },
                                    content: unsafe { to_static_str(self.content) },
                                })? else {
                                    return Err(serde::de::Error::missing_field(unsafe { to_static_str(self.tag) }))
                                };
                                map.next_value_seed(BorrowedTypedSerdeData {
                                    ty: self.ty,
                                    value: self.value,
                                })
                            }
                        }

                        deserializer.deserialize_struct(
                            unsafe { to_static_str(name) },
                            unsafe { to_static_str_slice(&[tag, content]) },
                            AdjacentlyTaggedNewtypeVariantVisitor {
                                tag,
                                variant,
                                variant_index: *variant_index,
                                content,
                                ty,
                                value,
                            },
                        )
                    }
                    (
                        SerdeDataVariantType::Tuple { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        struct TupleVariantSeed<'a> {
                            variant: &'a str,
                            fields: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for TupleVariantSeed<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the tuple variant {}", self.variant))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (ty, expected) in self.fields.iter().zip(self.values.iter()) {
                                    seq.next_element_seed(BorrowedTypedSerdeData {
                                        ty,
                                        value: expected,
                                    })?;
                                }
                                Ok(())
                            }
                        }

                        impl<'a, 'de> DeserializeSeed<'de> for TupleVariantSeed<'a> {
                            type Value = ();

                            fn deserialize<D: Deserializer<'de>>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error> {
                                deserializer.deserialize_tuple(self.fields.len(), self)
                            }
                        }

                        struct AdjacentlyTaggedTupleVariantVisitor<'a> {
                            tag: &'a str,
                            variant: &'a str,
                            variant_index: u32,
                            content: &'a str,
                            fields: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for AdjacentlyTaggedTupleVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the tuple variant {}", self.variant))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                if let Some(tag) = seq.next_element::<DelayedVariantIdentifier>()? {
                                    tag.check_variant(self.variant, self.variant_index)?;
                                } else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                }
                                seq.next_element_seed(TupleVariantSeed {
                                    variant: self.variant,
                                    fields: self.fields,
                                    values: self.values,
                                })?;
                                Ok(())
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                let Some(serde::__private::de::TagOrContentField::Tag) = map.next_key_seed(serde::__private::de::TagOrContentFieldVisitor {
                                    tag: unsafe { to_static_str(self.tag) },
                                    content: unsafe { to_static_str(self.content) },
                                })? else {
                                    return Err(serde::de::Error::missing_field(unsafe { to_static_str(self.tag) }))
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.variant, self.variant_index)?;
                                let Some(serde::__private::de::TagOrContentField::Content) = map.next_key_seed(serde::__private::de::TagOrContentFieldVisitor {
                                    tag: unsafe { to_static_str(self.tag) },
                                    content: unsafe { to_static_str(self.content) },
                                })? else {
                                    return Err(serde::de::Error::missing_field(unsafe { to_static_str(self.tag) }))
                                };
                                map.next_value_seed(TupleVariantSeed {
                                    variant: self.variant,
                                    fields: self.fields,
                                    values: self.values,
                                })
                            }
                        }

                        if values.len() != fields.len() {
                            return Err(serde::de::Error::custom(
                                "mismatch tuple struct variant fields len",
                            ));
                        }

                        deserializer.deserialize_struct(
                            unsafe { to_static_str(name) },
                            unsafe { to_static_str_slice(&[tag, content]) },
                            AdjacentlyTaggedTupleVariantVisitor {
                                tag,
                                variant,
                                variant_index: *variant_index,
                                content,
                                fields,
                                values,
                            },
                        )
                    }
                    (
                        SerdeDataVariantType::Struct { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        struct StructVariantSeed<'a> {
                            name: &'a str,
                            variant: &'a str,
                            fields: &'a [&'a str],
                            tys: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for StructVariantSeed<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the struct variant {}", self.variant))
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (((index, field), ty), expected) in (0..)
                                    .zip(self.fields.iter())
                                    .zip(self.tys.iter())
                                    .zip(self.values.iter())
                                {
                                    map.next_entry_seed(
                                        FieldIdentifierVisitor { field, index },
                                        BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        },
                                    )?;
                                }
                                Ok(())
                            }
                        }

                        impl<'a, 'de> DeserializeSeed<'de> for StructVariantSeed<'a> {
                            type Value = ();

                            fn deserialize<D: Deserializer<'de>>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error> {
                                deserializer.deserialize_struct(
                                    unsafe { to_static_str(self.name) },
                                    unsafe { to_static_str_slice(self.fields) },
                                    self,
                                )
                            }
                        }

                        struct AdjacentlyTaggedStructVariantVisitor<'a> {
                            name: &'a str,
                            tag: &'a str,
                            variant: &'a str,
                            variant_index: u32,
                            content: &'a str,
                            fields: &'a [&'a str],
                            tys: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for AdjacentlyTaggedStructVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the struct variant {}", self.variant))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                if let Some(tag) = seq.next_element::<DelayedVariantIdentifier>()? {
                                    tag.check_variant(self.variant, self.variant_index)?;
                                } else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                }
                                seq.next_element_seed(StructVariantSeed {
                                    name: self.name,
                                    variant: self.variant,
                                    fields: self.fields,
                                    tys: self.tys,
                                    values: self.values,
                                })?;
                                Ok(())
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                let Some(serde::__private::de::TagOrContentField::Tag) = map.next_key_seed(serde::__private::de::TagOrContentFieldVisitor {
                                    tag: unsafe { to_static_str(self.tag) },
                                    content: unsafe { to_static_str(self.content) },
                                })? else {
                                    return Err(serde::de::Error::missing_field(unsafe { to_static_str(self.tag) }))
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.variant, self.variant_index)?;
                                let Some(serde::__private::de::TagOrContentField::Content) = map.next_key_seed(serde::__private::de::TagOrContentFieldVisitor {
                                    tag: unsafe { to_static_str(self.tag) },
                                    content: unsafe { to_static_str(self.content) },
                                })? else {
                                    return Err(serde::de::Error::missing_field(unsafe { to_static_str(self.tag) }))
                                };
                                map.next_value_seed(StructVariantSeed {
                                    name: self.name,
                                    variant: self.variant,
                                    fields: self.fields,
                                    tys: self.tys,
                                    values: self.values,
                                })
                            }
                        }

                        if values.len() != fields.0.len() || values.len() != fields.1.len() {
                            return Err(serde::de::Error::custom("mismatch struct fields len"));
                        }

                        deserializer.deserialize_struct(
                            unsafe { to_static_str(name) },
                            unsafe { to_static_str_slice(&[tag, content]) },
                            AdjacentlyTaggedStructVariantVisitor {
                                name,
                                tag,
                                variant,
                                variant_index: *variant_index,
                                content,
                                fields: &fields.0,
                                tys: &fields.1,
                                values,
                            },
                        )
                    }
                    _ => Err(serde::de::Error::custom("invalid serde enum data")),
                }
            }
            (
                SerdeDataType::Enum {
                    name,
                    variants,
                    representation: SerdeEnumRepresentation::InternallyTagged { tag },
                },
                SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                },
            ) => {
                let (variant, ty) = match (
                    variants.0.get(*variant_index as usize),
                    variants.1.get(*variant_index as usize),
                ) {
                    (Some(variant), Some(ty)) => (variant, ty),
                    _ => return Err(serde::de::Error::custom("out of bounds variant index")),
                };

                let expecting = match (ty, value) {
                    (SerdeDataVariantType::Unit, SerdeDataVariantValue::Unit) => {
                        format!("the unit variant {}", variant)
                    }
                    (
                        SerdeDataVariantType::Newtype { .. },
                        SerdeDataVariantValue::Newtype { .. },
                    ) => format!("the newtype variant {}", variant),
                    (SerdeDataVariantType::Tuple { .. }, SerdeDataVariantValue::Struct { .. }) => {
                        return Err(serde::de::Error::custom(
                            "invalid serde internally tagged tuple variant",
                        ))
                    }
                    (SerdeDataVariantType::Struct { .. }, SerdeDataVariantValue::Struct { .. }) => {
                        format!("the struct variant {}", variant)
                    }
                    _ => return Err(serde::de::Error::custom("invalid serde enum data")),
                };

                enum DelayedVariantIdentifier {
                    Index(u64),
                    Str(String),
                    Bytes(Vec<u8>),
                }

                impl DelayedVariantIdentifier {
                    fn check_variant<E: serde::de::Error>(
                        self,
                        variant: &str,
                        variant_index: u32,
                    ) -> Result<(), E> {
                        match self {
                            DelayedVariantIdentifier::Index(v) if v == u64::from(variant_index) => {
                                Ok(())
                            }
                            DelayedVariantIdentifier::Index(v) => Err(serde::de::Error::custom(
                                format!("expected variant index {} found {}", variant_index, v),
                            )),
                            DelayedVariantIdentifier::Str(ref v) if v == variant => Ok(()),
                            DelayedVariantIdentifier::Str(ref v) => Err(serde::de::Error::custom(
                                format!("expected variant identifier {} found {}", variant, v),
                            )),
                            DelayedVariantIdentifier::Bytes(ref v) if v == variant.as_bytes() => {
                                Ok(())
                            }
                            DelayedVariantIdentifier::Bytes(ref v) => {
                                Err(serde::de::Error::custom(format!(
                                    "expected variant identifier {:?} found {:?}",
                                    variant.as_bytes(),
                                    v
                                )))
                            }
                        }
                    }
                }

                impl<'de> Deserialize<'de> for DelayedVariantIdentifier {
                    fn deserialize<D: Deserializer<'de>>(
                        deserializer: D,
                    ) -> Result<Self, D::Error> {
                        struct DelayedVariantIdentifierVisitor;

                        impl<'de> Visitor<'de> for DelayedVariantIdentifierVisitor {
                            type Value = DelayedVariantIdentifier;

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("a variant identifier")
                            }

                            fn visit_u64<E: serde::de::Error>(
                                self,
                                v: u64,
                            ) -> Result<Self::Value, E> {
                                Ok(DelayedVariantIdentifier::Index(v))
                            }

                            fn visit_str<E: serde::de::Error>(
                                self,
                                v: &str,
                            ) -> Result<Self::Value, E> {
                                Ok(DelayedVariantIdentifier::Str(String::from(v)))
                            }

                            fn visit_bytes<E: serde::de::Error>(
                                self,
                                v: &[u8],
                            ) -> Result<Self::Value, E> {
                                Ok(DelayedVariantIdentifier::Bytes(Vec::from(v)))
                            }
                        }

                        deserializer.deserialize_identifier(DelayedVariantIdentifierVisitor)
                    }
                }

                let (tag, content) =
                    deserializer.deserialize_any(serde::__private::de::TaggedContentVisitor::<
                        DelayedVariantIdentifier,
                    >::new(
                        unsafe { to_static_str(tag) },
                        unsafe { to_static_str(&expecting) },
                    ))?;
                tag.check_variant(variant, *variant_index)?;

                let deserializer =
                    serde::__private::de::ContentDeserializer::<D::Error>::new(content);

                match (ty, value) {
                    (SerdeDataVariantType::Unit, SerdeDataVariantValue::Unit) => deserializer
                        .deserialize_any(serde::__private::de::InternallyTaggedUnitVisitor::new(
                            name, variant,
                        )),
                    (
                        SerdeDataVariantType::Newtype { inner: ty },
                        SerdeDataVariantValue::Newtype { inner: value },
                    ) => BorrowedTypedSerdeData { ty, value }.deserialize(deserializer),
                    (
                        SerdeDataVariantType::Tuple { fields: _ },
                        SerdeDataVariantValue::Struct { fields: _ },
                    ) => Err(serde::de::Error::custom(
                        "invalid serde internally tagged tuple variant",
                    )),
                    (
                        SerdeDataVariantType::Struct { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        struct FieldIdentifierVisitor<'a> {
                            field: &'a str,
                            index: u64,
                        }

                        impl<'a, 'de> Visitor<'de> for FieldIdentifierVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("a field identifier")
                            }

                            fn visit_u64<E: serde::de::Error>(
                                self,
                                v: u64,
                            ) -> Result<Self::Value, E> {
                                if v == self.index {
                                    Ok(())
                                } else {
                                    Err(serde::de::Error::custom(format!(
                                        "expected field index {} found {}",
                                        self.index, v
                                    )))
                                }
                            }

                            fn visit_str<E: serde::de::Error>(
                                self,
                                v: &str,
                            ) -> Result<Self::Value, E> {
                                if v == self.field {
                                    Ok(())
                                } else {
                                    Err(serde::de::Error::custom(format!(
                                        "expected field identifier {} found {}",
                                        self.field, v
                                    )))
                                }
                            }

                            fn visit_bytes<E: serde::de::Error>(
                                self,
                                v: &[u8],
                            ) -> Result<Self::Value, E> {
                                if v == self.field.as_bytes() {
                                    Ok(())
                                } else {
                                    Err(serde::de::Error::custom(format!(
                                        "expected field identifier {:?} found {:?}",
                                        self.field.as_bytes(),
                                        v
                                    )))
                                }
                            }
                        }

                        impl<'a, 'de> DeserializeSeed<'de> for FieldIdentifierVisitor<'a> {
                            type Value = ();

                            fn deserialize<D: Deserializer<'de>>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error> {
                                deserializer.deserialize_identifier(self)
                            }
                        }

                        struct StructVariantVisitor<'a> {
                            variant: &'a str,
                            fields: &'a [&'a str],
                            tys: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for StructVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter
                                    .write_fmt(format_args!("the struct variant {}", self.variant))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (ty, expected) in self.tys.iter().zip(self.values.iter()) {
                                    seq.next_element_seed(BorrowedTypedSerdeData {
                                        ty,
                                        value: expected,
                                    })?;
                                }
                                Ok(())
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (((index, field), ty), expected) in (0..)
                                    .zip(self.fields.iter())
                                    .zip(self.tys.iter())
                                    .zip(self.values.iter())
                                {
                                    map.next_entry_seed(
                                        FieldIdentifierVisitor { field, index },
                                        BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        },
                                    )?;
                                }
                                Ok(())
                            }
                        }

                        if values.len() != fields.0.len() || values.len() != fields.1.len() {
                            return Err(serde::de::Error::custom("mismatch struct fields len"));
                        }

                        deserializer.deserialize_struct(
                            unsafe { to_static_str(name) },
                            unsafe { to_static_str_slice(&fields.0) },
                            StructVariantVisitor {
                                variant,
                                fields: &fields.0,
                                tys: &fields.1,
                                values,
                            },
                        )
                    }
                    _ => Err(serde::de::Error::custom("invalid serde enum data")),
                }
            }
            _ => Err(serde::de::Error::custom("invalid serde data")),
        }
    }
}

impl<'a> Arbitrary<'a> for TypedSerdeData<'a> {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let pretty_config = ArbitraryPrettyConfig::arbitrary(u)?.into();
        let mut ty = SerdeDataType::arbitrary(u)?;
        let value = ty.arbitrary_value(u, &pretty_config)?;
        Ok(Self {
            pretty_config,
            ty,
            value,
        })
    }
}

#[derive(Debug, Default, PartialEq, Arbitrary, Serialize)]
pub enum SerdeDataValue<'a> {
    #[default]
    Unit,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    ISize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    USize(usize),
    F32(f32),
    F64(f64),
    Char(char),
    String(&'a str),
    ByteBuf(&'a [u8]),
    Option {
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Option<Box<Self>>,
    },
    Seq {
        #[arbitrary(with = arbitrary_recursion_guard)]
        elems: Vec<Self>,
    },
    Map {
        #[arbitrary(with = arbitrary_recursion_guard)]
        elems: Vec<(Self, Self)>,
    },
    UnitStruct,
    Newtype {
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<Self>,
    },
    Struct {
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<Self>,
    },
    Enum {
        variant: u32,
        #[arbitrary(with = arbitrary_recursion_guard)]
        value: SerdeDataVariantValue<'a>,
    },
}

#[derive(Debug, Default, PartialEq, Arbitrary, Serialize)]
pub enum SerdeDataType<'a> {
    #[default]
    Unit,
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,
    F32,
    F64,
    Char,
    String,
    ByteBuf,
    Option {
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<Self>,
    },
    Array {
        #[arbitrary(with = arbitrary_recursion_guard)]
        kind: Box<Self>,
        #[arbitrary(value = ARRAY_UNINIT_LEN)]
        len: usize,
    },
    Tuple {
        #[arbitrary(with = arbitrary_recursion_guard)]
        elems: Vec<Self>,
    },
    Vec {
        #[arbitrary(with = arbitrary_recursion_guard)]
        item: Box<Self>,
    },
    Map {
        #[arbitrary(with = arbitrary_recursion_guard)]
        key: Box<Self>,
        #[arbitrary(with = arbitrary_recursion_guard)]
        value: Box<Self>,
    },
    UnitStruct {
        name: &'a str,
    },
    Newtype {
        name: &'a str,
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<Self>,
    },
    TupleStruct {
        name: &'a str,
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<Self>,
    },
    Struct {
        name: &'a str,
        #[arbitrary(with = arbitrary_str_tuple_vec_recursion_guard)]
        fields: (Vec<&'a str>, Vec<Self>),
    },
    Enum {
        name: &'a str,
        #[arbitrary(with = arbitrary_str_tuple_vec_recursion_guard)]
        variants: (Vec<&'a str>, Vec<SerdeDataVariantType<'a>>),
        representation: SerdeEnumRepresentation<'a>,
    },
}

#[derive(Debug, Default, PartialEq, Arbitrary, Serialize)]
pub enum SerdeEnumRepresentation<'a> {
    #[default]
    ExternallyTagged,
    InternallyTagged {
        tag: &'a str,
    },
    AdjacentlyTagged {
        tag: &'a str,
        content: &'a str,
    },
    Untagged,
}

impl<'a> SerdeDataType<'a> {
    fn arbitrary_value<'u>(
        &mut self,
        u: &mut Unstructured<'u>,
        pretty: &PrettyConfig,
    ) -> arbitrary::Result<SerdeDataValue<'u>> {
        let mut name_length: usize = 0;

        let value = match self {
            SerdeDataType::Unit => SerdeDataValue::Unit,
            SerdeDataType::Bool => SerdeDataValue::Bool(bool::arbitrary(u)?),
            SerdeDataType::I8 => SerdeDataValue::I8(i8::arbitrary(u)?),
            SerdeDataType::I16 => SerdeDataValue::I16(i16::arbitrary(u)?),
            SerdeDataType::I32 => SerdeDataValue::I32(i32::arbitrary(u)?),
            SerdeDataType::I64 => SerdeDataValue::I64(i64::arbitrary(u)?),
            SerdeDataType::I128 => SerdeDataValue::I128(i128::arbitrary(u)?),
            SerdeDataType::ISize => SerdeDataValue::ISize(isize::arbitrary(u)?),
            SerdeDataType::U8 => SerdeDataValue::U8(u8::arbitrary(u)?),
            SerdeDataType::U16 => SerdeDataValue::U16(u16::arbitrary(u)?),
            SerdeDataType::U32 => SerdeDataValue::U32(u32::arbitrary(u)?),
            SerdeDataType::U64 => SerdeDataValue::U64(u64::arbitrary(u)?),
            SerdeDataType::U128 => SerdeDataValue::U128(u128::arbitrary(u)?),
            SerdeDataType::USize => SerdeDataValue::USize(usize::arbitrary(u)?),
            SerdeDataType::F32 => SerdeDataValue::F32(f32::arbitrary(u)?),
            SerdeDataType::F64 => SerdeDataValue::F64(f64::arbitrary(u)?),
            SerdeDataType::Char => SerdeDataValue::Char(char::arbitrary(u)?),
            SerdeDataType::String => SerdeDataValue::String(<&str>::arbitrary(u)?),
            SerdeDataType::ByteBuf => SerdeDataValue::ByteBuf(<&[u8]>::arbitrary(u)?),
            SerdeDataType::Option { inner } => {
                let value = match Option::<()>::arbitrary(u)? {
                    Some(_) => Some(Box::new(inner.arbitrary_value(u, pretty)?)),
                    None => None,
                };
                SerdeDataValue::Option { inner: value }
            }
            SerdeDataType::Array { kind, len } => {
                let mut array = Vec::new();

                if *len == ARRAY_UNINIT_LEN {
                    // Actually initialise the array length with the first array instantiation
                    while u.arbitrary()? {
                        array.push(kind.arbitrary_value(u, pretty)?);
                    }
                    array.shrink_to_fit();
                    if array.is_empty() {
                        **kind = SerdeDataType::Unit;
                    }
                    *len = array.len();
                } else {
                    // Use the already-determined array length
                    array.reserve_exact(*len);
                    for _ in 0..*len {
                        array.push(kind.arbitrary_value(u, pretty)?);
                    }
                }

                SerdeDataValue::Seq { elems: array }
            }
            SerdeDataType::Tuple { elems } => {
                if elems.is_empty() {
                    *self = SerdeDataType::Unit;
                    return self.arbitrary_value(u, pretty);
                }

                let mut tuple = Vec::with_capacity(elems.len());
                for ty in elems {
                    tuple.push(ty.arbitrary_value(u, pretty)?);
                }
                SerdeDataValue::Seq { elems: tuple }
            }
            SerdeDataType::Vec { item } => {
                let mut vec = Vec::new();
                while u.arbitrary()? {
                    vec.push(item.arbitrary_value(u, pretty)?);
                }
                vec.shrink_to_fit();
                SerdeDataValue::Seq { elems: vec }
            }
            SerdeDataType::Map { key, value } => {
                let mut map = Vec::new();
                while u.arbitrary()? {
                    map.push((
                        key.arbitrary_value(u, pretty)?,
                        value.arbitrary_value(u, pretty)?,
                    ));
                }
                map.shrink_to_fit();
                SerdeDataValue::Map { elems: map }
            }
            SerdeDataType::UnitStruct { name } => {
                name_length += name.len();

                SerdeDataValue::UnitStruct
            }
            SerdeDataType::Newtype { name, inner } => {
                let inner = inner.arbitrary_value(u, pretty)?;

                // ron::value::RawValue cannot safely be constructed from syntactically invalid ron
                if *name == RAW_VALUE_TOKEN {
                    if let SerdeDataValue::String(ron) = &inner {
                        if ron::value::RawValue::from_ron(ron).is_err() {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }
                    }
                }

                name_length += name.len();

                SerdeDataValue::Newtype {
                    inner: Box::new(inner),
                }
            }
            SerdeDataType::TupleStruct { name, fields } => {
                name_length += name.len();
                let mut tuple = Vec::with_capacity(fields.len());
                for ty in fields {
                    tuple.push(ty.arbitrary_value(u, pretty)?);
                }
                SerdeDataValue::Struct { fields: tuple }
            }
            SerdeDataType::Struct { name, fields } => {
                name_length += name.len();
                let mut r#struct = Vec::with_capacity(fields.1.len());
                for (field, ty) in fields.0.iter().zip(&mut fields.1) {
                    name_length += field.len();
                    r#struct.push(ty.arbitrary_value(u, pretty)?);
                }
                SerdeDataValue::Struct { fields: r#struct }
            }
            SerdeDataType::Enum {
                name,
                variants,
                representation,
            } => {
                name_length += name.len();

                // BUG: struct names inside untagged do not roundtrip
                if matches!(
                    representation,
                    SerdeEnumRepresentation::Untagged |
                    SerdeEnumRepresentation::InternallyTagged { tag: _ }
                    if pretty.struct_names || pretty.extensions.contains(Extensions::IMPLICIT_SOME)
                ) {
                    return Err(arbitrary::Error::IncorrectFormat);
                }

                if matches!(
                    representation,
                    SerdeEnumRepresentation::AdjacentlyTagged { tag, content }
                    if tag == content
                ) {
                    return Err(arbitrary::Error::IncorrectFormat);
                }

                let variant_index = u.choose_index(variants.1.len())?;
                let (variant, ty) = match (
                    variants.0.get_mut(variant_index),
                    variants.1.get_mut(variant_index),
                ) {
                    (Some(variant), Some(ty)) => (variant, ty),
                    _ => return Err(arbitrary::Error::EmptyChoose),
                };
                let variant_index =
                    u32::try_from(variant_index).map_err(|_| arbitrary::Error::IncorrectFormat)?;

                name_length += variant.len();

                let value = match ty {
                    SerdeDataVariantType::Unit => SerdeDataVariantValue::Unit,
                    SerdeDataVariantType::Newtype { ref mut inner } => {
                        let value = SerdeDataVariantValue::Newtype {
                            inner: Box::new(inner.arbitrary_value(u, pretty)?),
                        };
                        if matches!(representation, SerdeEnumRepresentation::Untagged | SerdeEnumRepresentation::InternallyTagged { tag: _ } if !inner.supported_inside_untagged(pretty))
                        {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }
                        value
                    }
                    SerdeDataVariantType::Tuple { fields } => {
                        if matches!(
                            representation,
                            SerdeEnumRepresentation::InternallyTagged { tag: _ }
                        ) {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }

                        if matches!(
                            representation,
                            SerdeEnumRepresentation::Untagged if fields.is_empty()
                        ) {
                            // BUG: empty untagged tuple variant looks like a unit to ron
                            return Err(arbitrary::Error::IncorrectFormat);
                        }

                        if fields.len() == 1 {
                            // BUG: one-sized variant looks like a newtype variant to ron
                            return Err(arbitrary::Error::IncorrectFormat);
                        }

                        let mut tuple = Vec::with_capacity(fields.len());
                        for ty in fields.iter_mut() {
                            tuple.push(ty.arbitrary_value(u, pretty)?);
                        }
                        let value = SerdeDataVariantValue::Struct { fields: tuple };
                        if matches!(representation, SerdeEnumRepresentation::Untagged if !fields.iter().all(|field| field.supported_inside_untagged(pretty)))
                        {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }
                        value
                    }
                    SerdeDataVariantType::Struct { fields } => {
                        if matches!(
                            representation,
                            SerdeEnumRepresentation::Untagged if fields.0.is_empty()
                        ) {
                            // BUG: empty untagged struct variants look like units to ron
                            return Err(arbitrary::Error::IncorrectFormat);
                        }
                        let mut r#struct = Vec::with_capacity(fields.1.len());
                        for (field, ty) in fields.0.iter().zip(&mut fields.1) {
                            name_length += field.len();
                            r#struct.push(ty.arbitrary_value(u, pretty)?);
                        }
                        let value = SerdeDataVariantValue::Struct { fields: r#struct };
                        if matches!(representation, SerdeEnumRepresentation::Untagged | SerdeEnumRepresentation::InternallyTagged { tag: _ } if !fields.1.iter().all(|field| field.supported_inside_untagged(pretty)))
                        {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }
                        value
                    }
                };

                SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                }
            }
        };

        for _ in LONG_NAME_COST_THRESHOLD..name_length {
            // Enforce that producing long struct/field/enum/variant names costs per usage
            let _ = u.arbitrary::<bool>()?;
        }

        // Enforce that producing a value or adding a level is never free
        let _ = u.arbitrary::<bool>()?;

        Ok(value)
    }

    fn supported_inside_untagged(&self, pretty: &PrettyConfig) -> bool {
        match self {
            SerdeDataType::Unit => true,
            SerdeDataType::Bool => true,
            SerdeDataType::I8 => true,
            SerdeDataType::I16 => true,
            SerdeDataType::I32 => true,
            SerdeDataType::I64 => true,
            SerdeDataType::I128 => false, // BUG: serde content doesn't support i128 yet
            SerdeDataType::ISize => true,
            SerdeDataType::U8 => true,
            SerdeDataType::U16 => true,
            SerdeDataType::U32 => true,
            SerdeDataType::U64 => true,
            SerdeDataType::U128 => false, // BUG: serde content doesn't support u128 yet
            SerdeDataType::USize => true,
            SerdeDataType::F32 => true,
            SerdeDataType::F64 => true,
            SerdeDataType::Char => true,
            SerdeDataType::String => true,
            SerdeDataType::ByteBuf => true,
            SerdeDataType::Option { inner } => inner.supported_inside_untagged(pretty),
            SerdeDataType::Array { kind, len } => {
                if *len == 0 {
                    // BUG: a zero-length array look like a unit to ron
                    return false;
                }

                kind.supported_inside_untagged(pretty)
            }
            SerdeDataType::Tuple { elems } => elems
                .iter()
                .all(|element| element.supported_inside_untagged(pretty)),
            SerdeDataType::Vec { item } => item.supported_inside_untagged(pretty),
            SerdeDataType::Map { key, value } => {
                key.supported_inside_untagged(pretty) && value.supported_inside_untagged(pretty)
            }
            SerdeDataType::UnitStruct { name: _ } => true,
            SerdeDataType::Newtype { name: _, inner: _ } => {
                // if *name == RAW_VALUE_TOKEN {
                //     return false;
                // }

                // inner.supported_inside_untagged()

                false
            }
            SerdeDataType::TupleStruct { name: _, fields } => {
                if fields.is_empty() {
                    // BUG: an empty tuple struct looks like a unit to ron
                    return false;
                }

                fields
                    .iter()
                    .all(|field| field.supported_inside_untagged(pretty))
            }
            SerdeDataType::Struct { name: _, fields } => {
                if fields.0.is_empty() {
                    // BUG: an empty struct looks like a unit to ron
                    return false;
                }

                fields
                    .1
                    .iter()
                    .all(|field| field.supported_inside_untagged(pretty))
            }
            SerdeDataType::Enum {
                name: _,
                variants,
                representation: _,
            } => variants.1.iter().all(|variant| match variant {
                SerdeDataVariantType::Unit => true,
                SerdeDataVariantType::Newtype { inner } => inner.supported_inside_untagged(pretty),
                SerdeDataVariantType::Tuple { fields } => {
                    if fields.is_empty() {
                        // BUG: an empty tuple struct looks like a unit to ron
                        return false;
                    }

                    fields
                        .iter()
                        .all(|field| field.supported_inside_untagged(pretty))
                }
                SerdeDataVariantType::Struct { fields } => {
                    if fields.0.is_empty() {
                        // BUG: an empty struct looks like a unit to ron
                        return false;
                    }

                    fields
                        .1
                        .iter()
                        .all(|field| field.supported_inside_untagged(pretty))
                }
            }),
        }
    }
}

#[derive(Debug, Default, PartialEq, Arbitrary, Serialize)]
pub enum SerdeDataVariantType<'a> {
    #[default]
    Unit,
    Newtype {
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<SerdeDataType<'a>>,
    },
    Tuple {
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<SerdeDataType<'a>>,
    },
    Struct {
        #[arbitrary(with = arbitrary_str_tuple_vec_recursion_guard)]
        fields: (Vec<&'a str>, Vec<SerdeDataType<'a>>),
    },
}

#[derive(Debug, Default, PartialEq, Arbitrary, Serialize)]
pub enum SerdeDataVariantValue<'a> {
    #[default]
    Unit,
    Newtype {
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<SerdeDataValue<'a>>,
    },
    Struct {
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<SerdeDataValue<'a>>,
    },
}

static RECURSION_DEPTH: AtomicUsize = AtomicUsize::new(0);

fn arbitrary_recursion_guard<'a, T: Arbitrary<'a> + Default>(
    u: &mut Unstructured<'a>,
) -> arbitrary::Result<T> {
    let max_depth = RECURSION_LIMIT * 2;

    let result = if RECURSION_DEPTH.fetch_add(1, Ordering::Relaxed) < max_depth {
        T::arbitrary(u)
    } else {
        Ok(T::default())
    };

    RECURSION_DEPTH.fetch_sub(1, Ordering::Relaxed);

    result
}

fn arbitrary_str_tuple_vec_recursion_guard<'a, T: Arbitrary<'a>>(
    u: &mut Unstructured<'a>,
) -> arbitrary::Result<(Vec<&'a str>, Vec<T>)> {
    let max_depth = RECURSION_LIMIT * 2;

    let result = if RECURSION_DEPTH.fetch_add(1, Ordering::Relaxed) < max_depth {
        let mut s = Vec::new();
        let mut v = Vec::new();

        while u.arbitrary()? {
            s.push(<&str>::arbitrary(u)?);
            v.push(T::arbitrary(u)?);
        }

        s.shrink_to_fit();
        v.shrink_to_fit();

        Ok((s, v))
    } else {
        Ok((Vec::new(), Vec::new()))
    };

    RECURSION_DEPTH.fetch_sub(1, Ordering::Relaxed);

    result
}
