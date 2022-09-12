use std::io;

use serde::{ser, Serialize};

use super::{Error, Result, Serializer};

pub struct RawValueSerializer<'a, W: io::Write> {
    ser: &'a mut Serializer<W>,
}

impl<'a, W: io::Write> RawValueSerializer<'a, W> {
    pub fn new(ser: &'a mut Serializer<W>) -> Self {
        Self { ser }
    }
}

impl<'a, W: io::Write> ser::Serializer for RawValueSerializer<'a, W> {
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
        self.ser.output.write_all(ron.as_bytes())?;
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
