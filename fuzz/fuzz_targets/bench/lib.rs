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

const FLATTEN_CONFLICT_MSG: &str = "ron::fuzz::FlattenFieldConflict";

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
            // The fuzzer may produce flattened structs with conflicting fields,
            //  which we only detect during serialising
            Err(ron::error::Error::Message(msg)) if msg == FLATTEN_CONFLICT_MSG => return None,
            // Everything else is actually a bug we want to find
            Err(err) => panic!("{:#?} -! {:#?}", typed_value, err),
        };

        if let Err(err) = options.from_str::<ron::Value>(&ron) {
            match err.code {
                // Erroring on deep recursion is better than crashing on a stack overflow
                ron::error::Error::ExceededRecursionLimit => return None,
                // Everything else is actually a bug we want to find
                _ => panic!("{:#?} -> {} -! {:#?}", typed_value, ron, err),
            }
        };

        if let Err((err, path)) =
            (|| -> Result<(), (ron::error::SpannedError, Option<serde_path_to_error::Path>)> {
                let mut deserializer = ron::de::Deserializer::from_str_with_options(&ron, &options)
                    .map_err(|err| (err, None))?;
                let mut track = serde_path_to_error::Track::new();
                match typed_value.deserialize(serde_path_to_error::Deserializer::new(
                    &mut deserializer,
                    &mut track,
                )) {
                    Ok(()) => Ok(()),
                    Err(err) => Err((deserializer.span_error(err), Some(track.path()))),
                }?;
                deserializer
                    .end()
                    .map_err(|e| deserializer.span_error(e))
                    .map_err(|err| (err, None))?;
                Ok(())
            })()
        {
            match err.code {
                // Erroring on deep recursion is better than crashing on a stack overflow
                ron::error::Error::ExceededRecursionLimit => return None,
                // Duplicate struct fields only cause issues inside internally (or adjacently)
                //  tagged or untagged enums (or in flattened fields where we detect them
                //  before they cause issues), so we allow them in arbitrary otherwise
                ron::error::Error::DuplicateStructField { .. } => return None,
                // Everything else is actually a bug we want to find
                _ => panic!("{:#?} -> {} -! {:#?} @ {:#?}", typed_value, ron, err, path),
            }
        };

        Some(typed_value)
    } else {
        None
    }
}

// NOTE: Keep synchronised with ron::value::raw::RAW_VALUE_TOKEN
const RAW_VALUE_TOKEN: &str = "$ron::private::RawValue";

