use serde::de::{self, Visitor};

use super::{Error, Result};

pub struct IdDeserializer<D> {
    d: D,
}

impl<D> IdDeserializer<D> {
    pub fn new(d: D) -> Self {
        IdDeserializer {
            d
        }
    }
}

impl<'a, D> de::Deserializer<'a> for IdDeserializer<D>
where D: de::Deserializer<'a, Error = Error>
{
    type Error = Error;

    fn deserialize_identifier<V>(
        self,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'a>
    {
        self.d.deserialize_identifier(visitor)
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        self.deserialize_identifier(visitor)
    }

    fn deserialize_bool<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_i8<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_i16<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_i32<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_i64<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_u8<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_u16<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_u32<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_u64<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_f32<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_f64<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_char<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_str<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_string<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_bytes<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_byte_buf<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_option<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_unit<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_unit_struct<V>(
        self,
        _: &'static str,
        _: V
    ) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        _: V
    ) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_seq<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_tuple<V>(
        self,
        _: usize,
        _: V
    ) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        _: V
    ) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_map<V>(self, _: V) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V
    ) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V
    ) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }

    fn deserialize_ignored_any<V>(
        self,
        _: V
    ) -> Result<V::Value>
        where V: Visitor<'a>
    {
        unimplemented!("IdDeserializer may only be used for identifiers")
    }
}
