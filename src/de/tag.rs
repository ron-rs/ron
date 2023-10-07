use serde::de::{self, Visitor};

use super::{Error, Result};

pub struct Deserializer<'a, 'b: 'a> {
    de: &'a mut super::Deserializer<'b>,
}

impl<'a, 'b: 'a> Deserializer<'a, 'b> {
    pub fn new(de: &'a mut super::Deserializer<'b>) -> Self {
        Deserializer { de }
    }
}

impl<'a, 'b: 'a, 'c> de::Deserializer<'b> for &'c mut Deserializer<'a, 'b> {
    type Error = Error;

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        self.de.deserialize_str(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'b>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    #[cfg(feature = "integer128")]
    fn deserialize_i128<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    #[cfg(feature = "integer128")]
    fn deserialize_u128<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        Err(Error::ExpectedString)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'b>,
    {
        self.deserialize_any(visitor)
    }
}
