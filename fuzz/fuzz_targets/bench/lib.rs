use std::convert::TryFrom;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use arbitrary::{Arbitrary, Unstructured};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Deserialize, Serialize, Serializer,
};

use ron::{extensions::Extensions, ser::PrettyConfig};

pub fn roundtrip_arbitrary_typed_ron_or_panic(data: &[u8]) -> Option<TypedSerdeData> {
    if let Ok(typed_value) = TypedSerdeData::arbitrary(&mut Unstructured::new(data)) {
        let ron = match ron::Options::default()
            .to_string_pretty(&typed_value, typed_value.pretty_config())
        {
            Ok(ron) => ron,
            // Erroring on deep recursion is better than crashing on a stack overflow
            Err(ron::error::Error::ExceededRecursionLimit) => return None,
            // We want the fuzzer to try to generate valid identifiers
            Err(ron::error::Error::InvalidIdentifier(_)) => return None,
            // The fuzzer can find this code path (lol) but give the wrong data
            Err(ron::error::Error::ExpectedRawValue) => return None,
            // Everything else is actually a bug we want to find
            Err(err) => panic!("{:?} -! {:?}", typed_value, err),
        };
        if let Err(err) = ron::Options::default().from_str::<ron::Value>(&ron) {
            match err.code {
                // Erroring on deep recursion is better than crashing on a stack overflow
                ron::error::Error::ExceededRecursionLimit => return None,
                // FIXME: temporarily allow unimplemented cases to pass
                ron::error::Error::Message(msg) if msg == "fuzz-unimplemented-fuzz" => return None,
                // Everything else is actually a bug we want to find
                err => panic!("{:?} -> {} -! {:?}", typed_value, ron, err),
            }
        };
        if let Err(err) = ron::Options::default().from_str_seed(&ron, &typed_value) {
            match err.code {
                // Erroring on deep recursion is better than crashing on a stack overflow
                ron::error::Error::ExceededRecursionLimit => return None,
                // FIXME: temporarily allow unimplemented cases to pass
                ron::error::Error::Message(msg) if msg == "fuzz-unimplemented-fuzz" => return None,
                // Everything else is actually a bug we want to find
                err => panic!("{:?} -> {} -! {:?}", typed_value, ron, err),
            }
        };
        // TODO: also do typed deserialise
        Some(typed_value)
    } else {
        None
    }
}

#[derive(Debug, PartialEq, Arbitrary)]
struct ArbitraryPrettyConfig {
    /// Limit the pretty-ness up to the given depth.
    depth_limit: usize,
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
}

fn arbitrary_ron_extensions(u: &mut Unstructured) -> arbitrary::Result<Extensions> {
    Extensions::from_bits(usize::arbitrary(u)?).ok_or(arbitrary::Error::IncorrectFormat)
}

impl From<ArbitraryPrettyConfig> for PrettyConfig {
    fn from(arbitrary: ArbitraryPrettyConfig) -> Self {
        Self::default()
            .depth_limit(arbitrary.depth_limit)
            .struct_names(arbitrary.struct_names)
            .separate_tuple_members(arbitrary.separate_tuple_members)
            .enumerate_arrays(arbitrary.enumerate_arrays)
            .extensions(arbitrary.extensions)
            .compact_arrays(arbitrary.compact_arrays)
            .escape_strings(arbitrary.escape_strings)
            .compact_structs(arbitrary.compact_structs)
            .compact_maps(arbitrary.compact_maps)
    }
}

#[derive(Debug, PartialEq)]
pub struct TypedSerdeData {
    pretty_config: PrettyConfig,
    ty: SerdeDataType,
    value: SerdeDataValue,
}

impl TypedSerdeData {
    #[allow(dead_code)]
    pub fn pretty_config(&self) -> PrettyConfig {
        self.pretty_config.clone()
    }
}

struct BorrowedTypedSerdeData<'a> {
    ty: &'a SerdeDataType,
    value: &'a SerdeDataValue,
}

impl Serialize for TypedSerdeData {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        BorrowedTypedSerdeData {
            ty: &self.ty,
            value: &self.value,
        }
        .serialize(serializer)
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &'a TypedSerdeData {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        BorrowedTypedSerdeData {
            ty: &self.ty,
            value: &self.value,
        }
        .deserialize(deserializer)
    }
}

