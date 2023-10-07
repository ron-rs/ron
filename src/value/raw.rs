// Inspired by David Tolnay's serde-rs
// https://github.com/serde-rs/json/blob/master/src/raw.rs
// Licensed under either of Apache License, Version 2.0 or MIT license at your option.

use std::{fmt, ops::Range};

use serde::{de, ser, Deserialize, Serialize};

use crate::{
    error::{Error, SpannedResult},
    options::Options,
};

// NOTE: Keep synchronised with fuzz/arbitrary::typed_data::RAW_VALUE_TOKEN
pub(crate) const RAW_VALUE_TOKEN: &str = "$ron::private::RawValue";

#[allow(clippy::module_name_repetitions)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawValue {
    ron: str,
}

#[allow(unsafe_code)]
impl RawValue {
    fn from_borrowed_str(ron: &str) -> &Self {
        // Safety: RawValue is a transparent newtype around str
        unsafe { &*(ron as *const str as *const RawValue) }
    }

    fn from_boxed_str(ron: Box<str>) -> Box<Self> {
        // Safety: RawValue is a transparent newtype around str
        unsafe { std::mem::transmute::<Box<str>, Box<RawValue>>(ron) }
    }

    fn into_boxed_str(raw_value: Box<Self>) -> Box<str> {
        // Safety: RawValue is a transparent newtype around str
        unsafe { std::mem::transmute::<Box<RawValue>, Box<str>>(raw_value) }
    }

    #[allow(clippy::expect_used)]
    fn trim_range(ron: &str) -> Range<usize> {
        fn trim_range_inner(ron: &str) -> Result<Range<usize>, Error> {
            let mut deserializer = crate::Deserializer::from_str(ron).map_err(Error::from)?;

            deserializer.parser.skip_ws()?;
            let start_offset = ron.len() - deserializer.parser.src().len();

            let _ = serde::de::IgnoredAny::deserialize(&mut deserializer)?;

            deserializer.parser.skip_ws()?;
            let end_offset = deserializer.parser.pre_ws_src().len();

            Ok(start_offset..(ron.len() - end_offset))
        }

        trim_range_inner(ron).expect("RawValue must contain valid ron")
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
    #[must_use]
    pub fn get_ron(&self) -> &str {
        &self.ron
    }

    /// Helper function to validate a RON string and turn it into a
    /// [`RawValue`].
    pub fn from_ron(ron: &str) -> SpannedResult<&Self> {
        let mut deserializer = crate::Deserializer::from_str(ron)?;

        // raw values can be used everywhere but extensions cannot
        if !deserializer.extensions().is_empty() {
            return Err(deserializer.span_error(Error::Message(String::from(
                "ron::value::RawValue cannot enable extensions",
            ))));
        }

        let _ = <&Self>::deserialize(&mut deserializer).map_err(|e| deserializer.span_error(e))?;

        deserializer.end().map_err(|e| deserializer.span_error(e))?;

        Ok(Self::from_borrowed_str(ron))
    }

    /// Helper function to validate a RON string and turn it into a
    /// [`RawValue`].
    pub fn from_boxed_ron(ron: Box<str>) -> SpannedResult<Box<Self>> {
        match Self::from_ron(&ron) {
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

impl RawValue {
    #[must_use]
    /// Trims any leadning and trailing whitespace off the raw RON string,
    /// including whitespace characters and comments.
    pub fn trim(&self) -> &Self {
        Self::from_borrowed_str(&self.ron[RawValue::trim_range(&self.ron)])
    }

    #[must_use]
    #[allow(unsafe_code)]
    /// Trims any leadning and trailing whitespace off the boxed raw RON string,
    /// including whitespace characters and comments.
    pub fn trim_boxed(self: Box<Self>) -> Box<Self> {
        let trim_range = RawValue::trim_range(&self.ron);
        let mut boxed_ron = RawValue::into_boxed_str(self).into_string();
        // SAFETY: ron[trim_range] is a valid str, so draining everything
        //         before and after leaves a valid str
        unsafe {
            boxed_ron.as_mut_vec().drain(trim_range.end..);
            boxed_ron.as_mut_vec().drain(0..trim_range.start);
        }
        RawValue::from_boxed_str(boxed_ron.into_boxed_str())
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
