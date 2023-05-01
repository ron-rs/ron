// Inspired by David Tolnay's serde-rs
// https://github.com/serde-rs/json/blob/master/src/raw.rs
// Licensed under either of Apache License, Version 2.0 or MIT license at your option.

use std::fmt;

use serde::{de, ser, Deserialize, Serialize};

use crate::{
    error::{Error, SpannedResult},
    options::Options,
};

pub(crate) const RAW_VALUE_TOKEN: &str = "$ron::private::RawValue";

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawValue {
    ron: str,
}

impl RawValue {
    fn from_borrowed_str(ron: &str) -> &Self {
        // Safety: RawValue is a transparent newtype around str
        unsafe { std::mem::transmute::<&str, &RawValue>(ron) }
    }

    fn from_boxed_str(ron: Box<str>) -> Box<Self> {
        // Safety: RawValue is a transparent newtype around str
        unsafe { std::mem::transmute::<Box<str>, Box<RawValue>>(ron) }
    }

    fn into_boxed_str(raw_value: Box<Self>) -> Box<str> {
        // Safety: RawValue is a transparent newtype around str
        unsafe { std::mem::transmute::<Box<RawValue>, Box<str>>(raw_value) }
    }
}

impl Clone for Box<RawValue> {
    fn clone(&self) -> Self {
        (**self).to_owned()
    }
}

impl ToOwned for RawValue {
    type Owned = Box<RawValue>;

    fn to_owned(&self) -> Self::Owned {
        RawValue::from_boxed_str(self.ron.to_owned().into_boxed_str())
    }
}

impl fmt::Debug for RawValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("RawValue")
            .field(&format_args!("{}", &self.ron))
            .finish()
    }
}

impl fmt::Display for RawValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.ron)
    }
}

impl RawValue {
    /// Get the inner raw RON string, which is guaranteed to contain valid RON.
    pub fn get_ron(&self) -> &str {
        &self.ron
    }

    /// Helper function to validate a RON string and turn it into a
    /// [`RawValue`].
    pub fn from_ron(ron: &str) -> SpannedResult<&Self> {
        Options::default()
            .from_str::<&Self>(ron)
            .map(|_| Self::from_borrowed_str(ron))
    }

    /// Helper function to validate a RON string and turn it into a
    /// [`RawValue`].
    pub fn from_boxed_ron(ron: Box<str>) -> SpannedResult<Box<Self>> {
        match Options::default().from_str::<&Self>(&ron) {
            Ok(_) => Ok(Self::from_boxed_str(ron)),
            Err(err) => Err(err),
        }
    }

    /// Helper function to deserialize the inner RON string into `T`.
    pub fn into_rust<'de, T: Deserialize<'de>>(&'de self) -> SpannedResult<T> {
        Options::default().from_str(&self.ron)
    }

    /// Helper function to serialize `value` into a RON string.
    pub fn from_rust<T: Serialize>(value: &T) -> Result<Box<Self>, Error> {
        let ron = Options::default().to_string(value)?;

        Ok(RawValue::from_boxed_str(ron.into_boxed_str()))
    }
}

impl From<Box<RawValue>> for Box<str> {
    fn from(raw_value: Box<RawValue>) -> Self {
        RawValue::into_boxed_str(raw_value)
    }
}

impl Serialize for RawValue {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(RAW_VALUE_TOKEN, &self.ron)
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a RawValue {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ReferenceVisitor;

        impl<'de> de::Visitor<'de> for ReferenceVisitor {
            type Value = &'de RawValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                // This error message only shows up with foreign Deserializers
                write!(formatter, "any valid borrowed RON-value-string")
            }

            fn visit_borrowed_str<E: de::Error>(self, ron: &'de str) -> Result<Self::Value, E> {
                match Options::default().from_str::<de::IgnoredAny>(ron) {
                    Ok(_) => Ok(RawValue::from_borrowed_str(ron)),
                    Err(err) => Err(de::Error::custom(format!(
                        "invalid RON value at {}: {}",
                        err.position, err.code
                    ))),
                }
            }

            fn visit_newtype_struct<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.deserialize_str(self)
            }
        }

        deserializer.deserialize_newtype_struct(RAW_VALUE_TOKEN, ReferenceVisitor)
    }
}

impl<'de> Deserialize<'de> for Box<RawValue> {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BoxedVisitor;

        impl<'de> de::Visitor<'de> for BoxedVisitor {
            type Value = Box<RawValue>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                // This error message only shows up with foreign Deserializers
                write!(formatter, "any valid RON-value-string")
            }

            fn visit_str<E: de::Error>(self, ron: &str) -> Result<Self::Value, E> {
                match Options::default().from_str::<de::IgnoredAny>(ron) {
                    Ok(_) => Ok(RawValue::from_boxed_str(ron.to_owned().into_boxed_str())),
                    Err(err) => Err(de::Error::custom(format!(
                        "invalid RON value at {}: {}",
                        err.position, err.code
                    ))),
                }
            }

            fn visit_string<E: de::Error>(self, ron: String) -> Result<Self::Value, E> {
                match Options::default().from_str::<de::IgnoredAny>(&ron) {
                    Ok(_) => Ok(RawValue::from_boxed_str(ron.into_boxed_str())),
                    Err(err) => Err(de::Error::custom(format!(
                        "invalid RON value at {}: {}",
                        err.position, err.code
                    ))),
                }
            }

            fn visit_newtype_struct<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.deserialize_string(self)
            }
        }

        deserializer.deserialize_newtype_struct(RAW_VALUE_TOKEN, BoxedVisitor)
    }
}