unsafe fn to_static(s: &str) -> &'static str {
    &*(s as *const str)
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
                serializer.serialize_unit_struct(unsafe { to_static(name) })
            }
            (SerdeDataType::Newtype { name, inner }, SerdeDataValue::Newtype { inner: value }) => {
                serializer.serialize_newtype_struct(
                    unsafe { to_static(name) },
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

                let mut tuple =
                    serializer.serialize_tuple_struct(unsafe { to_static(name) }, fields.len())?;
                for (ty, data) in fields.iter().zip(values.iter()) {
                    tuple.serialize_field(&BorrowedTypedSerdeData { ty, value: data })?;
                }
                tuple.end()
            }
            (SerdeDataType::Struct { name, fields }, SerdeDataValue::Struct { fields: values }) => {
                if values.len() != fields.len() {
                    return Err(serde::ser::Error::custom("mismatch struct fields len"));
                }

                let mut r#struct =
                    serializer.serialize_struct(unsafe { to_static(name) }, fields.len())?;
                for ((field, ty), data) in fields.iter().zip(values.iter()) {
                    r#struct.serialize_field(
                        unsafe { to_static(field) },
                        &BorrowedTypedSerdeData { ty, value: data },
                    )?;
                }
                r#struct.end()
            }
            (
                SerdeDataType::Enum { name, variants },
                SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                },
            ) => {
                let (variant, ty) = match variants.get(*variant_index as usize) {
                    Some(variant) => variant,
                    None => return Err(serde::ser::Error::custom("out of bounds variant index")),
                };

                match (ty, value) {
                    (SerdeDataVariantType::Unit, SerdeDataVariantValue::Unit) => serializer
                        .serialize_unit_variant(
                            unsafe { to_static(name) },
                            *variant_index,
                            unsafe { to_static(variant) },
                        ),
                    (
                        SerdeDataVariantType::Newtype { inner: ty },
                        SerdeDataVariantValue::Newtype { inner: value },
                    ) => serializer.serialize_newtype_variant(
                        unsafe { to_static(name) },
                        *variant_index,
                        unsafe { to_static(variant) },
                        &BorrowedTypedSerdeData { ty, value },
                    ),
                    (
                        SerdeDataVariantType::Tuple { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        if values.len() != fields.len() {
                            return Err(serde::ser::Error::custom(
                                "mismatch tuple struct variant fields len",
                            ));
                        }

                        let mut tuple = serializer.serialize_tuple_variant(
                            unsafe { to_static(name) },
                            *variant_index,
                            unsafe { to_static(variant) },
                            fields.len(),
                        )?;
                        for (ty, data) in fields.iter().zip(values.iter()) {
                            tuple.serialize_field(&BorrowedTypedSerdeData { ty, value: data })?;
                        }
                        tuple.end()
                    }
                    (
                        SerdeDataVariantType::Struct { fields },
                        SerdeDataVariantValue::Struct { fields: values },
                    ) => {
                        if values.len() != fields.len() {
                            return Err(serde::ser::Error::custom(
                                "mismatch struct variant fields len",
                            ));
                        }

                        let mut r#struct = serializer.serialize_struct_variant(
                            unsafe { to_static(name) },
                            *variant_index,
                            unsafe { to_static(variant) },
                            fields.len(),
                        )?;
                        for ((field, ty), data) in fields.iter().zip(values.iter()) {
                            r#struct.serialize_field(
                                unsafe { to_static(field) },
                                &BorrowedTypedSerdeData { ty, value: data },
                            )?;
                        }
                        r#struct.end()
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
                deserialize_matching(deserializer, v)
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
                    ty: &'a SerdeDataType,
                    value: Option<&'a SerdeDataValue>,
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
                    kind: &'a SerdeDataType,
                    elems: &'a [SerdeDataValue],
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
                    tys: &'a [SerdeDataType],
                    values: &'a [SerdeDataValue],
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

                if values.len() != tys.len() {
                    return Err(serde::de::Error::custom("mismatch tuple len"));
                }

                deserializer.deserialize_tuple(tys.len(), TupleVisitor { tys, values })
            }
            (SerdeDataType::Vec { item }, SerdeDataValue::Seq { elems }) => {
                struct VecVisitor<'a> {
                    item: &'a SerdeDataType,
                    elems: &'a [SerdeDataValue],
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
                    key: &'a SerdeDataType,
                    value: &'a SerdeDataType,
                    elems: &'a [(SerdeDataValue, SerdeDataValue)],
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

                deserializer
                    .deserialize_unit_struct(unsafe { to_static(name) }, UnitStructVisitor { name })
            }
            (SerdeDataType::Newtype { name, inner }, SerdeDataValue::Newtype { inner: value }) => {
                struct NewtypeVisitor<'a> {
                    name: &'a str,
                    inner: &'a SerdeDataType,
                    value: &'a SerdeDataValue,
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
                }

                deserializer.deserialize_newtype_struct(
                    unsafe { to_static(name) },
                    NewtypeVisitor { name, inner, value },
                )
            }
            (
                SerdeDataType::TupleStruct { name, fields },
                SerdeDataValue::Struct { fields: values },
            ) => {
                let _ = (name, fields, values);
                Err(serde::de::Error::custom("fuzz-unimplemented-fuzz"))
            }
            (SerdeDataType::Struct { name, fields }, SerdeDataValue::Struct { fields: values }) => {
                let _ = (name, fields, values);
                Err(serde::de::Error::custom("fuzz-unimplemented-fuzz"))
            }
            (
                SerdeDataType::Enum { name, variants },
                SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                },
            ) => {
                let _ = (name, variants, variant_index, value);
                Err(serde::de::Error::custom("fuzz-unimplemented-fuzz"))
            }
            _ => Err(serde::de::Error::custom("invalid serde data")),
        }
    }
}