#[allow(clippy::struct_excessive_bools)]
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
            (
                SerdeDataType::Struct {
                    name,
                    tag: _,
                    fields,
                },
                SerdeDataValue::Struct { fields: values },
            ) => {
                if values.len() != fields.0.len()
                    || values.len() != fields.1.len()
                    || values.len() != fields.2.len()
                {
                    return Err(serde::ser::Error::custom("mismatch struct fields len"));
                }

                if fields.2.iter().any(|x| *x) {
                    struct FlatMapKeyConflictDetector<'a, M: SerializeMap> {
                        keys: &'a mut Vec<String>,
                        map: &'a mut M,
                    }

                    impl<'a, M: SerializeMap> SerializeMap for FlatMapKeyConflictDetector<'a, M> {
                        type Ok = ();
                        type Error = M::Error;

                        fn serialize_key<T: ?Sized + Serialize>(
                            &mut self,
                            key: &T,
                        ) -> Result<(), Self::Error> {
                            #[allow(clippy::unwrap_used)] // FIXME
                            let key_str = ron::to_string(key).unwrap();
                            if self.keys.contains(&key_str) {
                                return Err(serde::ser::Error::custom(FLATTEN_CONFLICT_MSG));
                            }
                            self.keys.push(key_str);

                            self.map.serialize_key(key)
                        }

                        fn serialize_value<T: ?Sized + Serialize>(
                            &mut self,
                            value: &T,
                        ) -> Result<(), Self::Error> {
                            self.map.serialize_value(value)
                        }

                        fn end(self) -> Result<Self::Ok, Self::Error> {
                            Ok(())
                        }

                        fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(
                            &mut self,
                            key: &K,
                            value: &V,
                        ) -> Result<(), Self::Error> {
                            #[allow(clippy::unwrap_used)] // FIXME
                            let key_str = ron::to_string(key).unwrap();
                            if self.keys.contains(&key_str) {
                                return Err(serde::ser::Error::custom(FLATTEN_CONFLICT_MSG));
                            }
                            self.keys.push(key_str);

                            self.map.serialize_entry(key, value)
                        }
                    }

                    let mut flattened_keys = Vec::new();

                    let mut map = serializer.serialize_map(None)?;
                    for (((field, ty), flatten), data) in fields
                        .0
                        .iter()
                        .zip(fields.1.iter())
                        .zip(fields.2.iter())
                        .zip(values.iter())
                    {
                        if *flatten {
                            (&BorrowedTypedSerdeData { ty, value: data }
                                as &dyn erased_serde::Serialize)
                                .serialize(serde::__private::ser::FlatMapSerializer(
                                    &mut FlatMapKeyConflictDetector {
                                        keys: &mut flattened_keys,
                                        map: &mut map,
                                    },
                                ))?;
                        } else {
                            #[allow(clippy::unwrap_used)] // FIXME
                            let field_str = ron::to_string(field).unwrap();
                            if flattened_keys.contains(&field_str) {
                                return Err(serde::ser::Error::custom(FLATTEN_CONFLICT_MSG));
                            }
                            flattened_keys.push(field_str);

                            map.serialize_entry(
                                field,
                                &BorrowedTypedSerdeData { ty, value: data },
                            )?;
                        }
                    }
                    map.end()
                } else {
                    let mut r#struct = serializer
                        .serialize_struct(unsafe { to_static_str(name) }, values.len())?;
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
                        SerdeDataVariantType::TaggedOther,
                        SerdeDataVariantValue::TaggedOther {
                            variant: other_variant,
                            index: other_variant_index,
                        },
                    ) => match representation {
                        SerdeEnumRepresentation::ExternallyTagged => {
                            Err(serde::ser::Error::custom(
                                "invalid serde enum data: tagged other in externally tagged",
                            ))
                        }
                        SerdeEnumRepresentation::Untagged => Err(serde::ser::Error::custom(
                            "invalid serde enum data: tagged other in untagged",
                        )),
                        SerdeEnumRepresentation::AdjacentlyTagged { tag, content: _ } => {
                            let mut r#struct =
                                serializer.serialize_struct(unsafe { to_static_str(name) }, 1)?;
                            r#struct.serialize_field(
                                unsafe { to_static_str(tag) },
                                &serde::__private::ser::AdjacentlyTaggedEnumVariant {
                                    enum_name: unsafe { to_static_str(name) },
                                    variant_index: *other_variant_index,
                                    variant_name: unsafe { to_static_str(other_variant) },
                                },
                            )?;
                            r#struct.end()
                        }
                        SerdeEnumRepresentation::InternallyTagged { tag } => {
                            let mut r#struct =
                                serializer.serialize_struct(unsafe { to_static_str(name) }, 1)?;
                            r#struct
                                .serialize_field(unsafe { to_static_str(tag) }, other_variant)?;
                            r#struct.end()
                        }
                    },
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
                        if values.len() != fields.0.len()
                            || values.len() != fields.1.len()
                            || values.len() != fields.2.len()
                        {
                            return Err(serde::ser::Error::custom(
                                "mismatch struct variant fields len",
                            ));
                        }

                        struct UntaggedStruct<'a> {
                            name: &'a str,
                            fields: &'a (Vec<&'a str>, Vec<SerdeDataType<'a>>, Vec<bool>),
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

                        if fields.2.iter().any(|x| *x) {
                            struct FlatMapKeyConflictDetector<'a, M: SerializeMap> {
                                keys: &'a mut Vec<String>,
                                map: &'a mut M,
                            }

                            impl<'a, M: SerializeMap> SerializeMap for FlatMapKeyConflictDetector<'a, M> {
                                type Ok = ();
                                type Error = M::Error;

                                fn serialize_key<T: ?Sized + Serialize>(
                                    &mut self,
                                    key: &T,
                                ) -> Result<(), Self::Error> {
                                    #[allow(clippy::unwrap_used)] // FIXME
                                    let key_str = ron::to_string(key).unwrap();
                                    if self.keys.contains(&key_str) {
                                        return Err(serde::ser::Error::custom(
                                            FLATTEN_CONFLICT_MSG,
                                        ));
                                    }
                                    self.keys.push(key_str);

                                    self.map.serialize_key(key)
                                }

                                fn serialize_value<T: ?Sized + Serialize>(
                                    &mut self,
                                    value: &T,
                                ) -> Result<(), Self::Error> {
                                    self.map.serialize_value(value)
                                }

                                fn end(self) -> Result<Self::Ok, Self::Error> {
                                    Ok(())
                                }

                                fn serialize_entry<K: ?Sized + Serialize, V: ?Sized + Serialize>(
                                    &mut self,
                                    key: &K,
                                    value: &V,
                                ) -> Result<(), Self::Error> {
                                    #[allow(clippy::unwrap_used)] // FIXME
                                    let key_str = ron::to_string(key).unwrap();
                                    if self.keys.contains(&key_str) {
                                        return Err(serde::ser::Error::custom(
                                            FLATTEN_CONFLICT_MSG,
                                        ));
                                    }
                                    self.keys.push(key_str);

                                    self.map.serialize_entry(key, value)
                                }
                            }

                            struct FlattenedStructVariant<'a> {
                                fields: &'a (Vec<&'a str>, Vec<SerdeDataType<'a>>, Vec<bool>),
                                values: &'a [SerdeDataValue<'a>],
                            }

                            impl<'a> Serialize for FlattenedStructVariant<'a> {
                                fn serialize<S: Serializer>(
                                    &self,
                                    serializer: S,
                                ) -> Result<S::Ok, S::Error> {
                                    let mut flattened_keys = Vec::new();

                                    let mut map = serializer.serialize_map(None)?;
                                    for (((field, ty), flatten), data) in self
                                        .fields
                                        .0
                                        .iter()
                                        .zip(self.fields.1.iter())
                                        .zip(self.fields.2.iter())
                                        .zip(self.values.iter())
                                    {
                                        if *flatten {
                                            (&BorrowedTypedSerdeData { ty, value: data }
                                                as &dyn erased_serde::Serialize)
                                                .serialize(
                                                    serde::__private::ser::FlatMapSerializer(
                                                        &mut FlatMapKeyConflictDetector {
                                                            keys: &mut flattened_keys,
                                                            map: &mut map,
                                                        },
                                                    ),
                                                )?;
                                        } else {
                                            #[allow(clippy::unwrap_used)] // FIXME
                                            let field_str = ron::to_string(field).unwrap();
                                            if flattened_keys.contains(&field_str) {
                                                return Err(serde::ser::Error::custom(
                                                    FLATTEN_CONFLICT_MSG,
                                                ));
                                            }
                                            flattened_keys.push(field_str);

                                            map.serialize_entry(
                                                field,
                                                &BorrowedTypedSerdeData { ty, value: data },
                                            )?;
                                        }
                                    }
                                    map.end()
                                }
                            }

                            match representation {
                                SerdeEnumRepresentation::ExternallyTagged => serializer
                                    .serialize_newtype_variant(
                                        unsafe { to_static_str(name) },
                                        *variant_index,
                                        unsafe { to_static_str(variant) },
                                        &FlattenedStructVariant { fields, values },
                                    ),
                                SerdeEnumRepresentation::Untagged => {
                                    let mut flattened_keys = Vec::new();

                                    let mut map = serializer.serialize_map(None)?;
                                    for (((field, ty), flatten), data) in fields
                                        .0
                                        .iter()
                                        .zip(fields.1.iter())
                                        .zip(fields.2.iter())
                                        .zip(values.iter())
                                    {
                                        if *flatten {
                                            (&BorrowedTypedSerdeData { ty, value: data }
                                                as &dyn erased_serde::Serialize)
                                                .serialize(
                                                    serde::__private::ser::FlatMapSerializer(
                                                        &mut FlatMapKeyConflictDetector {
                                                            keys: &mut flattened_keys,
                                                            map: &mut map,
                                                        },
                                                    ),
                                                )?;
                                        } else {
                                            #[allow(clippy::unwrap_used)] // FIXME
                                            let field_str = ron::to_string(field).unwrap();
                                            if flattened_keys.contains(&field_str) {
                                                return Err(serde::ser::Error::custom(
                                                    FLATTEN_CONFLICT_MSG,
                                                ));
                                            }
                                            flattened_keys.push(field_str);

                                            map.serialize_entry(
                                                field,
                                                &BorrowedTypedSerdeData { ty, value: data },
                                            )?;
                                        }
                                    }
                                    map.end()
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
                                        &FlattenedStructVariant { fields, values },
                                    )?;
                                    r#struct.end()
                                }
                                SerdeEnumRepresentation::InternallyTagged { tag } => {
                                    let mut flattened_keys = vec![String::from(*tag)];

                                    let mut map = serializer.serialize_map(None)?;
                                    map.serialize_entry(tag, variant)?;
                                    for (((field, ty), flatten), data) in fields
                                        .0
                                        .iter()
                                        .zip(fields.1.iter())
                                        .zip(fields.2.iter())
                                        .zip(values.iter())
                                    {
                                        if *flatten {
                                            (&BorrowedTypedSerdeData { ty, value: data }
                                                as &dyn erased_serde::Serialize)
                                                .serialize(
                                                    serde::__private::ser::FlatMapSerializer(
                                                        &mut FlatMapKeyConflictDetector {
                                                            keys: &mut flattened_keys,
                                                            map: &mut map,
                                                        },
                                                    ),
                                                )?;
                                        } else {
                                            #[allow(clippy::unwrap_used)] // FIXME
                                            let field_str = ron::to_string(field).unwrap();
                                            if flattened_keys.contains(&field_str) {
                                                return Err(serde::ser::Error::custom(
                                                    FLATTEN_CONFLICT_MSG,
                                                ));
                                            }
                                            flattened_keys.push(field_str);

                                            map.serialize_entry(
                                                field,
                                                &BorrowedTypedSerdeData { ty, value: data },
                                            )?;
                                        }
                                    }
                                    map.end()
                                }
                            }
                        } else {
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
                                    r#struct
                                        .serialize_field(unsafe { to_static_str(tag) }, variant)?;
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
                    "expected {check:?} found {value:?}"
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

                if (v.is_nan() && value.is_nan()) || (value.to_bits() == v.to_bits()) {
                    Ok(())
                } else {
                    Err(serde::de::Error::custom(format!(
                        "expected {v:?} found {value:?}"
                    )))
                }
            }
            (SerdeDataType::F64, SerdeDataValue::F64(v)) => {
                let value = f64::deserialize(deserializer)?;

                if (v.is_nan() && value.is_nan()) || (value.to_bits() == v.to_bits()) {
                    Ok(())
                } else {
                    Err(serde::de::Error::custom(format!(
                        "expected {v:?} found {value:?}"
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

                    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                        if self.value.is_none() {
                            Ok(())
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "expected {:?} found None-like unit",
                                self.value
                            )))
                        }
                    }

                    fn __private_visit_untagged_option<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, ()> {
                        match self.value {
                            None => Ok(()),
                            Some(expected) => BorrowedTypedSerdeData {
                                ty: self.ty,
                                value: expected,
                            }
                            .deserialize(deserializer)
                            .map_err(|err| {
                                panic!(
                                    "expected untagged {:?} but failed with {}",
                                    Some(expected),
                                    err
                                )
                            }),
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
                        for (i, expected) in self.elems.iter().enumerate() {
                            let Some(()) = seq.next_element_seed(BorrowedTypedSerdeData {
                                ty: self.kind,
                                value: expected,
                            })?
                            else {
                                return Err(serde::de::Error::invalid_length(i, &self));
                            };
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
                        for (i, (ty, expected)) in
                            self.tys.iter().zip(self.values.iter()).enumerate()
                        {
                            let Some(()) = seq.next_element_seed(BorrowedTypedSerdeData {
                                ty,
                                value: expected,
                            })?
                            else {
                                return Err(serde::de::Error::invalid_length(i, &self));
                            };
                        }
                        Ok(())
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
                        for (i, expected) in self.elems.iter().enumerate() {
                            let Some(()) = seq.next_element_seed(BorrowedTypedSerdeData {
                                ty: self.item,
                                value: expected,
                            })?
                            else {
                                return Err(serde::de::Error::invalid_length(
                                    i,
                                    &format!("a sequence of length {}", self.elems.len()).as_str(),
                                ));
                            };
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
                        for (i, (ekey, eval)) in self.elems.iter().enumerate() {
                            let Some(((), ())) = map.next_entry_seed(
                                BorrowedTypedSerdeData {
                                    ty: self.key,
                                    value: ekey,
                                },
                                BorrowedTypedSerdeData {
                                    ty: self.value,
                                    value: eval,
                                },
                            )?
                            else {
                                return Err(serde::de::Error::invalid_length(
                                    i,
                                    &format!("a map with {} elements", self.elems.len()).as_str(),
                                ));
                            };
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
                        formatter.write_fmt(format_args!("unit struct {}", self.name))
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
                        formatter.write_fmt(format_args!("tuple struct {}", self.name))
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

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        let Some(()) = seq.next_element_seed(BorrowedTypedSerdeData {
                            ty: self.inner,
                            value: self.value,
                        })?
                        else {
                            return Err(serde::de::Error::invalid_length(
                                0,
                                &format!("tuple struct {} with 1 element", self.name).as_str(),
                            ));
                        };
                        Ok(())
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        // ron::value::RawValue expects a visit_str call
                        //  even though it's disguised as a newtype
                        if self.name == RAW_VALUE_TOKEN {
                            if let SerdeDataValue::String(ron) = &self.value {
                                if let (Ok(v_ron), Ok(ron)) = (
                                    ron::value::RawValue::from_ron(v),
                                    ron::value::RawValue::from_ron(ron),
                                ) {
                                    // pretty serialising can add whitespace and comments
                                    //  before and after the raw value
                                    if v_ron.trim().get_ron() == ron.trim().get_ron() {
                                        return Ok(());
                                    }
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
                        formatter.write_fmt(format_args!("tuple struct {}", self.name))
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        for (i, (ty, expected)) in
                            self.fields.iter().zip(self.values.iter()).enumerate()
                        {
                            let Some(()) = seq.next_element_seed(BorrowedTypedSerdeData {
                                ty,
                                value: expected,
                            })?
                            else {
                                return Err(serde::de::Error::invalid_length(
                                    i,
                                    &format!(
                                        "tuple struct {} with {} elements",
                                        self.name,
                                        self.values.len()
                                    )
                                    .as_str(),
                                ));
                            };
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
            (
                SerdeDataType::Struct {
                    name,
                    tag: _,
                    fields,
                },
                SerdeDataValue::Struct { fields: values },
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
                                "expected field identifier {:?} found {:?}",
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

                struct MaybeFlattenFieldIdentifierVisitor<'a> {
                    field: Option<&'a str>,
                }

                impl<'a, 'de> Visitor<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                    type Value = Option<serde::__private::de::Content<'de>>;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a field identifier")
                    }

                    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::Bool(v)))
                    }

                    fn visit_i8<E: serde::de::Error>(self, v: i8) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::I8(v)))
                    }

                    fn visit_i16<E: serde::de::Error>(self, v: i16) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::I16(v)))
                    }

                    fn visit_i32<E: serde::de::Error>(self, v: i32) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::I32(v)))
                    }

                    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::I64(v)))
                    }

                    // BUG: serde does not yet support i128 here
                    // fn visit_i128<E: serde::de::Error>(self, v: i128) -> Result<Self::Value, E> {
                    //     Ok(Some(serde::__private::de::Content::I128(v)))
                    // }

                    fn visit_u8<E: serde::de::Error>(self, v: u8) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::U8(v)))
                    }

                    fn visit_u16<E: serde::de::Error>(self, v: u16) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::U16(v)))
                    }

                    fn visit_u32<E: serde::de::Error>(self, v: u32) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::U32(v)))
                    }

                    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::U64(v)))
                    }

                    // BUG: serde does not yet support u128 here
                    // fn visit_u128<E: serde::de::Error>(self, v: u128) -> Result<Self::Value, E> {
                    //     Ok(Some(serde::__private::de::Content::U128(v)))
                    // }

                    fn visit_f32<E: serde::de::Error>(self, v: f32) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::F32(v)))
                    }

                    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::F64(v)))
                    }

                    fn visit_char<E: serde::de::Error>(self, v: char) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::Char(v)))
                    }

                    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        if matches!(self.field, Some(field) if v == field) {
                            Ok(None)
                        } else {
                            Ok(Some(serde::__private::de::Content::String(String::from(v))))
                        }
                    }

                    fn visit_borrowed_str<E: serde::de::Error>(
                        self,
                        v: &'de str,
                    ) -> Result<Self::Value, E> {
                        if matches!(self.field, Some(field) if v == field) {
                            Ok(None)
                        } else {
                            Ok(Some(serde::__private::de::Content::Str(v)))
                        }
                    }

                    fn visit_string<E: serde::de::Error>(
                        self,
                        v: String,
                    ) -> Result<Self::Value, E> {
                        if matches!(self.field, Some(field) if v == field) {
                            Ok(None)
                        } else {
                            Ok(Some(serde::__private::de::Content::String(v)))
                        }
                    }

                    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                        if matches!(self.field, Some(field) if v == field.as_bytes()) {
                            Ok(None)
                        } else {
                            Ok(Some(serde::__private::de::Content::ByteBuf(Vec::from(v))))
                        }
                    }

                    fn visit_borrowed_bytes<E: serde::de::Error>(
                        self,
                        v: &'de [u8],
                    ) -> Result<Self::Value, E> {
                        if matches!(self.field, Some(field) if v == field.as_bytes()) {
                            Ok(None)
                        } else {
                            Ok(Some(serde::__private::de::Content::Bytes(v)))
                        }
                    }

                    fn visit_byte_buf<E: serde::de::Error>(
                        self,
                        v: Vec<u8>,
                    ) -> Result<Self::Value, E> {
                        if matches!(self.field, Some(field) if v == field.as_bytes()) {
                            Ok(None)
                        } else {
                            Ok(Some(serde::__private::de::Content::ByteBuf(v)))
                        }
                    }

                    fn visit_unit<E>(self) -> Result<Self::Value, E> {
                        Ok(Some(serde::__private::de::Content::Unit))
                    }
                }

                impl<'a, 'de> DeserializeSeed<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                    type Value = Option<serde::__private::de::Content<'de>>;

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
                        formatter.write_fmt(format_args!("struct {}", self.name))
                    }

                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error> {
                        for (i, (ty, expected)) in
                            self.tys.iter().zip(self.values.iter()).enumerate()
                        {
                            let Some(()) = seq.next_element_seed(BorrowedTypedSerdeData {
                                ty,
                                value: expected,
                            })?
                            else {
                                return Err(serde::de::Error::invalid_length(
                                    i,
                                    &format!(
                                        "struct {} with {} elements",
                                        self.name,
                                        self.values.len()
                                    )
                                    .as_str(),
                                ));
                            };
                        }
                        Ok(())
                    }

                    fn visit_map<A: MapAccess<'de>>(
                        self,
                        mut map: A,
                    ) -> Result<Self::Value, A::Error> {
                        for (i, (((index, field), ty), expected)) in (0..)
                            .zip(self.fields.iter())
                            .zip(self.tys.iter())
                            .zip(self.values.iter())
                            .enumerate()
                        {
                            let Some(((), ())) = map.next_entry_seed(
                                FieldIdentifierVisitor { field, index },
                                BorrowedTypedSerdeData {
                                    ty,
                                    value: expected,
                                },
                            )?
                            else {
                                return Err(serde::de::Error::invalid_length(
                                    i,
                                    &format!(
                                        "struct {} with {} elements",
                                        self.name,
                                        self.values.len()
                                    )
                                    .as_str(),
                                ));
                            };
                        }
                        // flattened structs are incompatible with strict fields
                        while map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                            map.next_value::<serde::de::IgnoredAny>().map(|_| ())?;
                        }
                        Ok(())
                    }
                }

                struct FlattenStructVisitor<'a> {
                    name: &'a str,
                    fields: &'a [&'a str],
                    tys: &'a [SerdeDataType<'a>],
                    flatten: &'a [bool],
                    values: &'a [SerdeDataValue<'a>],
                }

                impl<'a, 'de> Visitor<'de> for FlattenStructVisitor<'a> {
                    type Value = ();

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        // ron's flattened struct canary depends on the expecting
                        //  message to start with "struct "
                        formatter.write_fmt(format_args!("struct {}", self.name))
                    }

                    fn visit_map<A: MapAccess<'de>>(
                        self,
                        mut map: A,
                    ) -> Result<Self::Value, A::Error> {
                        let mut collect = Vec::<
                            Option<(serde::__private::de::Content, serde::__private::de::Content)>,
                        >::new();

                        for (((field, ty), flatten), expected) in self
                            .fields
                            .iter()
                            .zip(self.tys.iter())
                            .zip(self.flatten.iter())
                            .zip(self.values.iter())
                        {
                            if !*flatten {
                                while let Some(Some(key)) =
                                    map.next_key_seed(MaybeFlattenFieldIdentifierVisitor {
                                        field: Some(field),
                                    })?
                                {
                                    collect.push(Some((key, map.next_value()?)));
                                }

                                map.next_value_seed(BorrowedTypedSerdeData {
                                    ty,
                                    value: expected,
                                })?;
                            }
                        }

                        while let Some(Some(key)) =
                            map.next_key_seed(MaybeFlattenFieldIdentifierVisitor { field: None })?
                        {
                            collect.push(Some((key, map.next_value()?)));
                        }

                        for ((ty, flatten), expected) in self
                            .tys
                            .iter()
                            .zip(self.flatten.iter())
                            .zip(self.values.iter())
                        {
                            if *flatten {
                                BorrowedTypedSerdeData {
                                    ty,
                                    value: expected,
                                }
                                .deserialize(
                                    serde::__private::de::FlatMapDeserializer(
                                        &mut collect,
                                        std::marker::PhantomData,
                                    ),
                                )?;
                            }
                        }

                        Ok(())
                    }
                }

                if values.len() != fields.0.len()
                    || values.len() != fields.1.len()
                    || values.len() != fields.2.len()
                {
                    return Err(serde::de::Error::custom("mismatch struct fields len"));
                }

                if fields.2.iter().any(|x| *x) {
                    deserializer.deserialize_map(FlattenStructVisitor {
                        name,
                        fields: &fields.0,
                        tys: &fields.1,
                        flatten: &fields.2,
                        values,
                    })
                } else {
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
                        formatter
                            .write_fmt(format_args!("enum variant {}::{}", self.name, self.variant))
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
                                SerdeDataVariantType::TaggedOther,
                                SerdeDataVariantValue::TaggedOther { .. },
                            ) => Err(serde::de::Error::custom(
                                "invalid serde enum data: tagged other in externally tagged",
                            )),
                            (
                                SerdeDataVariantType::Newtype { inner: ty },
                                SerdeDataVariantValue::Newtype { inner: value },
                            ) => variant.newtype_variant_seed(BorrowedTypedSerdeData { ty, value }),
                            (
                                SerdeDataVariantType::Tuple { fields },
                                SerdeDataVariantValue::Struct { fields: values },
                            ) => {
                                struct TupleVariantVisitor<'a> {
                                    name: &'a str,
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
                                            "tuple variant {}::{}",
                                            self.name, self.variant
                                        ))
                                    }

                                    fn visit_seq<A: SeqAccess<'de>>(
                                        self,
                                        mut seq: A,
                                    ) -> Result<Self::Value, A::Error>
                                    {
                                        for (i, (ty, expected)) in
                                            self.fields.iter().zip(self.values.iter()).enumerate()
                                        {
                                            let Some(()) =
                                                seq.next_element_seed(BorrowedTypedSerdeData {
                                                    ty,
                                                    value: expected,
                                                })?
                                            else {
                                                return Err(serde::de::Error::invalid_length(
                                                    i,
                                                    &format!(
                                                        "tuple variant {}::{} with {} elements",
                                                        self.name,
                                                        self.variant,
                                                        self.values.len()
                                                    )
                                                    .as_str(),
                                                ));
                                            };
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
                                        name: self.name,
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
                                                "expected field identifier {:?} found {:?}",
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
                                    name: &'a str,
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
                                            "struct variant {}::{}",
                                            self.name, self.variant
                                        ))
                                    }

                                    fn visit_seq<A: SeqAccess<'de>>(
                                        self,
                                        mut seq: A,
                                    ) -> Result<Self::Value, A::Error>
                                    {
                                        for (i, (ty, expected)) in
                                            self.tys.iter().zip(self.values.iter()).enumerate()
                                        {
                                            let Some(()) =
                                                seq.next_element_seed(BorrowedTypedSerdeData {
                                                    ty,
                                                    value: expected,
                                                })?
                                            else {
                                                return Err(serde::de::Error::invalid_length(
                                                    i,
                                                    &format!(
                                                        "struct variant {}::{} with {} elements",
                                                        self.name,
                                                        self.variant,
                                                        self.values.len()
                                                    )
                                                    .as_str(),
                                                ));
                                            };
                                        }
                                        Ok(())
                                    }

                                    fn visit_map<A: MapAccess<'de>>(
                                        self,
                                        mut map: A,
                                    ) -> Result<Self::Value, A::Error>
                                    {
                                        for (i, (((index, field), ty), expected)) in (0..)
                                            .zip(self.fields.iter())
                                            .zip(self.tys.iter())
                                            .zip(self.values.iter())
                                            .enumerate()
                                        {
                                            let Some(((), ())) = map.next_entry_seed(
                                                FieldIdentifierVisitor { field, index },
                                                BorrowedTypedSerdeData {
                                                    ty,
                                                    value: expected,
                                                },
                                            )?
                                            else {
                                                return Err(serde::de::Error::invalid_length(
                                                    i,
                                                    &format!(
                                                        "struct variant {}::{} with {} elements",
                                                        self.name,
                                                        self.variant,
                                                        self.values.len()
                                                    )
                                                    .as_str(),
                                                ));
                                            };
                                        }
                                        // flattened struct variants are incompatible with strict fields
                                        while map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                                            map.next_value::<serde::de::IgnoredAny>()
                                                .map(|_| ())?;
                                        }
                                        Ok(())
                                    }
                                }

                                if values.len() != fields.0.len()
                                    || values.len() != fields.1.len()
                                    || values.len() != fields.2.len()
                                {
                                    return Err(serde::de::Error::custom(
                                        "mismatch struct fields len",
                                    ));
                                }

                                if fields.2.iter().any(|x| *x) {
                                    struct MaybeFlattenFieldIdentifierVisitor<'a> {
                                        field: Option<&'a str>,
                                    }

                                    impl<'a, 'de> Visitor<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                                        type Value = Option<serde::__private::de::Content<'de>>;

                                        fn expecting(
                                            &self,
                                            formatter: &mut fmt::Formatter,
                                        ) -> fmt::Result {
                                            formatter.write_str("a field identifier")
                                        }

                                        fn visit_bool<E: serde::de::Error>(
                                            self,
                                            v: bool,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::Bool(v)))
                                        }

                                        fn visit_i8<E: serde::de::Error>(
                                            self,
                                            v: i8,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::I8(v)))
                                        }

                                        fn visit_i16<E: serde::de::Error>(
                                            self,
                                            v: i16,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::I16(v)))
                                        }

                                        fn visit_i32<E: serde::de::Error>(
                                            self,
                                            v: i32,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::I32(v)))
                                        }

                                        fn visit_i64<E: serde::de::Error>(
                                            self,
                                            v: i64,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::I64(v)))
                                        }

                                        // BUG: serde does not yet support i128 here
                                        // fn visit_i128<E: serde::de::Error>(self, v: i128) -> Result<Self::Value, E> {
                                        //     Ok(Some(serde::__private::de::Content::I128(v)))
                                        // }

                                        fn visit_u8<E: serde::de::Error>(
                                            self,
                                            v: u8,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::U8(v)))
                                        }

                                        fn visit_u16<E: serde::de::Error>(
                                            self,
                                            v: u16,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::U16(v)))
                                        }

                                        fn visit_u32<E: serde::de::Error>(
                                            self,
                                            v: u32,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::U32(v)))
                                        }

                                        fn visit_u64<E: serde::de::Error>(
                                            self,
                                            v: u64,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::U64(v)))
                                        }

                                        // BUG: serde does not yet support u128 here
                                        // fn visit_u128<E: serde::de::Error>(self, v: u128) -> Result<Self::Value, E> {
                                        //     Ok(Some(serde::__private::de::Content::U128(v)))
                                        // }

                                        fn visit_f32<E: serde::de::Error>(
                                            self,
                                            v: f32,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::F32(v)))
                                        }

                                        fn visit_f64<E: serde::de::Error>(
                                            self,
                                            v: f64,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::F64(v)))
                                        }

                                        fn visit_char<E: serde::de::Error>(
                                            self,
                                            v: char,
                                        ) -> Result<Self::Value, E>
                                        {
                                            Ok(Some(serde::__private::de::Content::Char(v)))
                                        }

                                        fn visit_str<E: serde::de::Error>(
                                            self,
                                            v: &str,
                                        ) -> Result<Self::Value, E>
                                        {
                                            if matches!(self.field, Some(field) if v == field) {
                                                Ok(None)
                                            } else {
                                                Ok(Some(serde::__private::de::Content::String(
                                                    String::from(v),
                                                )))
                                            }
                                        }

                                        fn visit_borrowed_str<E: serde::de::Error>(
                                            self,
                                            v: &'de str,
                                        ) -> Result<Self::Value, E>
                                        {
                                            if matches!(self.field, Some(field) if v == field) {
                                                Ok(None)
                                            } else {
                                                Ok(Some(serde::__private::de::Content::Str(v)))
                                            }
                                        }

                                        fn visit_string<E: serde::de::Error>(
                                            self,
                                            v: String,
                                        ) -> Result<Self::Value, E>
                                        {
                                            if matches!(self.field, Some(field) if v == field) {
                                                Ok(None)
                                            } else {
                                                Ok(Some(serde::__private::de::Content::String(v)))
                                            }
                                        }

                                        fn visit_bytes<E: serde::de::Error>(
                                            self,
                                            v: &[u8],
                                        ) -> Result<Self::Value, E>
                                        {
                                            if matches!(self.field, Some(field) if v == field.as_bytes())
                                            {
                                                Ok(None)
                                            } else {
                                                Ok(Some(serde::__private::de::Content::ByteBuf(
                                                    Vec::from(v),
                                                )))
                                            }
                                        }

                                        fn visit_borrowed_bytes<E: serde::de::Error>(
                                            self,
                                            v: &'de [u8],
                                        ) -> Result<Self::Value, E>
                                        {
                                            if matches!(self.field, Some(field) if v == field.as_bytes())
                                            {
                                                Ok(None)
                                            } else {
                                                Ok(Some(serde::__private::de::Content::Bytes(v)))
                                            }
                                        }

                                        fn visit_byte_buf<E: serde::de::Error>(
                                            self,
                                            v: Vec<u8>,
                                        ) -> Result<Self::Value, E>
                                        {
                                            if matches!(self.field, Some(field) if v == field.as_bytes())
                                            {
                                                Ok(None)
                                            } else {
                                                Ok(Some(serde::__private::de::Content::ByteBuf(v)))
                                            }
                                        }

                                        fn visit_unit<E>(self) -> Result<Self::Value, E> {
                                            Ok(Some(serde::__private::de::Content::Unit))
                                        }
                                    }

                                    impl<'a, 'de> DeserializeSeed<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                                        type Value = Option<serde::__private::de::Content<'de>>;

                                        fn deserialize<D: Deserializer<'de>>(
                                            self,
                                            deserializer: D,
                                        ) -> Result<Self::Value, D::Error>
                                        {
                                            deserializer.deserialize_identifier(self)
                                        }
                                    }

                                    struct FlattenStructVariantVisitor<'a> {
                                        name: &'a str,
                                        variant: &'a str,
                                        fields: &'a [&'a str],
                                        tys: &'a [SerdeDataType<'a>],
                                        flatten: &'a [bool],
                                        values: &'a [SerdeDataValue<'a>],
                                    }

                                    impl<'a, 'de> Visitor<'de> for FlattenStructVariantVisitor<'a> {
                                        type Value = ();

                                        fn expecting(
                                            &self,
                                            formatter: &mut fmt::Formatter,
                                        ) -> fmt::Result {
                                            // ron's flattened struct canary depends on the expecting
                                            //  message to start with "struct "
                                            formatter.write_fmt(format_args!(
                                                "struct variant {}::{}",
                                                self.name, self.variant
                                            ))
                                        }

                                        fn visit_map<A: MapAccess<'de>>(
                                            self,
                                            mut map: A,
                                        ) -> Result<Self::Value, A::Error>
                                        {
                                            let mut collect = Vec::<
                                                Option<(
                                                    serde::__private::de::Content,
                                                    serde::__private::de::Content,
                                                )>,
                                            >::new(
                                            );

                                            for (((field, ty), flatten), expected) in self
                                                .fields
                                                .iter()
                                                .zip(self.tys.iter())
                                                .zip(self.flatten.iter())
                                                .zip(self.values.iter())
                                            {
                                                if !*flatten {
                                                    while let Some(Some(key)) = map.next_key_seed(
                                                        MaybeFlattenFieldIdentifierVisitor {
                                                            field: Some(field),
                                                        },
                                                    )? {
                                                        collect
                                                            .push(Some((key, map.next_value()?)));
                                                    }

                                                    map.next_value_seed(BorrowedTypedSerdeData {
                                                        ty,
                                                        value: expected,
                                                    })?;
                                                }
                                            }

                                            while let Some(Some(key)) = map.next_key_seed(
                                                MaybeFlattenFieldIdentifierVisitor { field: None },
                                            )? {
                                                collect.push(Some((key, map.next_value()?)));
                                            }

                                            for ((ty, flatten), expected) in self
                                                .tys
                                                .iter()
                                                .zip(self.flatten.iter())
                                                .zip(self.values.iter())
                                            {
                                                if *flatten {
                                                    BorrowedTypedSerdeData {
                                                        ty,
                                                        value: expected,
                                                    }
                                                    .deserialize(
                                                        serde::__private::de::FlatMapDeserializer(
                                                            &mut collect,
                                                            std::marker::PhantomData,
                                                        ),
                                                    )?;
                                                }
                                            }

                                            Ok(())
                                        }
                                    }

                                    impl<'a, 'de> DeserializeSeed<'de> for FlattenStructVariantVisitor<'a> {
                                        type Value = ();

                                        fn deserialize<D: Deserializer<'de>>(
                                            self,
                                            deserializer: D,
                                        ) -> Result<Self::Value, D::Error>
                                        {
                                            deserializer.deserialize_map(self)
                                        }
                                    }

                                    variant.newtype_variant_seed(FlattenStructVariantVisitor {
                                        name: self.name,
                                        variant: self.variant,
                                        fields: &fields.0,
                                        tys: &fields.1,
                                        flatten: &fields.2,
                                        values,
                                    })
                                } else {
                                    variant.struct_variant(
                                        unsafe { to_static_str_slice(&fields.0) },
                                        StructVariantVisitor {
                                            name: self.name,
                                            variant: self.variant,
                                            fields: &fields.0,
                                            tys: &fields.1,
                                            values,
                                        },
                                    )
                                }
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
                        SerdeDataVariantType::TaggedOther,
                        SerdeDataVariantValue::TaggedOther { .. },
                    ) => Err(serde::de::Error::custom(
                        "invalid serde enum data: tagged other in untagged",
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
                            name: &'a str,
                            variant: &'a str,
                            fields: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for TupleVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_fmt(format_args!(
                                    "tuple variant {}::{}",
                                    self.name, self.variant
                                ))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (i, (ty, expected)) in
                                    self.fields.iter().zip(self.values.iter()).enumerate()
                                {
                                    let Some(()) =
                                        seq.next_element_seed(BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        })?
                                    else {
                                        return Err(serde::de::Error::invalid_length(
                                            i,
                                            &format!(
                                                "tuple variant {}::{} with {} elements",
                                                self.name,
                                                self.variant,
                                                self.values.len()
                                            )
                                            .as_str(),
                                        ));
                                    };
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
                                name,
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
                            type Value = Option<()>;

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("a field identifier")
                            }

                            fn visit_u64<E: serde::de::Error>(
                                self,
                                v: u64,
                            ) -> Result<Self::Value, E> {
                                if v == self.index {
                                    Ok(Some(()))
                                } else {
                                    Ok(None)
                                }
                            }

                            fn visit_str<E: serde::de::Error>(
                                self,
                                v: &str,
                            ) -> Result<Self::Value, E> {
                                if v == self.field {
                                    Ok(Some(()))
                                } else {
                                    Ok(None)
                                }
                            }

                            fn visit_bytes<E: serde::de::Error>(
                                self,
                                v: &[u8],
                            ) -> Result<Self::Value, E> {
                                if v == self.field.as_bytes() {
                                    Ok(Some(()))
                                } else {
                                    Ok(None)
                                }
                            }
                        }

                        impl<'a, 'de> DeserializeSeed<'de> for FieldIdentifierVisitor<'a> {
                            type Value = Option<()>;

                            fn deserialize<D: Deserializer<'de>>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error> {
                                deserializer.deserialize_identifier(self)
                            }
                        }

                        struct StructVariantVisitor<'a> {
                            name: &'a str,
                            variant: &'a str,
                            fields: &'a [&'a str],
                            tys: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for StructVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_fmt(format_args!(
                                    "struct variant {}::{}",
                                    self.name, self.variant
                                ))
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
                                    // untagged struct variants inside a flattened struct
                                    //  must sort through other keys as well, *sigh*
                                    loop {
                                        match map.next_key_seed(FieldIdentifierVisitor {
                                            field,
                                            index,
                                        })? {
                                            Some(Some(())) => {
                                                break map.next_value_seed(
                                                    BorrowedTypedSerdeData {
                                                        ty,
                                                        value: expected,
                                                    },
                                                )?
                                            }
                                            Some(None) => map
                                                .next_value::<serde::de::IgnoredAny>()
                                                .map(|_| ())?,
                                            None => {
                                                return Err(serde::de::Error::missing_field(
                                                    unsafe { to_static_str(field) },
                                                ))
                                            }
                                        }
                                    }
                                }
                                // untagged struct variants inside a flattened struct must
                                //  consume all remaining other keys as well, *sigh*
                                while map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                                    map.next_value::<serde::de::IgnoredAny>().map(|_| ())?;
                                }
                                Ok(())
                            }
                        }

                        if values.len() != fields.0.len()
                            || values.len() != fields.1.len()
                            || values.len() != fields.2.len()
                        {
                            return Err(serde::de::Error::custom("mismatch struct fields len"));
                        }

                        if fields.2.iter().any(|x| *x) {
                            struct MaybeFlattenFieldIdentifierVisitor<'a> {
                                field: Option<&'a str>,
                            }

                            impl<'a, 'de> Visitor<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                                type Value = Option<serde::__private::de::Content<'de>>;

                                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                    formatter.write_str("a field identifier")
                                }

                                fn visit_bool<E: serde::de::Error>(
                                    self,
                                    v: bool,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::Bool(v)))
                                }

                                fn visit_i8<E: serde::de::Error>(
                                    self,
                                    v: i8,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::I8(v)))
                                }

                                fn visit_i16<E: serde::de::Error>(
                                    self,
                                    v: i16,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::I16(v)))
                                }

                                fn visit_i32<E: serde::de::Error>(
                                    self,
                                    v: i32,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::I32(v)))
                                }

                                fn visit_i64<E: serde::de::Error>(
                                    self,
                                    v: i64,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::I64(v)))
                                }

                                // BUG: serde does not yet support i128 here
                                // fn visit_i128<E: serde::de::Error>(self, v: i128) -> Result<Self::Value, E> {
                                //     Ok(Some(serde::__private::de::Content::I128(v)))
                                // }

                                fn visit_u8<E: serde::de::Error>(
                                    self,
                                    v: u8,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::U8(v)))
                                }

                                fn visit_u16<E: serde::de::Error>(
                                    self,
                                    v: u16,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::U16(v)))
                                }

                                fn visit_u32<E: serde::de::Error>(
                                    self,
                                    v: u32,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::U32(v)))
                                }

                                fn visit_u64<E: serde::de::Error>(
                                    self,
                                    v: u64,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::U64(v)))
                                }

                                // BUG: serde does not yet support u128 here
                                // fn visit_u128<E: serde::de::Error>(self, v: u128) -> Result<Self::Value, E> {
                                //     Ok(Some(serde::__private::de::Content::U128(v)))
                                // }

                                fn visit_f32<E: serde::de::Error>(
                                    self,
                                    v: f32,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::F32(v)))
                                }

                                fn visit_f64<E: serde::de::Error>(
                                    self,
                                    v: f64,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::F64(v)))
                                }

                                fn visit_char<E: serde::de::Error>(
                                    self,
                                    v: char,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::Char(v)))
                                }

                                fn visit_str<E: serde::de::Error>(
                                    self,
                                    v: &str,
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::String(
                                            String::from(v),
                                        )))
                                    }
                                }

                                fn visit_borrowed_str<E: serde::de::Error>(
                                    self,
                                    v: &'de str,
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::Str(v)))
                                    }
                                }

                                fn visit_string<E: serde::de::Error>(
                                    self,
                                    v: String,
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::String(v)))
                                    }
                                }

                                fn visit_bytes<E: serde::de::Error>(
                                    self,
                                    v: &[u8],
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::ByteBuf(Vec::from(
                                            v,
                                        ))))
                                    }
                                }

                                fn visit_borrowed_bytes<E: serde::de::Error>(
                                    self,
                                    v: &'de [u8],
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::Bytes(v)))
                                    }
                                }

                                fn visit_byte_buf<E: serde::de::Error>(
                                    self,
                                    v: Vec<u8>,
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::ByteBuf(v)))
                                    }
                                }

                                fn visit_unit<E>(self) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::Unit))
                                }
                            }

                            impl<'a, 'de> DeserializeSeed<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                                type Value = Option<serde::__private::de::Content<'de>>;

                                fn deserialize<D: Deserializer<'de>>(
                                    self,
                                    deserializer: D,
                                ) -> Result<Self::Value, D::Error> {
                                    deserializer.deserialize_identifier(self)
                                }
                            }

                            struct FlattenStructVariantVisitor<'a> {
                                name: &'a str,
                                variant: &'a str,
                                fields: &'a [&'a str],
                                tys: &'a [SerdeDataType<'a>],
                                flatten: &'a [bool],
                                values: &'a [SerdeDataValue<'a>],
                            }

                            impl<'a, 'de> Visitor<'de> for FlattenStructVariantVisitor<'a> {
                                type Value = ();

                                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                    // ron's flattened struct canary depends on the expecting
                                    //  message to start with "struct "
                                    formatter.write_fmt(format_args!(
                                        "struct variant {}::{}",
                                        self.name, self.variant
                                    ))
                                }

                                fn visit_map<A: MapAccess<'de>>(
                                    self,
                                    mut map: A,
                                ) -> Result<Self::Value, A::Error> {
                                    let mut collect = Vec::<
                                        Option<(
                                            serde::__private::de::Content,
                                            serde::__private::de::Content,
                                        )>,
                                    >::new();

                                    for (((field, ty), flatten), expected) in self
                                        .fields
                                        .iter()
                                        .zip(self.tys.iter())
                                        .zip(self.flatten.iter())
                                        .zip(self.values.iter())
                                    {
                                        if !*flatten {
                                            while let Some(Some(key)) = map.next_key_seed(
                                                MaybeFlattenFieldIdentifierVisitor {
                                                    field: Some(field),
                                                },
                                            )? {
                                                collect.push(Some((key, map.next_value()?)));
                                            }

                                            map.next_value_seed(BorrowedTypedSerdeData {
                                                ty,
                                                value: expected,
                                            })?;
                                        }
                                    }

                                    while let Some(Some(key)) =
                                        map.next_key_seed(MaybeFlattenFieldIdentifierVisitor {
                                            field: None,
                                        })?
                                    {
                                        collect.push(Some((key, map.next_value()?)));
                                    }

                                    for ((ty, flatten), expected) in self
                                        .tys
                                        .iter()
                                        .zip(self.flatten.iter())
                                        .zip(self.values.iter())
                                    {
                                        if *flatten {
                                            BorrowedTypedSerdeData {
                                                ty,
                                                value: expected,
                                            }
                                            .deserialize(
                                                serde::__private::de::FlatMapDeserializer(
                                                    &mut collect,
                                                    std::marker::PhantomData,
                                                ),
                                            )?;
                                        }
                                    }

                                    Ok(())
                                }
                            }

                            deserializer.deserialize_any(FlattenStructVariantVisitor {
                                name,
                                variant,
                                fields: &fields.0,
                                tys: &fields.1,
                                flatten: &fields.2,
                                values,
                            })
                        } else {
                            deserializer.deserialize_struct(
                                unsafe { to_static_str(name) },
                                unsafe { to_static_str_slice(&fields.0) },
                                StructVariantVisitor {
                                    name,
                                    variant,
                                    fields: &fields.0,
                                    tys: &fields.1,
                                    values,
                                },
                            )
                        }
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
                                "expected field identifier {:?} found {:?}",
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
                                format!("expected variant index {variant_index} found {v}"),
                            )),
                            DelayedVariantIdentifier::Str(ref v) if v == variant => Ok(()),
                            DelayedVariantIdentifier::Str(ref v) => Err(serde::de::Error::custom(
                                format!("expected variant identifier {variant} found {v}"),
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
                            name: &'a str,
                            tag: &'a str,
                            variant: &'a str,
                            variant_index: u32,
                            content: &'a str,
                        }

                        impl<'a, 'de> Visitor<'de> for AdjacentlyTaggedUnitVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_fmt(format_args!(
                                    "unit variant {}::{}",
                                    self.name, self.variant
                                ))
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
                                let Some(serde::__private::de::TagOrContentField::Tag) = map
                                    .next_key_seed(
                                        serde::__private::de::TagOrContentFieldVisitor {
                                            tag: unsafe { to_static_str(self.tag) },
                                            content: unsafe { to_static_str(self.content) },
                                        },
                                    )?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.variant, self.variant_index)
                            }
                        }

                        deserializer.deserialize_struct(
                            unsafe { to_static_str(name) },
                            unsafe { to_static_str_slice(std::slice::from_ref(tag)) },
                            AdjacentlyTaggedUnitVariantVisitor {
                                name,
                                tag,
                                variant,
                                variant_index: *variant_index,
                                content,
                            },
                        )
                    }
                    (
                        SerdeDataVariantType::TaggedOther,
                        SerdeDataVariantValue::TaggedOther {
                            variant: other_variant,
                            index: other_variant_index,
                        },
                    ) => {
                        struct AdjacentlyTaggedOtherVariantVisitor<'a> {
                            name: &'a str,
                            tag: &'a str,
                            variant: &'a str,
                            other_variant: &'a str,
                            other_variant_index: u32,
                            content: &'a str,
                        }

                        impl<'a, 'de> Visitor<'de> for AdjacentlyTaggedOtherVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_fmt(format_args!(
                                    "unit variant {}::{}",
                                    self.name, self.variant
                                ))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                if let Some(tag) = seq.next_element::<DelayedVariantIdentifier>()? {
                                    tag.check_variant(self.other_variant, self.other_variant_index)
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
                                let Some(serde::__private::de::TagOrContentField::Tag) = map
                                    .next_key_seed(
                                        serde::__private::de::TagOrContentFieldVisitor {
                                            tag: unsafe { to_static_str(self.tag) },
                                            content: unsafe { to_static_str(self.content) },
                                        },
                                    )?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.other_variant, self.other_variant_index)
                            }
                        }

                        deserializer.deserialize_struct(
                            unsafe { to_static_str(name) },
                            unsafe { to_static_str_slice(std::slice::from_ref(tag)) },
                            AdjacentlyTaggedOtherVariantVisitor {
                                name,
                                tag,
                                variant,
                                other_variant,
                                other_variant_index: *other_variant_index,
                                content,
                            },
                        )
                    }
                    (
                        SerdeDataVariantType::Newtype { inner: ty },
                        SerdeDataVariantValue::Newtype { inner: value },
                    ) => {
                        struct AdjacentlyTaggedNewtypeVariantVisitor<'a> {
                            name: &'a str,
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
                                formatter.write_fmt(format_args!(
                                    "newtype variant {}::{}",
                                    self.name, self.variant
                                ))
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
                                })?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.content)
                                    }));
                                };
                                Ok(())
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                let Some(serde::__private::de::TagOrContentField::Tag) = map
                                    .next_key_seed(
                                        serde::__private::de::TagOrContentFieldVisitor {
                                            tag: unsafe { to_static_str(self.tag) },
                                            content: unsafe { to_static_str(self.content) },
                                        },
                                    )?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.variant, self.variant_index)?;
                                let Some(serde::__private::de::TagOrContentField::Content) = map
                                    .next_key_seed(
                                        serde::__private::de::TagOrContentFieldVisitor {
                                            tag: unsafe { to_static_str(self.tag) },
                                            content: unsafe { to_static_str(self.content) },
                                        },
                                    )?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
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
                                name,
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
                            name: &'a str,
                            variant: &'a str,
                            fields: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for TupleVariantSeed<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_fmt(format_args!(
                                    "tuple variant {}::{}",
                                    self.name, self.variant
                                ))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (i, (ty, expected)) in
                                    self.fields.iter().zip(self.values.iter()).enumerate()
                                {
                                    let Some(()) =
                                        seq.next_element_seed(BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        })?
                                    else {
                                        return Err(serde::de::Error::invalid_length(
                                            i,
                                            &format!(
                                                "tuple variant {}::{} with {} elements",
                                                self.name,
                                                self.variant,
                                                self.values.len()
                                            )
                                            .as_str(),
                                        ));
                                    };
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
                            name: &'a str,
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
                                formatter.write_fmt(format_args!(
                                    "tuple variant {}::{}",
                                    self.name, self.variant
                                ))
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
                                let Some(()) = seq.next_element_seed(TupleVariantSeed {
                                    name: self.name,
                                    variant: self.variant,
                                    fields: self.fields,
                                    values: self.values,
                                })?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.content)
                                    }));
                                };
                                Ok(())
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                let Some(serde::__private::de::TagOrContentField::Tag) = map
                                    .next_key_seed(
                                        serde::__private::de::TagOrContentFieldVisitor {
                                            tag: unsafe { to_static_str(self.tag) },
                                            content: unsafe { to_static_str(self.content) },
                                        },
                                    )?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.variant, self.variant_index)?;
                                let Some(serde::__private::de::TagOrContentField::Content) = map
                                    .next_key_seed(
                                        serde::__private::de::TagOrContentFieldVisitor {
                                            tag: unsafe { to_static_str(self.tag) },
                                            content: unsafe { to_static_str(self.content) },
                                        },
                                    )?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                };
                                map.next_value_seed(TupleVariantSeed {
                                    name: self.name,
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
                                name,
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
                                formatter.write_fmt(format_args!(
                                    "struct variant {}::{}",
                                    self.name, self.variant
                                ))
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (i, (((index, field), ty), expected)) in (0..)
                                    .zip(self.fields.iter())
                                    .zip(self.tys.iter())
                                    .zip(self.values.iter())
                                    .enumerate()
                                {
                                    let Some(((), ())) = map.next_entry_seed(
                                        FieldIdentifierVisitor { field, index },
                                        BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        },
                                    )?
                                    else {
                                        return Err(serde::de::Error::invalid_length(
                                            i,
                                            &format!(
                                                "struct variant {}::{} with {} elements",
                                                self.name,
                                                self.variant,
                                                self.values.len()
                                            )
                                            .as_str(),
                                        ));
                                    };
                                }
                                // flattened struct variants are incompatible with strict fields
                                while map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                                    map.next_value::<serde::de::IgnoredAny>().map(|_| ())?;
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

                        struct MaybeFlattenFieldIdentifierVisitor<'a> {
                            field: Option<&'a str>,
                        }

                        impl<'a, 'de> Visitor<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                            type Value = Option<serde::__private::de::Content<'de>>;

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("a field identifier")
                            }

                            fn visit_bool<E: serde::de::Error>(
                                self,
                                v: bool,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::Bool(v)))
                            }

                            fn visit_i8<E: serde::de::Error>(
                                self,
                                v: i8,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::I8(v)))
                            }

                            fn visit_i16<E: serde::de::Error>(
                                self,
                                v: i16,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::I16(v)))
                            }

                            fn visit_i32<E: serde::de::Error>(
                                self,
                                v: i32,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::I32(v)))
                            }

                            fn visit_i64<E: serde::de::Error>(
                                self,
                                v: i64,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::I64(v)))
                            }

                            // BUG: serde does not yet support i128 here
                            // fn visit_i128<E: serde::de::Error>(self, v: i128) -> Result<Self::Value, E> {
                            //     Ok(Some(serde::__private::de::Content::I128(v)))
                            // }

                            fn visit_u8<E: serde::de::Error>(
                                self,
                                v: u8,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::U8(v)))
                            }

                            fn visit_u16<E: serde::de::Error>(
                                self,
                                v: u16,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::U16(v)))
                            }

                            fn visit_u32<E: serde::de::Error>(
                                self,
                                v: u32,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::U32(v)))
                            }

                            fn visit_u64<E: serde::de::Error>(
                                self,
                                v: u64,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::U64(v)))
                            }

                            // BUG: serde does not yet support u128 here
                            // fn visit_u128<E: serde::de::Error>(self, v: u128) -> Result<Self::Value, E> {
                            //     Ok(Some(serde::__private::de::Content::U128(v)))
                            // }

                            fn visit_f32<E: serde::de::Error>(
                                self,
                                v: f32,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::F32(v)))
                            }

                            fn visit_f64<E: serde::de::Error>(
                                self,
                                v: f64,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::F64(v)))
                            }

                            fn visit_char<E: serde::de::Error>(
                                self,
                                v: char,
                            ) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::Char(v)))
                            }

                            fn visit_str<E: serde::de::Error>(
                                self,
                                v: &str,
                            ) -> Result<Self::Value, E> {
                                if matches!(self.field, Some(field) if v == field) {
                                    Ok(None)
                                } else {
                                    Ok(Some(serde::__private::de::Content::String(String::from(v))))
                                }
                            }

                            fn visit_borrowed_str<E: serde::de::Error>(
                                self,
                                v: &'de str,
                            ) -> Result<Self::Value, E> {
                                if matches!(self.field, Some(field) if v == field) {
                                    Ok(None)
                                } else {
                                    Ok(Some(serde::__private::de::Content::Str(v)))
                                }
                            }

                            fn visit_string<E: serde::de::Error>(
                                self,
                                v: String,
                            ) -> Result<Self::Value, E> {
                                if matches!(self.field, Some(field) if v == field) {
                                    Ok(None)
                                } else {
                                    Ok(Some(serde::__private::de::Content::String(v)))
                                }
                            }

                            fn visit_bytes<E: serde::de::Error>(
                                self,
                                v: &[u8],
                            ) -> Result<Self::Value, E> {
                                if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                    Ok(None)
                                } else {
                                    Ok(Some(serde::__private::de::Content::ByteBuf(Vec::from(v))))
                                }
                            }

                            fn visit_borrowed_bytes<E: serde::de::Error>(
                                self,
                                v: &'de [u8],
                            ) -> Result<Self::Value, E> {
                                if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                    Ok(None)
                                } else {
                                    Ok(Some(serde::__private::de::Content::Bytes(v)))
                                }
                            }

                            fn visit_byte_buf<E: serde::de::Error>(
                                self,
                                v: Vec<u8>,
                            ) -> Result<Self::Value, E> {
                                if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                    Ok(None)
                                } else {
                                    Ok(Some(serde::__private::de::Content::ByteBuf(v)))
                                }
                            }

                            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                                Ok(Some(serde::__private::de::Content::Unit))
                            }
                        }

                        impl<'a, 'de> DeserializeSeed<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                            type Value = Option<serde::__private::de::Content<'de>>;

                            fn deserialize<D: Deserializer<'de>>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error> {
                                deserializer.deserialize_identifier(self)
                            }
                        }

                        struct FlattenStructVariantSeed<'a> {
                            name: &'a str,
                            variant: &'a str,
                            fields: &'a [&'a str],
                            tys: &'a [SerdeDataType<'a>],
                            flatten: &'a [bool],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for FlattenStructVariantSeed<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                // ron's flattened struct canary depends on the expecting
                                //  message to start with "struct "
                                formatter.write_fmt(format_args!(
                                    "struct variant {}::{}",
                                    self.name, self.variant
                                ))
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                let mut collect = Vec::<
                                    Option<(
                                        serde::__private::de::Content,
                                        serde::__private::de::Content,
                                    )>,
                                >::new();

                                for (((field, ty), flatten), expected) in self
                                    .fields
                                    .iter()
                                    .zip(self.tys.iter())
                                    .zip(self.flatten.iter())
                                    .zip(self.values.iter())
                                {
                                    if !*flatten {
                                        while let Some(Some(key)) =
                                            map.next_key_seed(MaybeFlattenFieldIdentifierVisitor {
                                                field: Some(field),
                                            })?
                                        {
                                            collect.push(Some((key, map.next_value()?)));
                                        }

                                        map.next_value_seed(BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        })?;
                                    }
                                }

                                while let Some(Some(key)) =
                                    map.next_key_seed(MaybeFlattenFieldIdentifierVisitor {
                                        field: None,
                                    })?
                                {
                                    collect.push(Some((key, map.next_value()?)));
                                }

                                for ((ty, flatten), expected) in self
                                    .tys
                                    .iter()
                                    .zip(self.flatten.iter())
                                    .zip(self.values.iter())
                                {
                                    if *flatten {
                                        BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        }
                                        .deserialize(
                                            serde::__private::de::FlatMapDeserializer(
                                                &mut collect,
                                                std::marker::PhantomData,
                                            ),
                                        )?;
                                    }
                                }

                                Ok(())
                            }
                        }

                        impl<'a, 'de> DeserializeSeed<'de> for FlattenStructVariantSeed<'a> {
                            type Value = ();

                            fn deserialize<D: Deserializer<'de>>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error> {
                                deserializer.deserialize_any(self)
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
                            flatten: &'a [bool],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for AdjacentlyTaggedStructVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_fmt(format_args!(
                                    "struct variant {}::{}",
                                    self.name, self.variant
                                ))
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

                                #[allow(clippy::redundant_else)]
                                if self.flatten.iter().any(|x| *x) {
                                    let Some(()) =
                                        seq.next_element_seed(FlattenStructVariantSeed {
                                            name: self.name,
                                            variant: self.variant,
                                            fields: self.fields,
                                            tys: self.tys,
                                            flatten: self.flatten,
                                            values: self.values,
                                        })?
                                    else {
                                        return Err(serde::de::Error::missing_field(unsafe {
                                            to_static_str(self.content)
                                        }));
                                    };
                                } else {
                                    let Some(()) = seq.next_element_seed(StructVariantSeed {
                                        name: self.name,
                                        variant: self.variant,
                                        fields: self.fields,
                                        tys: self.tys,
                                        values: self.values,
                                    })?
                                    else {
                                        return Err(serde::de::Error::missing_field(unsafe {
                                            to_static_str(self.content)
                                        }));
                                    };
                                }

                                Ok(())
                            }

                            fn visit_map<A: MapAccess<'de>>(
                                self,
                                mut map: A,
                            ) -> Result<Self::Value, A::Error> {
                                let Some(serde::__private::de::TagOrContentField::Tag) = map
                                    .next_key_seed(
                                        serde::__private::de::TagOrContentFieldVisitor {
                                            tag: unsafe { to_static_str(self.tag) },
                                            content: unsafe { to_static_str(self.content) },
                                        },
                                    )?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                };
                                map.next_value::<DelayedVariantIdentifier>()?
                                    .check_variant(self.variant, self.variant_index)?;
                                let Some(serde::__private::de::TagOrContentField::Content) = map
                                    .next_key_seed(
                                        serde::__private::de::TagOrContentFieldVisitor {
                                            tag: unsafe { to_static_str(self.tag) },
                                            content: unsafe { to_static_str(self.content) },
                                        },
                                    )?
                                else {
                                    return Err(serde::de::Error::missing_field(unsafe {
                                        to_static_str(self.tag)
                                    }));
                                };
                                if self.flatten.iter().any(|x| *x) {
                                    map.next_value_seed(FlattenStructVariantSeed {
                                        name: self.name,
                                        variant: self.variant,
                                        fields: self.fields,
                                        tys: self.tys,
                                        flatten: self.flatten,
                                        values: self.values,
                                    })?;
                                } else {
                                    map.next_value_seed(StructVariantSeed {
                                        name: self.name,
                                        variant: self.variant,
                                        fields: self.fields,
                                        tys: self.tys,
                                        values: self.values,
                                    })?;
                                }
                                Ok(())
                            }
                        }

                        if values.len() != fields.0.len()
                            || values.len() != fields.1.len()
                            || values.len() != fields.2.len()
                        {
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
                                flatten: &fields.2,
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
                        format!("unit variant {name}::{variant}")
                    }
                    (
                        SerdeDataVariantType::TaggedOther,
                        SerdeDataVariantValue::TaggedOther {
                            variant: _,
                            index: _,
                        },
                    ) => {
                        format!("unit variant {name}::{variant}")
                    }
                    (
                        SerdeDataVariantType::Newtype { .. },
                        SerdeDataVariantValue::Newtype { .. },
                    ) => format!("newtype variant {name}::{variant}"),
                    (SerdeDataVariantType::Tuple { .. }, SerdeDataVariantValue::Struct { .. }) => {
                        return Err(serde::de::Error::custom(
                            "invalid serde internally tagged tuple variant",
                        ))
                    }
                    (SerdeDataVariantType::Struct { .. }, SerdeDataVariantValue::Struct { .. }) => {
                        format!("struct variant {name}::{variant}")
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
                                format!("expected variant index {variant_index} found {v}"),
                            )),
                            DelayedVariantIdentifier::Str(ref v) if v == variant => Ok(()),
                            DelayedVariantIdentifier::Str(ref v) => Err(serde::de::Error::custom(
                                format!("expected variant identifier {variant} found {v}"),
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

                match (ty, value) {
                    (
                        SerdeDataVariantType::TaggedOther,
                        SerdeDataVariantValue::TaggedOther {
                            variant: other_variant,
                            index: other_variant_index,
                        },
                    ) => tag.check_variant(other_variant, *other_variant_index),
                    _ => tag.check_variant(variant, *variant_index),
                }?;

                let deserializer =
                    serde::__private::de::ContentDeserializer::<D::Error>::new(content);

                match (ty, value) {
                    (SerdeDataVariantType::Unit, SerdeDataVariantValue::Unit) => deserializer
                        .deserialize_any(serde::__private::de::InternallyTaggedUnitVisitor::new(
                            name, variant,
                        )),
                    (
                        SerdeDataVariantType::TaggedOther,
                        SerdeDataVariantValue::TaggedOther {
                            variant: other_variant,
                            index: _,
                        },
                    ) => deserializer.deserialize_any(
                        serde::__private::de::InternallyTaggedUnitVisitor::new(name, other_variant),
                    ),
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
                            type Value = Option<()>;

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("a field identifier")
                            }

                            fn visit_u64<E: serde::de::Error>(
                                self,
                                v: u64,
                            ) -> Result<Self::Value, E> {
                                if v == self.index {
                                    Ok(Some(()))
                                } else {
                                    Ok(None)
                                }
                            }

                            fn visit_str<E: serde::de::Error>(
                                self,
                                v: &str,
                            ) -> Result<Self::Value, E> {
                                if v == self.field {
                                    Ok(Some(()))
                                } else {
                                    Ok(None)
                                }
                            }

                            fn visit_bytes<E: serde::de::Error>(
                                self,
                                v: &[u8],
                            ) -> Result<Self::Value, E> {
                                if v == self.field.as_bytes() {
                                    Ok(Some(()))
                                } else {
                                    Ok(None)
                                }
                            }
                        }

                        impl<'a, 'de> DeserializeSeed<'de> for FieldIdentifierVisitor<'a> {
                            type Value = Option<()>;

                            fn deserialize<D: Deserializer<'de>>(
                                self,
                                deserializer: D,
                            ) -> Result<Self::Value, D::Error> {
                                deserializer.deserialize_identifier(self)
                            }
                        }

                        struct StructVariantVisitor<'a> {
                            name: &'a str,
                            variant: &'a str,
                            fields: &'a [&'a str],
                            tys: &'a [SerdeDataType<'a>],
                            values: &'a [SerdeDataValue<'a>],
                        }

                        impl<'a, 'de> Visitor<'de> for StructVariantVisitor<'a> {
                            type Value = ();

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_fmt(format_args!(
                                    "struct variant {}::{}",
                                    self.name, self.variant
                                ))
                            }

                            fn visit_seq<A: SeqAccess<'de>>(
                                self,
                                mut seq: A,
                            ) -> Result<Self::Value, A::Error> {
                                for (i, (ty, expected)) in
                                    self.tys.iter().zip(self.values.iter()).enumerate()
                                {
                                    let Some(()) =
                                        seq.next_element_seed(BorrowedTypedSerdeData {
                                            ty,
                                            value: expected,
                                        })?
                                    else {
                                        return Err(serde::de::Error::invalid_length(
                                            i,
                                            &format!(
                                                "struct variant {}::{} with {} elements",
                                                self.name,
                                                self.variant,
                                                self.values.len()
                                            )
                                            .as_str(),
                                        ));
                                    };
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
                                    // internally tagged struct variants inside a flattened
                                    //  struct must sort through other keys as well, *sigh*
                                    loop {
                                        match map.next_key_seed(FieldIdentifierVisitor {
                                            field,
                                            index,
                                        })? {
                                            Some(Some(())) => {
                                                break map.next_value_seed(
                                                    BorrowedTypedSerdeData {
                                                        ty,
                                                        value: expected,
                                                    },
                                                )?
                                            }
                                            Some(None) => map
                                                .next_value::<serde::de::IgnoredAny>()
                                                .map(|_| ())?,
                                            None => {
                                                return Err(serde::de::Error::missing_field(
                                                    unsafe { to_static_str(field) },
                                                ))
                                            }
                                        }
                                    }
                                }
                                // internally tagged struct variants inside a flattened struct
                                //  must consume all remaining other keys as well, *sigh*
                                while map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                                    map.next_value::<serde::de::IgnoredAny>().map(|_| ())?;
                                }
                                Ok(())
                            }
                        }

                        if values.len() != fields.0.len()
                            || values.len() != fields.1.len()
                            || values.len() != fields.2.len()
                        {
                            return Err(serde::de::Error::custom("mismatch struct fields len"));
                        }

                        if fields.2.iter().any(|x| *x) {
                            struct MaybeFlattenFieldIdentifierVisitor<'a> {
                                field: Option<&'a str>,
                            }

                            impl<'a, 'de> Visitor<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                                type Value = Option<serde::__private::de::Content<'de>>;

                                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                    formatter.write_str("a field identifier")
                                }

                                fn visit_bool<E: serde::de::Error>(
                                    self,
                                    v: bool,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::Bool(v)))
                                }

                                fn visit_i8<E: serde::de::Error>(
                                    self,
                                    v: i8,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::I8(v)))
                                }

                                fn visit_i16<E: serde::de::Error>(
                                    self,
                                    v: i16,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::I16(v)))
                                }

                                fn visit_i32<E: serde::de::Error>(
                                    self,
                                    v: i32,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::I32(v)))
                                }

                                fn visit_i64<E: serde::de::Error>(
                                    self,
                                    v: i64,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::I64(v)))
                                }

                                // BUG: serde does not yet support i128 here
                                // fn visit_i128<E: serde::de::Error>(self, v: i128) -> Result<Self::Value, E> {
                                //     Ok(Some(serde::__private::de::Content::I128(v)))
                                // }

                                fn visit_u8<E: serde::de::Error>(
                                    self,
                                    v: u8,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::U8(v)))
                                }

                                fn visit_u16<E: serde::de::Error>(
                                    self,
                                    v: u16,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::U16(v)))
                                }

                                fn visit_u32<E: serde::de::Error>(
                                    self,
                                    v: u32,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::U32(v)))
                                }

                                fn visit_u64<E: serde::de::Error>(
                                    self,
                                    v: u64,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::U64(v)))
                                }

                                // BUG: serde does not yet support u128 here
                                // fn visit_u128<E: serde::de::Error>(self, v: u128) -> Result<Self::Value, E> {
                                //     Ok(Some(serde::__private::de::Content::U128(v)))
                                // }

                                fn visit_f32<E: serde::de::Error>(
                                    self,
                                    v: f32,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::F32(v)))
                                }

                                fn visit_f64<E: serde::de::Error>(
                                    self,
                                    v: f64,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::F64(v)))
                                }

                                fn visit_char<E: serde::de::Error>(
                                    self,
                                    v: char,
                                ) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::Char(v)))
                                }

                                fn visit_str<E: serde::de::Error>(
                                    self,
                                    v: &str,
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::String(
                                            String::from(v),
                                        )))
                                    }
                                }

                                fn visit_borrowed_str<E: serde::de::Error>(
                                    self,
                                    v: &'de str,
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::Str(v)))
                                    }
                                }

                                fn visit_string<E: serde::de::Error>(
                                    self,
                                    v: String,
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::String(v)))
                                    }
                                }

                                fn visit_bytes<E: serde::de::Error>(
                                    self,
                                    v: &[u8],
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::ByteBuf(Vec::from(
                                            v,
                                        ))))
                                    }
                                }

                                fn visit_borrowed_bytes<E: serde::de::Error>(
                                    self,
                                    v: &'de [u8],
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::Bytes(v)))
                                    }
                                }

                                fn visit_byte_buf<E: serde::de::Error>(
                                    self,
                                    v: Vec<u8>,
                                ) -> Result<Self::Value, E> {
                                    if matches!(self.field, Some(field) if v == field.as_bytes()) {
                                        Ok(None)
                                    } else {
                                        Ok(Some(serde::__private::de::Content::ByteBuf(v)))
                                    }
                                }

                                fn visit_unit<E>(self) -> Result<Self::Value, E> {
                                    Ok(Some(serde::__private::de::Content::Unit))
                                }
                            }

                            impl<'a, 'de> DeserializeSeed<'de> for MaybeFlattenFieldIdentifierVisitor<'a> {
                                type Value = Option<serde::__private::de::Content<'de>>;

                                fn deserialize<D: Deserializer<'de>>(
                                    self,
                                    deserializer: D,
                                ) -> Result<Self::Value, D::Error> {
                                    deserializer.deserialize_identifier(self)
                                }
                            }

                            struct FlattenStructVariantVisitor<'a> {
                                name: &'a str,
                                variant: &'a str,
                                fields: &'a [&'a str],
                                tys: &'a [SerdeDataType<'a>],
                                flatten: &'a [bool],
                                values: &'a [SerdeDataValue<'a>],
                            }

                            impl<'a, 'de> Visitor<'de> for FlattenStructVariantVisitor<'a> {
                                type Value = ();

                                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                    // ron's flattened struct canary depends on the expecting
                                    //  message to start with "struct "
                                    formatter.write_fmt(format_args!(
                                        "struct variant {}::{}",
                                        self.name, self.variant
                                    ))
                                }

                                fn visit_map<A: MapAccess<'de>>(
                                    self,
                                    mut map: A,
                                ) -> Result<Self::Value, A::Error> {
                                    let mut collect = Vec::<
                                        Option<(
                                            serde::__private::de::Content,
                                            serde::__private::de::Content,
                                        )>,
                                    >::new();

                                    for (((field, ty), flatten), expected) in self
                                        .fields
                                        .iter()
                                        .zip(self.tys.iter())
                                        .zip(self.flatten.iter())
                                        .zip(self.values.iter())
                                    {
                                        if !*flatten {
                                            while let Some(Some(key)) = map.next_key_seed(
                                                MaybeFlattenFieldIdentifierVisitor {
                                                    field: Some(field),
                                                },
                                            )? {
                                                collect.push(Some((key, map.next_value()?)));
                                            }

                                            map.next_value_seed(BorrowedTypedSerdeData {
                                                ty,
                                                value: expected,
                                            })?;
                                        }
                                    }

                                    while let Some(Some(key)) =
                                        map.next_key_seed(MaybeFlattenFieldIdentifierVisitor {
                                            field: None,
                                        })?
                                    {
                                        collect.push(Some((key, map.next_value()?)));
                                    }

                                    for ((ty, flatten), expected) in self
                                        .tys
                                        .iter()
                                        .zip(self.flatten.iter())
                                        .zip(self.values.iter())
                                    {
                                        if *flatten {
                                            BorrowedTypedSerdeData {
                                                ty,
                                                value: expected,
                                            }
                                            .deserialize(
                                                serde::__private::de::FlatMapDeserializer(
                                                    &mut collect,
                                                    std::marker::PhantomData,
                                                ),
                                            )?;
                                        }
                                    }

                                    Ok(())
                                }
                            }

                            deserializer.deserialize_any(FlattenStructVariantVisitor {
                                name,
                                variant,
                                fields: &fields.0,
                                tys: &fields.1,
                                flatten: &fields.2,
                                values,
                            })
                        } else {
                            deserializer.deserialize_struct(
                                unsafe { to_static_str(name) },
                                unsafe { to_static_str_slice(&fields.0) },
                                StructVariantVisitor {
                                    name,
                                    variant,
                                    fields: &fields.0,
                                    tys: &fields.1,
                                    values,
                                },
                            )
                        }
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
        tag: Option<&'a str>,
        #[arbitrary(with = arbitrary_struct_fields_recursion_guard)]
        fields: (Vec<&'a str>, Vec<Self>, Vec<bool>),
    },
    Enum {
        name: &'a str,
        #[arbitrary(with = arbitrary_enum_variants_recursion_guard)]
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
    ) -> arbitrary::Result<SerdeDataValue<'u>>
    where
        'a: 'u,
    {
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
                    Some(()) => Some(Box::new(inner.arbitrary_value(u, pretty)?)),
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
                let inner_value = inner.arbitrary_value(u, pretty)?;

                // ron::value::RawValue cannot safely be constructed from syntactically invalid ron
                if *name == RAW_VALUE_TOKEN {
                    // Hacky way to find out if a string is serialised:
                    //  1. serialise into RON
                    //  2. try to deserialise into a String
                    let Ok(inner_ron) = ron::to_string(&BorrowedTypedSerdeData {
                        ty: inner,
                        value: &inner_value,
                    }) else {
                        return Err(arbitrary::Error::IncorrectFormat);
                    };

                    let Ok(ron) = ron::from_str::<String>(&inner_ron) else {
                        return Err(arbitrary::Error::IncorrectFormat);
                    };

                    // Check that the raw value content is valid
                    if ron::value::RawValue::from_ron(&ron).is_err() {
                        return Err(arbitrary::Error::IncorrectFormat);
                    }
                }

                name_length += name.len();

                SerdeDataValue::Newtype {
                    inner: Box::new(inner_value),
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
            SerdeDataType::Struct { name, tag, fields } => {
                name_length += name.len();
                if let Some(tag) = tag {
                    // This is not very elegant but emulates internally tagged structs
                    //  in a way that minimises code in the fuzzer and actually allows
                    //  the struct to roundtrip even in a `#[serde(deny_unknown_fields)]`
                    //  like mode, which internally tagged structs in serde can not
                    if !matches!((fields.0.first(), fields.1.first()), (Some(field), Some(SerdeDataType::String)) if field == tag)
                    {
                        fields.0.insert(0, tag);
                        fields.1.insert(0, SerdeDataType::String);
                        fields.2.insert(0, false);
                    }
                }
                let mut r#struct = Vec::with_capacity(fields.1.len() + usize::from(tag.is_some()));
                if let Some(tag) = tag {
                    name_length += tag.len();
                    r#struct.push(SerdeDataValue::String(name));
                }
                for (field, ty) in fields
                    .0
                    .iter()
                    .zip(&mut fields.1)
                    .skip(usize::from(tag.is_some()))
                {
                    name_length += field.len();
                    r#struct.push(ty.arbitrary_value(u, pretty)?);
                }
                let value = SerdeDataValue::Struct { fields: r#struct };
                let mut has_flatten_map = false;
                let mut has_unknown_key_inside_flatten = false;
                for (ty, flatten) in fields.1.iter().zip(fields.2.iter()) {
                    if *flatten && !ty.supported_inside_untagged(pretty, false, false) {
                        // Flattened fields are deserialised through serde's content type
                        return Err(arbitrary::Error::IncorrectFormat);
                    }
                    if !ty.supported_flattened_map_inside_flatten_field(
                        pretty,
                        *flatten,
                        false,
                        &mut has_flatten_map,
                        &mut has_unknown_key_inside_flatten,
                    ) {
                        // Flattened fields with maps must fulfil certain criteria
                        return Err(arbitrary::Error::IncorrectFormat);
                    }
                    if *flatten
                        && (pretty.struct_names
                            || pretty
                                .extensions
                                .contains(Extensions::EXPLICIT_STRUCT_NAMES))
                    {
                        // BUG: struct names inside flattend structs do not roundtrip
                        return Err(arbitrary::Error::IncorrectFormat);
                    }
                }
                value
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
                    if pretty.struct_names || pretty.extensions.contains(Extensions::EXPLICIT_STRUCT_NAMES)
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

                match representation {
                    SerdeEnumRepresentation::ExternallyTagged
                    | SerdeEnumRepresentation::Untagged => (),
                    SerdeEnumRepresentation::InternallyTagged { tag } => name_length += tag.len(),
                    SerdeEnumRepresentation::AdjacentlyTagged { tag, content } => {
                        name_length += tag.len() + content.len();
                    }
                };

                let value = match ty {
                    SerdeDataVariantType::Unit => SerdeDataVariantValue::Unit,
                    SerdeDataVariantType::TaggedOther => {
                        if matches!(
                            representation,
                            SerdeEnumRepresentation::ExternallyTagged
                                | SerdeEnumRepresentation::Untagged
                        ) {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }

                        SerdeDataVariantValue::TaggedOther {
                            variant: u.arbitrary()?,
                            index: u.int_in_range(
                                u32::try_from(variants.1.len())
                                    .map_err(|_| arbitrary::Error::IncorrectFormat)?
                                    ..=u32::MAX,
                            )?,
                        }
                    }
                    SerdeDataVariantType::Newtype { ref mut inner } => {
                        let value = Box::new(inner.arbitrary_value(u, pretty)?);
                        if matches!(representation, SerdeEnumRepresentation::Untagged | SerdeEnumRepresentation::InternallyTagged { tag: _ } if !inner.supported_inside_untagged(pretty, true, false))
                        {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }
                        if matches!(representation, SerdeEnumRepresentation::InternallyTagged { tag: _ } if !inner.supported_inside_internally_tagged_newtype(false))
                        {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }
                        SerdeDataVariantValue::Newtype { inner: value }
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

                        if matches!(representation, SerdeEnumRepresentation::Untagged)
                            && pretty
                                .extensions
                                .contains(Extensions::UNWRAP_VARIANT_NEWTYPES)
                            && fields.len() == 1
                        {
                            // BUG: one-sized tuple variant inside some variant newtype will look
                            //      like just the variant newtype without the tuple wrapper
                            return Err(arbitrary::Error::IncorrectFormat);
                        }

                        let mut tuple = Vec::with_capacity(fields.len());
                        for ty in &mut *fields {
                            tuple.push(ty.arbitrary_value(u, pretty)?);
                        }
                        let value = SerdeDataVariantValue::Struct { fields: tuple };
                        if matches!(representation, SerdeEnumRepresentation::Untagged | SerdeEnumRepresentation::InternallyTagged { tag: _ } if !fields.iter().all(|field| field.supported_inside_untagged(pretty, false, false)))
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
                        if matches!(representation, SerdeEnumRepresentation::Untagged | SerdeEnumRepresentation::InternallyTagged { tag: _ } if !fields.1.iter().all(|field| field.supported_inside_untagged(pretty, false, false)))
                        {
                            return Err(arbitrary::Error::IncorrectFormat);
                        }
                        let mut has_flatten_map = false;
                        let mut has_unknown_key_inside_flatten = false;
                        for (ty, flatten) in fields.1.iter().zip(fields.2.iter()) {
                            if *flatten && !ty.supported_inside_untagged(pretty, false, false) {
                                // Flattened fields are deserialised through serde's content type
                                return Err(arbitrary::Error::IncorrectFormat);
                            }
                            if !ty.supported_flattened_map_inside_flatten_field(
                                pretty,
                                *flatten,
                                false,
                                &mut has_flatten_map,
                                &mut has_unknown_key_inside_flatten,
                            ) {
                                // Flattened fields with maps must fulfil certain criteria
                                return Err(arbitrary::Error::IncorrectFormat);
                            }
                            if *flatten
                                && (pretty.struct_names
                                    || pretty
                                        .extensions
                                        .contains(Extensions::EXPLICIT_STRUCT_NAMES))
                            {
                                // BUG: struct names inside flattend structs do not roundtrip
                                return Err(arbitrary::Error::IncorrectFormat);
                            }
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

    fn supported_inside_untagged(
        &self,
        pretty: &PrettyConfig,
        inside_newtype_variant: bool,
        inside_option: bool,
    ) -> bool {
        match self {
            SerdeDataType::Unit => {
                // BUG: implicit `Some(())` is serialized as just `()`,
                //      which Option's deserializer accepts as `None`
                !(inside_option && pretty.extensions.contains(Extensions::IMPLICIT_SOME))
            }
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
            SerdeDataType::Option { inner } => inner.supported_inside_untagged(pretty, true, true),
            SerdeDataType::Array { kind, len } => {
                if *len == 0 {
                    // BUG: a zero-length array look like a unit to ron
                    return false;
                }

                if *len == 1
                    && inside_newtype_variant
                    && pretty
                        .extensions
                        .contains(Extensions::UNWRAP_VARIANT_NEWTYPES)
                {
                    // BUG: a one-length array inside an unwrapped variant newtype will be swallowed
                    return false;
                }

                kind.supported_inside_untagged(pretty, false, false)
            }
            SerdeDataType::Tuple { elems } => {
                if elems.is_empty() {
                    // BUG: a zero-length tuple look like a unit to ron
                    return false;
                }

                if elems.len() == 1
                    && inside_newtype_variant
                    && pretty
                        .extensions
                        .contains(Extensions::UNWRAP_VARIANT_NEWTYPES)
                {
                    // BUG: a one-length tuple inside an unwrapped variant newtype will be swallowed
                    return false;
                }

                elems
                    .iter()
                    .all(|element| element.supported_inside_untagged(pretty, false, false))
            }
            SerdeDataType::Vec { item } => item.supported_inside_untagged(pretty, false, false),
            SerdeDataType::Map { key, value } => {
                key.supported_inside_untagged(pretty, false, false)
                    && value.supported_inside_untagged(pretty, false, false)
            }
            SerdeDataType::UnitStruct { name: _ } => {
                // unit structs always serilize as units here since struct names
                //  are never allowed inside untagged

                // BUG: implicit `Some(())` is serialized as just `()`,
                //      which Option's deserializer accepts as `None`
                !(inside_option && pretty.extensions.contains(Extensions::IMPLICIT_SOME))
            }
            SerdeDataType::Newtype { name: _, inner: _ } => {
                // if *name == RAW_VALUE_TOKEN {
                //     return false;
                // }

                // inner.supported_inside_untagged(
                //     pretty,
                //     false,
                //     inside_option && pretty.extensions.contains(Extensions::UNWRAP_NEWTYPES),
                // )

                // BUG: newtypes inside untagged look like 1-tuples to ron
                false
            }
            SerdeDataType::TupleStruct { name: _, fields } => {
                if fields.is_empty() {
                    // BUG: an empty tuple struct looks like a unit to ron
                    return false;
                }

                if fields.len() == 1
                    && inside_newtype_variant
                    && pretty
                        .extensions
                        .contains(Extensions::UNWRAP_VARIANT_NEWTYPES)
                {
                    // BUG: a one-length tuple struct inside an unwrapped variant newtype will be swallowed
                    return false;
                }

                fields
                    .iter()
                    .all(|field| field.supported_inside_untagged(pretty, false, false))
            }
            SerdeDataType::Struct {
                name: _,
                tag: _,
                fields,
            } => {
                if fields.0.is_empty() {
                    // BUG: an empty struct looks like a unit to ron
                    return false;
                }

                fields
                    .1
                    .iter()
                    .all(|field| field.supported_inside_untagged(pretty, false, false))
            }
            SerdeDataType::Enum {
                name: _,
                variants,
                representation,
            } => variants.1.iter().all(|variant| match variant {
                SerdeDataVariantType::Unit => {
                    // BUG: implicit `Some(())` is serialized as just `()`,
                    //      which Option's deserializer accepts as `None`
                    !(inside_option
                        && pretty.extensions.contains(Extensions::IMPLICIT_SOME)
                        && matches!(representation, SerdeEnumRepresentation::Untagged))
                }
                SerdeDataVariantType::TaggedOther => true,
                SerdeDataVariantType::Newtype { inner } => inner.supported_inside_untagged(
                    pretty,
                    true,
                    inside_option
                        && pretty
                            .extensions
                            .contains(Extensions::UNWRAP_VARIANT_NEWTYPES),
                ),
                SerdeDataVariantType::Tuple { fields } => {
                    if fields.is_empty() {
                        // BUG: an empty tuple struct looks like a unit to ron
                        return false;
                    }

                    if matches!(representation, SerdeEnumRepresentation::ExternallyTagged)
                        && fields.len() == 1
                    {
                        // BUG: one-sized tuple variant looks like a newtype variant to ron
                        return false;
                    }

                    fields
                        .iter()
                        .all(|field| field.supported_inside_untagged(pretty, false, false))
                }
                SerdeDataVariantType::Struct { fields } => {
                    if fields.0.is_empty() {
                        // BUG: an empty struct looks like a unit to ron
                        return false;
                    }

                    fields
                        .1
                        .iter()
                        .all(|field| field.supported_inside_untagged(pretty, false, false))
                }
            }),
        }
    }

    fn supported_inside_internally_tagged_newtype(
        &self,
        inside_untagged_newtype_variant: bool,
    ) -> bool {
        // See https://github.com/serde-rs/serde/blob/ddc1ee564b33aa584e5a66817aafb27c3265b212/serde/src/private/ser.rs#L94-L336
        match self {
            SerdeDataType::Unit => {
                // BUG: a unit inside an untagged newtype variant expects a unit
                //      but only the tag is there
                !inside_untagged_newtype_variant
            }
            SerdeDataType::Bool => false,
            SerdeDataType::I8 => false,
            SerdeDataType::I16 => false,
            SerdeDataType::I32 => false,
            SerdeDataType::I64 => false,
            SerdeDataType::I128 => false,
            SerdeDataType::ISize => false,
            SerdeDataType::U8 => false,
            SerdeDataType::U16 => false,
            SerdeDataType::U32 => false,
            SerdeDataType::U64 => false,
            SerdeDataType::U128 => false,
            SerdeDataType::USize => false,
            SerdeDataType::F32 => false,
            SerdeDataType::F64 => false,
            SerdeDataType::Char => false,
            SerdeDataType::String => false,
            SerdeDataType::ByteBuf => false,
            SerdeDataType::Option { inner: _ } => false,
            SerdeDataType::Array { kind: _, len: _ } => false,
            SerdeDataType::Tuple { elems: _ } => false,
            SerdeDataType::Vec { item: _ } => false,
            SerdeDataType::Map { key: _, value: _ } => true,
            SerdeDataType::UnitStruct { name: _ } => {
                // BUG: a unit struct inside an untagged newtype variant requires a unit,
                //      but it won't get one because it serialises itself as a unit
                //      (since struct names are not allowed yet inside untagged),
                //      which is only serialised with the tag
                !inside_untagged_newtype_variant
            }
            SerdeDataType::Newtype { name: _, inner: ty } => {
                ty.supported_inside_internally_tagged_newtype(inside_untagged_newtype_variant)
            }
            SerdeDataType::TupleStruct { name: _, fields: _ } => false,
            SerdeDataType::Struct {
                name: _,
                tag: _,
                fields: _,
            } => true,
            SerdeDataType::Enum {
                name: _,
                variants,
                representation,
            } => {
                variants.1.iter().all(|ty| match ty {
                    SerdeDataVariantType::Unit | SerdeDataVariantType::TaggedOther => {
                        // BUG: an untagged unit variant requires a unit,
                        //      but it won't get one because it serialises itself as a unit,
                        //      which is only serialised with the tag
                        !matches!(representation, SerdeEnumRepresentation::Untagged)
                    }
                    SerdeDataVariantType::Newtype { inner: ty } => {
                        if matches!(representation, SerdeEnumRepresentation::Untagged) {
                            ty.supported_inside_internally_tagged_newtype(true)
                        } else {
                            true
                        }
                    }
                    SerdeDataVariantType::Tuple { fields: _ } => !matches!(
                        representation,
                        SerdeEnumRepresentation::Untagged
                            | SerdeEnumRepresentation::AdjacentlyTagged { .. }
                    ),
                    SerdeDataVariantType::Struct { fields: _ } => true,
                })
            }
        }
    }

    fn supported_inside_flatten(&self, inside_untagged_newtype_variant: bool) -> bool {
        match self {
            SerdeDataType::Unit => {
                // BUG: a unit inside an untagged newtype variant expects a unit
                //      but only the tag is there
                !inside_untagged_newtype_variant
            }
            SerdeDataType::Bool => false,
            SerdeDataType::I8 => false,
            SerdeDataType::I16 => false,
            SerdeDataType::I32 => false,
            SerdeDataType::I64 => false,
            SerdeDataType::I128 => false,
            SerdeDataType::ISize => false,
            SerdeDataType::U8 => false,
            SerdeDataType::U16 => false,
            SerdeDataType::U32 => false,
            SerdeDataType::U64 => false,
            SerdeDataType::U128 => false,
            SerdeDataType::USize => false,
            SerdeDataType::F32 => false,
            SerdeDataType::F64 => false,
            SerdeDataType::Char => false,
            SerdeDataType::String => false,
            SerdeDataType::ByteBuf => false,
            SerdeDataType::Option { inner } => {
                inner.supported_inside_flatten(inside_untagged_newtype_variant)
            }
            SerdeDataType::Array { kind: _, len: _ } => false,
            SerdeDataType::Tuple { elems: _ } => false,
            SerdeDataType::Vec { item: _ } => false,
            SerdeDataType::Map { key, value: _ } => key.supported_inside_flatten_key(),
            SerdeDataType::UnitStruct { name: _ } => false,
            SerdeDataType::Newtype { name, inner } => {
                if *name == RAW_VALUE_TOKEN {
                    return false;
                }

                inner.supported_inside_flatten(inside_untagged_newtype_variant)
            }
            SerdeDataType::TupleStruct { name: _, fields: _ } => false,
            SerdeDataType::Struct {
                name: _,
                tag: _,
                fields: _,
            } => true,
            SerdeDataType::Enum {
                name: _,
                variants,
                representation,
            } => variants.1.iter().all(|variant| match variant {
                SerdeDataVariantType::Unit => {
                    // unit variants are not supported
                    // BUG: untagged unit variants are serialised by skipping,
                    //      but the untagged unit variant uses deserialize_any
                    //      and expects to find a unit
                    false
                }
                SerdeDataVariantType::TaggedOther => {
                    // other variants are not supported,
                    //  since they are like unit variants,
                    // which would only work when untagged,
                    //  but other variants are always tagged
                    false
                }
                SerdeDataVariantType::Newtype { inner } => {
                    if matches!(representation, SerdeEnumRepresentation::Untagged) {
                        inner.supported_inside_flatten(true)
                    } else {
                        inner.supported_inside_flatten(inside_untagged_newtype_variant)
                    }
                }
                SerdeDataVariantType::Tuple { fields: _ } => {
                    !matches!(representation, SerdeEnumRepresentation::Untagged)
                }
                SerdeDataVariantType::Struct { fields: _ } => true,
            }),
        }
    }

    fn supported_inside_flatten_key(&self) -> bool {
        // Inside an untagged enum, we can support u8, u64, &str, and &[u8]
        // Outside an untagged enum, we can also support (), bool, i*, u*, f*, and char
        // However, if a flattend struct has two fields which serialize themselves as maps
        //  and have two different key types, deserializing will fail
        // Only allowing string keys (for now) is both sensible and easier
        matches!(self, SerdeDataType::String)
    }

    fn supported_flattened_map_inside_flatten_field(
        &self,
        pretty: &PrettyConfig,
        is_flattened: bool,
        is_untagged: bool,
        has_flattened_map: &mut bool,
        has_unknown_key: &mut bool,
    ) -> bool {
        match self {
            SerdeDataType::Unit => true,
            SerdeDataType::Bool => true,
            SerdeDataType::I8 => true,
            SerdeDataType::I16 => true,
            SerdeDataType::I32 => true,
            SerdeDataType::I64 => true,
            SerdeDataType::I128 => true,
            SerdeDataType::ISize => true,
            SerdeDataType::U8 => true,
            SerdeDataType::U16 => true,
            SerdeDataType::U32 => true,
            SerdeDataType::U64 => true,
            SerdeDataType::U128 => true,
            SerdeDataType::USize => true,
            SerdeDataType::F32 => true,
            SerdeDataType::F64 => true,
            SerdeDataType::Char => true,
            SerdeDataType::String => true,
            SerdeDataType::ByteBuf => true,
            SerdeDataType::Option { inner } => {
                if is_flattened && is_untagged {
                    // BUG: (serde)
                    //  - serialising a flattened None only produces an empty struct
                    //  - deserialising content from an empty flatten struct produces an empty map
                    //  - deserialising an option from a content empty map produces some
                    false
                } else if is_flattened || pretty.extensions.contains(Extensions::IMPLICIT_SOME) {
                    inner.supported_flattened_map_inside_flatten_field(
                        pretty,
                        is_flattened,
                        is_untagged,
                        has_flattened_map,
                        has_unknown_key,
                    )
                } else {
                    true
                }
            }
            SerdeDataType::Array { kind: _, len: _ } => true,
            SerdeDataType::Tuple { elems: _ } => true,
            SerdeDataType::Vec { item: _ } => true,
            SerdeDataType::Map { key: _, value: _ } => {
                if is_flattened {
                    if *has_unknown_key {
                        // BUG: a flattened map will also see the unknown key (serde)
                        return false;
                    }
                    if *has_flattened_map {
                        // BUG: at most one flattened map is supported (serde)
                        return false;
                    }
                    *has_flattened_map = true;
                }
                true
            }
            SerdeDataType::UnitStruct { name: _ } => true,
            SerdeDataType::Newtype { name: _, inner } => {
                if is_flattened || pretty.extensions.contains(Extensions::UNWRAP_NEWTYPES) {
                    inner.supported_flattened_map_inside_flatten_field(
                        pretty,
                        is_flattened,
                        is_untagged,
                        has_flattened_map,
                        has_unknown_key,
                    )
                } else {
                    true
                }
            }
            SerdeDataType::TupleStruct { name: _, fields: _ } => true,
            SerdeDataType::Struct {
                name: _,
                tag,
                fields,
            } => {
                if is_flattened {
                    // TODO: is this really the correct requirement?
                    // case clusterfuzz-testcase-minimized-arbitrary-6364230921879552
                    if tag.is_some() {
                        if *has_flattened_map {
                            // BUG: a flattened map will also see the unknown key (serde)
                            return false;
                        }
                        if *has_unknown_key && is_untagged {
                            // BUG: an untagged struct will use a map intermediary and see
                            //      the unknown key (serde)
                            return false;
                        }
                        *has_unknown_key = true;
                    }

                    fields
                        .1
                        .iter()
                        .zip(fields.2.iter())
                        .all(|(field, is_flattened)| {
                            field.supported_flattened_map_inside_flatten_field(
                                pretty,
                                *is_flattened,
                                is_untagged,
                                has_flattened_map,
                                has_unknown_key,
                            )
                        })
                } else if fields.2.iter().any(|x| *x) {
                    if *has_flattened_map {
                        // BUG: a flattened map will also see the unknown key (serde)
                        return false;
                    }
                    *has_unknown_key = true;
                    true
                } else {
                    true
                }
            }
            SerdeDataType::Enum {
                name: _,
                variants,
                representation,
            } => variants.1.iter().all(|variant| match variant {
                SerdeDataVariantType::Unit => if is_flattened && matches!(representation, SerdeEnumRepresentation::InternallyTagged { tag: _ } | SerdeEnumRepresentation::AdjacentlyTagged { tag: _, content: _ }) {
                    if *has_flattened_map {
                        // BUG: a flattened map will also see the unknown key (serde)
                        return false;
                    }
                    *has_unknown_key = true;
                    true
                } else { true },
                SerdeDataVariantType::TaggedOther => if is_flattened && matches!(representation, SerdeEnumRepresentation::InternallyTagged { tag: _ } | SerdeEnumRepresentation::AdjacentlyTagged { tag: _, content: _ }) {
                    if *has_flattened_map {
                        // BUG: a flattened map will also see the unknown key (serde)
                        return false;
                    }
                    *has_unknown_key = true;
                    true
                } else { true },
                SerdeDataVariantType::Newtype { inner } => {
                    if matches!(representation, SerdeEnumRepresentation::Untagged) {
                        inner.supported_flattened_map_inside_flatten_field(
                            pretty,
                            is_flattened,
                            true,
                            has_flattened_map,
                            has_unknown_key,
                        )
                    } else if is_flattened {
                        if matches!(representation, SerdeEnumRepresentation::ExternallyTagged) && *has_unknown_key {
                            // BUG: flattened enums are deserialised using the content deserialiser,
                            //      which expects to see a map with just one field (serde)
                            return false;
                        }

                        if matches!(representation, SerdeEnumRepresentation::InternallyTagged { tag: _ }) {
                            // BUG: an flattened internally tagged newtype alongside other flattened data
                            //      must not contain a unit, unit struct, or untagged unit variant
                            if !inner.supported_inside_internally_tagged_newtype(true) {
                                return false;
                            }

                            if !inner.supported_flattened_map_inside_flatten_field(
                                pretty,
                                is_flattened,
                                false,
                                has_flattened_map,
                                has_unknown_key,
                            ) {
                                return false;
                            }
                        }

                        if *has_flattened_map {
                            // BUG: a flattened map will also see the unknown key (serde)
                            return false;
                        }
                        *has_unknown_key = true;
                        true
                    } else {
                        true
                    }
                }
                SerdeDataVariantType::Tuple { fields: _ } => {
                    if is_flattened && matches!(representation, SerdeEnumRepresentation::ExternallyTagged | SerdeEnumRepresentation::AdjacentlyTagged { tag: _, content: _ }) {
                        if *has_flattened_map {
                            // BUG: a flattened map will also see the unknown key (serde)
                            return false;
                        }
                        *has_unknown_key = true;
                    }
                    true
                }
                SerdeDataVariantType::Struct { fields } => {
                    if is_flattened {
                        if *has_flattened_map {
                            // BUG: a flattened map will also see the unknown key (serde)
                            return false;
                        }
                        *has_unknown_key = true;
                    }

                    if matches!(representation, SerdeEnumRepresentation::Untagged if is_flattened || fields.2.iter().any(|x| *x))
                    {
                        if *has_flattened_map {
                            // BUG: a flattened map will also see the unknown key (serde)
                            return false;
                        }
                        if *has_unknown_key {
                            // BUG: a flattened untagged enum struct will also see the unknown key (serde)
                            return false;
                        }
                        *has_unknown_key = true;
                    }

                    if is_flattened && matches!(representation, SerdeEnumRepresentation::ExternallyTagged) && *has_unknown_key {
                        // BUG: flattened enums are deserialised using the content deserialiser,
                        //      which expects to see a map with just one field (serde)
                        return false;
                    }

                    true
                }
            }),
        }
    }
}

#[derive(Debug, Default, PartialEq, Arbitrary, Serialize)]
pub enum SerdeDataVariantType<'a> {
    #[default]
    Unit,
    TaggedOther,
    Newtype {
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<SerdeDataType<'a>>,
    },
    Tuple {
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<SerdeDataType<'a>>,
    },
    Struct {
        #[arbitrary(with = arbitrary_struct_fields_recursion_guard)]
        fields: (Vec<&'a str>, Vec<SerdeDataType<'a>>, Vec<bool>),
    },
}

#[derive(Debug, Default, PartialEq, Arbitrary, Serialize)]
pub enum SerdeDataVariantValue<'a> {
    #[default]
    Unit,
    TaggedOther {
        variant: &'a str,
        index: u32,
    },
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

fn arbitrary_struct_fields_recursion_guard<'a>(
    u: &mut Unstructured<'a>,
) -> arbitrary::Result<(Vec<&'a str>, Vec<SerdeDataType<'a>>, Vec<bool>)> {
    let max_depth = RECURSION_LIMIT * 2;

    let result = if RECURSION_DEPTH.fetch_add(1, Ordering::Relaxed) < max_depth {
        let mut fields = Vec::new();
        let mut types = Vec::new();
        let mut flattened = Vec::new();

        while u.arbitrary()? {
            fields.push(<&str>::arbitrary(u)?);
            let ty = SerdeDataType::arbitrary(u)?;
            flattened.push(u.arbitrary()? && ty.supported_inside_flatten(false));
            types.push(ty);
        }

        fields.shrink_to_fit();
        types.shrink_to_fit();
        flattened.shrink_to_fit();

        Ok((fields, types, flattened))
    } else {
        Ok((Vec::new(), Vec::new(), Vec::new()))
    };

    RECURSION_DEPTH.fetch_sub(1, Ordering::Relaxed);

    result
}

fn arbitrary_enum_variants_recursion_guard<'a>(
    u: &mut Unstructured<'a>,
) -> arbitrary::Result<(Vec<&'a str>, Vec<SerdeDataVariantType<'a>>)> {
    let max_depth = RECURSION_LIMIT * 2;

    let result = if RECURSION_DEPTH.fetch_add(1, Ordering::Relaxed) < max_depth {
        let mut variants = Vec::new();
        let mut types = Vec::new();

        while u.arbitrary()? {
            variants.push(<&str>::arbitrary(u)?);
            types.push(SerdeDataVariantType::arbitrary(u)?);
        }

        variants.shrink_to_fit();
        types.shrink_to_fit();

        Ok((variants, types))
    } else {
        Ok((Vec::new(), Vec::new()))
    };

    RECURSION_DEPTH.fetch_sub(1, Ordering::Relaxed);

    result
}
