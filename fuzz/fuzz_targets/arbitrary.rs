#![no_main]

use std::borrow::Cow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use serde::{
    de::{MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = SerdeData::arbitrary(&mut Unstructured::new(data)) {
        let ron = match ron::to_string(&value) {
            Ok(ron) => ron,
            Err(ron::error::Error::ExceededRecursionLimit) => return,
            Err(err) => panic!("{:?} -! {:?}", value, err),
        };
        let de = match ron::from_str::<SerdeData>(&ron) {
            Ok(de) => de,
            Err(err) if err.code == ron::error::Error::ExceededRecursionLimit => return,
            Err(err) => panic!("{:?} -> {:?} -! {:?}", value, ron, err),
        };
        assert_eq!(value, de, "{:?} -> {:?} -> {:?}", value, ron, de);
    }
});

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Arbitrary)]
enum SerdeData<'a> {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U128(u128),
    F32(F32),
    F64(F64),
    Char(char),
    #[serde(borrow)]
    Str(Cow<'a, str>),
    String(String),
    #[serde(borrow)]
    Bytes(Cow<'a, [u8]>),
    ByteBuf(Vec<u8>),
    Option(Option<Box<Self>>),
    Unit(()),
    #[serde(borrow)]
    Map(SerdeMap<'a>),
    Seq(Vec<Self>),
    #[serde(borrow)]
    Enum(SerdeEnum<'a>),
    #[serde(borrow)]
    Struct(SerdeStruct<'a>),
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Arbitrary)]
enum SerdeEnum<'a> {
    UnitVariant,
    #[serde(borrow)]
    NewtypeVariant(Box<SerdeData<'a>>),
    TupleVariant(Box<SerdeData<'a>>, Box<SerdeData<'a>>, Box<SerdeData<'a>>),
    StructVariant {
        a: Box<SerdeData<'a>>,
        r#fn: Box<SerdeData<'a>>,
        c: Box<SerdeData<'a>>,
    },
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Arbitrary)]
enum SerdeStruct<'a> {
    Unit(SerdeUnitStruct),
    #[serde(borrow)]
    Newtype(SerdeNewtypeStruct<'a>),
    #[serde(borrow)]
    Tuple(SerdeTupleStruct<'a>),
    #[serde(borrow)]
    Struct(SerdeStructStruct<'a>),
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Arbitrary)]
struct SerdeUnitStruct;

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Arbitrary)]
#[repr(transparent)]
struct SerdeNewtypeStruct<'a>(#[serde(borrow)] Box<SerdeData<'a>>);

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Arbitrary)]
struct SerdeTupleStruct<'a>(
    #[serde(borrow)] Box<SerdeData<'a>>,
    Box<SerdeData<'a>>,
    Box<SerdeData<'a>>,
);

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Arbitrary)]
struct SerdeStructStruct<'a> {
    #[serde(borrow)]
    a: Box<SerdeData<'a>>,
    #[serde(borrow)]
    r#fn: Box<SerdeData<'a>>,
    #[serde(borrow)]
    c: Box<SerdeData<'a>>,
}

#[derive(Debug, Serialize, Deserialize, Arbitrary)]
#[repr(transparent)]
struct F32(f32);

impl PartialEq for F32 {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() && other.0.is_nan() {
            return true;
        }
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for F32 {}

impl Hash for F32 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.0.to_bits())
    }
}

#[derive(Debug, Serialize, Deserialize, Arbitrary)]
#[repr(transparent)]
struct F64(f64);

impl PartialEq for F64 {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() && other.0.is_nan() {
            return true;
        }
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for F64 {}

impl Hash for F64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.to_bits())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Arbitrary)]
struct SerdeMap<'a>(Vec<(SerdeData<'a>, SerdeData<'a>)>);

impl<'a> Serialize for SerdeMap<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }

        map.end()
    }
}

impl<'a, 'de: 'a> Deserialize<'de> for SerdeMap<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SerdeMapVisitor<'a>(PhantomData<&'a ()>);

        impl<'a, 'de: 'a> Visitor<'de> for SerdeMapVisitor<'a> {
            type Value = SerdeMap<'a>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a map")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::with_capacity(map.size_hint().unwrap_or(0));

                while let Some(entry) = map.next_entry()? {
                    values.push(entry);
                }

                Ok(SerdeMap(values))
            }
        }

        deserializer.deserialize_map(SerdeMapVisitor(PhantomData))
    }
}