impl<'a> Arbitrary<'a> for TypedSerdeData {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let pretty_config = ArbitraryPrettyConfig::arbitrary(u)?.into();
        let ty = SerdeDataType::arbitrary(u)?;
        let value = ty.arbitrary_value(u)?;
        Ok(Self {
            pretty_config,
            ty,
            value,
        })
    }
}

#[derive(Debug, Default, PartialEq, Arbitrary)]
enum SerdeDataValue {
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
    String(String),
    ByteBuf(Vec<u8>),
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
        value: SerdeDataVariantValue,
    },
}

#[derive(Debug, Default, PartialEq, Arbitrary)]
enum SerdeDataType {
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
        #[arbitrary(with = arbitrary_recursion_guard)]
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
        name: String,
    },
    Newtype {
        name: String,
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<Self>,
    },
    TupleStruct {
        name: String,
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<Self>,
    },
    Struct {
        name: String,
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<(String, Self)>,
    },
    Enum {
        name: String,
        #[arbitrary(with = arbitrary_recursion_guard)]
        variants: Vec<(String, SerdeDataVariantType)>,
    },
}

impl SerdeDataType {
    fn arbitrary_value(&self, u: &mut Unstructured) -> arbitrary::Result<SerdeDataValue> {
        match self {
            SerdeDataType::Unit => Ok(SerdeDataValue::Unit),
            SerdeDataType::Bool => Ok(SerdeDataValue::Bool(bool::arbitrary(u)?)),
            SerdeDataType::I8 => Ok(SerdeDataValue::I8(i8::arbitrary(u)?)),
            SerdeDataType::I16 => Ok(SerdeDataValue::I16(i16::arbitrary(u)?)),
            SerdeDataType::I32 => Ok(SerdeDataValue::I32(i32::arbitrary(u)?)),
            SerdeDataType::I64 => Ok(SerdeDataValue::I64(i64::arbitrary(u)?)),
            SerdeDataType::I128 => Ok(SerdeDataValue::I128(i128::arbitrary(u)?)),
            SerdeDataType::ISize => Ok(SerdeDataValue::ISize(isize::arbitrary(u)?)),
            SerdeDataType::U8 => Ok(SerdeDataValue::U8(u8::arbitrary(u)?)),
            SerdeDataType::U16 => Ok(SerdeDataValue::U16(u16::arbitrary(u)?)),
            SerdeDataType::U32 => Ok(SerdeDataValue::U32(u32::arbitrary(u)?)),
            SerdeDataType::U64 => Ok(SerdeDataValue::U64(u64::arbitrary(u)?)),
            SerdeDataType::U128 => Ok(SerdeDataValue::U128(u128::arbitrary(u)?)),
            SerdeDataType::USize => Ok(SerdeDataValue::USize(usize::arbitrary(u)?)),
            SerdeDataType::F32 => Ok(SerdeDataValue::F32(f32::arbitrary(u)?)),
            SerdeDataType::F64 => Ok(SerdeDataValue::F64(f64::arbitrary(u)?)),
            SerdeDataType::Char => Ok(SerdeDataValue::Char(char::arbitrary(u)?)),
            SerdeDataType::String => Ok(SerdeDataValue::String(String::arbitrary(u)?)),
            SerdeDataType::ByteBuf => Ok(SerdeDataValue::ByteBuf(Vec::<u8>::arbitrary(u)?)),
            SerdeDataType::Option { inner } => {
                let value = match Option::<()>::arbitrary(u)? {
                    Some(_) => Some(Box::new(inner.arbitrary_value(u)?)),
                    None => None,
                };
                Ok(SerdeDataValue::Option { inner: value })
            }
            SerdeDataType::Array { kind, len } => {
                if *len > 32 {
                    // Restrict array lengths to be reasonable, as arbitrary cannot
                    return Err(arbitrary::Error::IncorrectFormat);
                }
                let mut array = Vec::with_capacity(*len);
                for _ in 0..*len {
                    array.push(kind.arbitrary_value(u)?);
                }
                Ok(SerdeDataValue::Seq { elems: array })
            }
            SerdeDataType::Tuple { elems } => {
                let mut tuple = Vec::with_capacity(elems.len());
                for ty in elems {
                    tuple.push(ty.arbitrary_value(u)?);
                }
                Ok(SerdeDataValue::Seq { elems: tuple })
            }
            SerdeDataType::Vec { item } => {
                let len = u.arbitrary_len::<SerdeDataValue>()?.min(4);
                let mut vec = Vec::with_capacity(len);
                for _ in 0..len {
                    vec.push(item.arbitrary_value(u)?);
                }
                Ok(SerdeDataValue::Seq { elems: vec })
            }
            SerdeDataType::Map { key, value } => {
                let len = u.arbitrary_len::<SerdeDataValue>()?.min(4);
                let mut map = Vec::with_capacity(len);
                for _ in 0..len {
                    map.push((key.arbitrary_value(u)?, value.arbitrary_value(u)?));
                }
                Ok(SerdeDataValue::Map { elems: map })
            }
            SerdeDataType::UnitStruct { name: _ } => Ok(SerdeDataValue::UnitStruct),
            SerdeDataType::Newtype { name: _, inner } => Ok(SerdeDataValue::Newtype {
                inner: Box::new(inner.arbitrary_value(u)?),
            }),
            SerdeDataType::TupleStruct { name: _, fields } => {
                let mut tuple = Vec::with_capacity(fields.len());
                for ty in fields {
                    tuple.push(ty.arbitrary_value(u)?);
                }
                Ok(SerdeDataValue::Struct { fields: tuple })
            }
            SerdeDataType::Struct { name: _, fields } => {
                let mut r#struct = Vec::with_capacity(fields.len());
                for (_, ty) in fields {
                    r#struct.push(ty.arbitrary_value(u)?);
                }
                Ok(SerdeDataValue::Struct { fields: r#struct })
            }
            SerdeDataType::Enum { name: _, variants } => {
                let variant_index = u.choose_index(variants.len())?;
                let (_, ty) = match variants.get(variant_index) {
                    Some(variant) => variant,
                    None => return Err(arbitrary::Error::EmptyChoose),
                };
                let variant_index =
                    u32::try_from(variant_index).map_err(|_| arbitrary::Error::IncorrectFormat)?;

                let value = match ty {
                    SerdeDataVariantType::Unit => SerdeDataVariantValue::Unit,
                    SerdeDataVariantType::Newtype { inner } => SerdeDataVariantValue::Newtype {
                        inner: Box::new(inner.arbitrary_value(u)?),
                    },
                    SerdeDataVariantType::Tuple { fields } => {
                        let mut tuple = Vec::with_capacity(fields.len());
                        for ty in fields {
                            tuple.push(ty.arbitrary_value(u)?);
                        }
                        SerdeDataVariantValue::Struct { fields: tuple }
                    }
                    SerdeDataVariantType::Struct { fields } => {
                        let mut r#struct = Vec::with_capacity(fields.len());
                        for (_, ty) in fields {
                            r#struct.push(ty.arbitrary_value(u)?);
                        }
                        SerdeDataVariantValue::Struct { fields: r#struct }
                    }
                };

                Ok(SerdeDataValue::Enum {
                    variant: variant_index,
                    value,
                })
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Arbitrary)]
enum SerdeDataVariantType {
    #[default]
    Unit,
    Newtype {
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<SerdeDataType>,
    },
    Tuple {
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<SerdeDataType>,
    },
    Struct {
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<(String, SerdeDataType)>,
    },
}

#[derive(Debug, Default, PartialEq, Arbitrary)]
enum SerdeDataVariantValue {
    #[default]
    Unit,
    Newtype {
        #[arbitrary(with = arbitrary_recursion_guard)]
        inner: Box<SerdeDataValue>,
    },
    Struct {
        #[arbitrary(with = arbitrary_recursion_guard)]
        fields: Vec<SerdeDataValue>,
    },
}

fn arbitrary_recursion_guard<'a, T: Arbitrary<'a> + Default>(
    u: &mut Unstructured<'a>,
) -> arbitrary::Result<T> {
    static RECURSION_DEPTH: AtomicUsize = AtomicUsize::new(0);

    let max_depth = ron::Options::default()
        .recursion_limit
        .map_or(256, |limit| limit * 2);

    let result = if RECURSION_DEPTH.fetch_add(1, Ordering::Relaxed) < max_depth {
        T::arbitrary(u)
    } else {
        Ok(T::default())
    };

    RECURSION_DEPTH.fetch_sub(1, Ordering::Relaxed);

    result
}