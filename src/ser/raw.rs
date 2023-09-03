use std::fmt;

use serde::{ser, Serialize};

use super::{Error, Result};

pub struct Serializer<'a, W: fmt::Write> {
    ser: &'a mut super::Serializer<W>,
}

impl<'a, W: fmt::Write> Serializer<'a, W> {
    pub fn new(ser: &'a mut super::Serializer<W>) -> Self {
        Self { ser }
    }
}

impl<'a, W: fmt::Write> ser::Serializer for Serializer<'a, W> {
    type Error = Error;
    type Ok = ();
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeStruct = ser::Impossible<(), Error>;
    type SerializeStructVariant = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;

    fn serialize_bool(self, _: bool) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_i8(self, _: i8) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_i16(self, _: i16) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_i32(self, _: i32) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_i64(self, _: i64) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    #[cfg(feature = "integer128")]
    fn serialize_i128(self, _: i128) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_u8(self, _: u8) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_u16(self, _: u16) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_u32(self, _: u32) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_u64(self, _: u64) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    #[cfg(feature = "integer128")]
    fn serialize_u128(self, _: u128) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_f32(self, _: f32) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_f64(self, _: f64) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_char(self, _: char) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_str(self, ron: &str) -> Result<()> {
        self.ser.output.write_str(ron)?;
        Ok(())
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_none(self) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_unit(self) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _: &'static str, _: &T) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<()> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct> {
        Err(Error::ExpectedRawValue)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::ExpectedRawValue)
    }
}

#[cfg(test)]
mod tests {
    macro_rules! test_non_raw_value {
        ($test_name:ident => $serialize_method:ident($($serialize_param:expr),*)) => {
            #[test]
            fn $test_name() {
                use serde::{Serialize, Serializer};

                struct Inner;

                impl Serialize for Inner {
                    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                        serializer.$serialize_method($($serialize_param),*).map(|_| unreachable!())
                    }
                }

                #[derive(Debug)]
                struct Newtype;

                impl Serialize for Newtype {
                    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                        serializer.serialize_newtype_struct(
                            crate::value::raw::RAW_VALUE_TOKEN, &Inner,
                        )
                    }
                }

                assert_eq!(
                    crate::to_string(&Newtype).unwrap_err(),
                    crate::Error::ExpectedRawValue
                )
            }
        };
    }

    test_non_raw_value! { test_bool => serialize_bool(false) }
    test_non_raw_value! { test_i8 => serialize_i8(0) }
    test_non_raw_value! { test_i16 => serialize_i16(0) }
    test_non_raw_value! { test_i32 => serialize_i32(0) }
    test_non_raw_value! { test_i64 => serialize_i64(0) }
    #[cfg(feature = "integer128")]
    test_non_raw_value! { test_i128 => serialize_i128(0) }
    test_non_raw_value! { test_u8 => serialize_u8(0) }
    test_non_raw_value! { test_u16 => serialize_u16(0) }
    test_non_raw_value! { test_u32 => serialize_u32(0) }
    test_non_raw_value! { test_u64 => serialize_u64(0) }
    #[cfg(feature = "integer128")]
    test_non_raw_value! { test_u128 => serialize_u128(0) }
    test_non_raw_value! { test_f32 => serialize_f32(0.0) }
    test_non_raw_value! { test_f64 => serialize_f64(0.0) }
    test_non_raw_value! { test_char => serialize_char('\0') }
    test_non_raw_value! { test_bytes => serialize_bytes(b"") }
    test_non_raw_value! { test_none => serialize_none() }
    test_non_raw_value! { test_some => serialize_some(&()) }
    test_non_raw_value! { test_unit => serialize_unit() }
    test_non_raw_value! { test_unit_struct => serialize_unit_struct("U") }
    test_non_raw_value! { test_unit_variant => serialize_unit_variant("E", 0, "U") }
    test_non_raw_value! { test_newtype_struct => serialize_newtype_struct("N", &()) }
    test_non_raw_value! { test_newtype_variant => serialize_newtype_variant("E", 0, "N", &()) }
    test_non_raw_value! { test_seq => serialize_seq(None) }
    test_non_raw_value! { test_tuple => serialize_tuple(0) }
    test_non_raw_value! { test_tuple_struct => serialize_tuple_struct("T", 0) }
    test_non_raw_value! { test_tuple_variant => serialize_tuple_variant("E", 0, "T", 0) }
    test_non_raw_value! { test_map => serialize_map(None) }
    test_non_raw_value! { test_struct => serialize_struct("S", 0) }
    test_non_raw_value! { test_struct_variant => serialize_struct_variant("E", 0, "S", 0) }
}
