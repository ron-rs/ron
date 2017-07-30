use std::borrow::Cow;
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
use std::error::Error as StdError;
use std::fmt;
use std::str::FromStr;

use pom::{DataInput, Input};
use pom::char_class;
use pom::parser::*;
use serde::de::{self, Deserializer as Deserializer_, DeserializeSeed, Visitor};

type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Eof,
    Syntax,
    ExpectedArray,
    ExpectedArrayComma,
    ExpectedArrayEnd,
    ExpectedBoolean,
    ExpectedEnum,
    ExpectedChar,
    ExpectedFloat,
    ExpectedInteger,
    ExpectedMap,
    ExpectedMapColon,
    ExpectedMapComma,
    ExpectedMapEnd,
    ExpectedStruct,
    ExpectedStructEnd,
    ExpectetUnit,
    ExpectedStructName,
    ExpectedString,
    ExpectedIdentifier,

    /// A custom error emitted by the deserializer.
    Message(String),
    TrailingCharacters,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Message(ref e) => write!(f, "Custom message: {}", e),
            _ => unimplemented!()
        }
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Message(ref e) => e,
            _ => unimplemented!()
        }
    }
}

pub struct Deserializer<'de> {
    input: DataInput<'de, u8>,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input: DataInput::new(input.as_bytes()),
        }
    }
    pub fn remainder(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.input.data[self.input.position..])
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
    where T: de::Deserialize<'a>
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.position == deserializer.input.data.len() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

impl<'de> Deserializer<'de> {
    fn parse_unsigned<T>(&mut self) -> Result<T>
        where T: 'static + FromStr, T::Err: fmt::Debug
    {
        let parser = one_of(b"0123456789").repeat(1..);
        parser.convert(|bytes| String::from_utf8(bytes))
              .convert(|string| FromStr::from_str(&string))
              .parse(&mut self.input)
              .map_err(|_| Error::ExpectedInteger)
    }

    fn parse_signed<T>(&mut self) -> Result<T>
        where T: 'static + FromStr, T::Err: fmt::Debug
    {
        let parser = one_of(b"+-").opt() +
                     one_of(b"0123456789").repeat(1..);
        parser.collect()
              .convert(|bytes| String::from_utf8(bytes))
              .convert(|string| FromStr::from_str(&string))
              .parse(&mut self.input)
              .map_err(|_| Error::ExpectedInteger)
    }

    fn parse_float<T>(&mut self) -> Result<T>
        where T: 'static + FromStr, T::Err: fmt::Debug
    {
        let integer = one_of(b"123456789") - one_of(b"0123456789").repeat(0..) | sym(b'0');
        let frac = sym(b'.') + one_of(b"0123456789").repeat(1..);
        let exp = one_of(b"eE") + one_of(b"+-").opt() + one_of(b"0123456789").repeat(1..);
        let parser = sym(b'-').opt() + integer + frac.opt() + exp.opt();

        parser.collect()
              .convert(|bytes| String::from_utf8(bytes))
              .convert(|string| FromStr::from_str(&string))
              .parse(&mut self.input)
              .map_err(|_| Error::ExpectedFloat)
    }

    fn consume(&mut self, what: &'static str) -> Result<()> {
        let parser = seq(what.as_bytes()).discard();
        parser.parse(&mut self.input)
              .map_err(|_| Error::Syntax)
    }
}

fn space<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}
fn comma<'a>() -> Parser<'a, u8, u8> {
    space() * sym(b',') - space()
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        panic!("Give me some!");
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        println!("deserialize_bool: {:?}", self.remainder());
        match seq(b"true").parse(&mut self.input) {
            Ok(_) => visitor.visit_bool(true),
            Err(_) => match seq(b"false").parse(&mut self.input) {
                Ok(_) => visitor.visit_bool(false),
                Err(_) => Err(Error::ExpectedBoolean)
            }
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i8(self.parse_signed()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i16(self.parse_signed()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i32(self.parse_signed()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i64(self.parse_signed()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u8(self.parse_unsigned()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u32(self.parse_unsigned()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u64(self.parse_unsigned()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_f32(self.parse_float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_f64(self.parse_float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let parser = sym(b'\'') * take(1) - sym(b'\'');
        match parser.parse(&mut self.input) {
            Ok(c) => visitor.visit_char(c[0] as char),
            Err(_) => Err(Error::ExpectedChar)
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let special_char = sym(b'\\') | sym(b'/') | sym(b'"')
            | sym(b'b').map(|_|b'\x08') | sym(b'f').map(|_|b'\x0C')
            | sym(b'n').map(|_|b'\n') | sym(b'r').map(|_|b'\r') | sym(b't').map(|_|b'\t');
        let escape_sequence = sym(b'\\') * special_char;
        let char_string = (none_of(b"\\\"") | escape_sequence).repeat(1..).convert(String::from_utf8);
        let utf16_char = seq(b"\\u") * is_a(char_class::hex_digit).repeat(4).convert(String::from_utf8).convert(|digits|u16::from_str_radix(&digits, 16));
        let utf16_string = utf16_char.repeat(1..).map(|chars| decode_utf16(chars).map(|r| r.unwrap_or(REPLACEMENT_CHARACTER)).collect::<String>());
        let parser = sym(b'"') * (char_string | utf16_string) - sym(b'"');

        match parser.parse(&mut self.input) {
            Ok(string) => visitor.visit_str(&string),
            Err(_) => Err(Error::ExpectedString)
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.deserialize_str(visitor)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match seq(b"null").discard().parse(&mut self.input) {
            Ok(_) => visitor.visit_none(),
            Err(_) => visitor.visit_some(self),
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.consume("()") {
            Ok(_) => visitor.visit_unit(),
            Err(_) => Err(Error::ExpectetUnit),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.consume(name) {
            Ok(_) => visitor.visit_unit(),
            Err(_) => Err(Error::ExpectedStructName),
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.consume(name) {
            Ok(_) => match self.consume("(") {
                Ok(_) => {
                    let value = visitor.visit_newtype_struct(&mut *self)?;
                    let _ = comma().parse(&mut self.input);
                    self.consume(")")
                        .map(|_| value)
                        .map_err(|_| Error::ExpectedStructEnd)
                },
                Err(_) => Err(Error::ExpectedStruct),
            },
            Err(_) => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.consume("[") {
            Ok(_) => {
                let value = visitor.visit_seq(CommaSeparated::new(b']', &mut self))?;
                let _ = comma().parse(&mut self.input);
                self.consume("]")
                    .map(|_| value)
                    .map_err(|_| Error::ExpectedArrayEnd)
            },
            Err(_) => Err(Error::ExpectedArray)
        }
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(
        mut self,
        _len: usize,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.consume("(") {
            Ok(_) => {
                let value = visitor.visit_seq(CommaSeparated::new(b')', &mut self))?;
                let _ = comma().parse(&mut self.input);
                self.consume(")")
                    .map(|_| value)
                    .map_err(|_| Error::ExpectedArrayEnd)
            },
            Err(_) => Err(Error::ExpectedArray)
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let _ = self.consume(name);
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        match self.consume("{") {
            Ok(_) => {
                let value = visitor.visit_map(CommaSeparated::new(b'}', &mut self))?;
                let _ = comma().parse(&mut self.input);
                self.consume("}")
                    .map(|_| value)
                    .map_err(|_| Error::ExpectedMapEnd)
            },
            Err(_) => Err(Error::ExpectedMap)
        }
    }

    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let _ = self.consume(name);

        match self.consume("(") {
            Ok(_) => {
                let value = visitor.visit_map(CommaSeparated::new(b')', &mut self))?;
                let _ = comma().parse(&mut self.input);
                self.consume(")")
                    .map(|_| value)
                    .map_err(|_| Error::ExpectedStructEnd)
            },
            Err(_) => Err(Error::ExpectedStruct)
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_enum(Enum::new(self))
    }

    fn deserialize_identifier<V>(
        self,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let first = is_a(|b| char_class::alpha(b) || b == b'_');
        let other = is_a(|b| char_class::alpha(b) || char_class::alphanum(b) || b == b'_');
        let parser = space() * (first + other.repeat(0..)) - space();
        match parser.collect().parse(&mut self.input) {
            Ok(bytes) => visitor.visit_bytes(&bytes),
            Err(_) => Err(Error::ExpectedIdentifier),
        }
    }

    fn deserialize_ignored_any<V>(
        self,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.deserialize_any(visitor)
    }
}

struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    terminator: u8,
    first: bool,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(terminator: u8, de: &'a mut Deserializer<'de>) -> Self {
        CommaSeparated { de, terminator, first: true }
    }
}

impl<'de, 'a> de::SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where T: DeserializeSeed<'de>
    {
        // Check if there are no more elements.
        if self.de.input.current() == Some(self.terminator) {
            return Ok(None)
        }
        // Comma is required before every element except the first.
        if !self.first {
            if comma().parse(&mut self.de.input).is_err() {
                return Err(Error::ExpectedArrayComma);
            }
            if self.de.input.current() == Some(self.terminator) {
                return Ok(None)
            }
        }
        self.first = false;
        let _ = space().parse(&mut self.de.input);
        // Deserialize an array element.
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de, 'a> de::MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
        where K: DeserializeSeed<'de>
    {
        // Check if there are no more elements.
        if self.de.input.current() == Some(self.terminator) {
            return Ok(None)
        }
        // Comma is required before every element except the first.
        if !self.first {
            if comma().parse(&mut self.de.input).is_err() {
                return Err(Error::ExpectedMapComma);
            }
            if self.de.input.current() == Some(self.terminator) {
                return Ok(None)
            }
        }
        self.first = false;
        let _ = space().parse(&mut self.de.input);
        // Deserialize a map key.
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where V: DeserializeSeed<'de>
    {
        let parser = space() * sym(b':') - space();
        match parser.parse(&mut self.de.input) {
            Ok(_) => seed.deserialize(&mut *self.de),
            Err(_) => Err(Error::ExpectedMapColon),
        }
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de: de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
        where V: DeserializeSeed<'de>
    {
        let value = seed.deserialize(&mut *self.de)?;
        Ok((value, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
        where T: DeserializeSeed<'de>
    {
        match self.de.consume("(") {
            Ok(_) => {
                let value = seed.deserialize(&mut *self.de)?;
                let _ = comma().parse(&mut self.de.input);
                self.de.consume(")")
                    .map(|_| value)
                    .map_err(|_| Error::ExpectedStructEnd)
            },
            Err(_) => Err(Error::ExpectedStruct)
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.de.deserialize_struct("", fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Deserialize)]
    struct EmptyStruct1;

    #[derive(Debug, PartialEq, Deserialize)]
    struct EmptyStruct2 {}

    #[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
    struct MyStruct { x: f32, y: f32 }

    #[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
    enum MyEnum {
        A,
        B(bool),
        C(bool, f32),
        D { a: i32, b: i32 }
    }

    #[test]
    fn test_empty_struct() {
        assert_eq!(Ok(EmptyStruct1), from_str("EmptyStruct1"));
        assert_eq!(Ok(EmptyStruct2{}), from_str("EmptyStruct2()"));
    }


    #[test]
    fn test_struct() {
        let my_struct = MyStruct { x: 4.0, y: 7.0 };

        assert_eq!(Ok(my_struct), from_str("MyStruct(x:4,y:7,)"));
        assert_eq!(Ok(my_struct), from_str("(x:4,y:7)"));

        #[derive(Debug, PartialEq, Deserialize)]
        struct NewType(i32);

        assert_eq!(Ok(NewType(42)), from_str("NewType(42)"));
        assert_eq!(Ok(NewType(42)), from_str("42"));

        #[derive(Debug, PartialEq, Deserialize)]
        struct TupleStruct(f32, f32);

        assert_eq!(Ok(TupleStruct(2.0, 5.0)), from_str("TupleStruct(2,5,)"));
        assert_eq!(Ok(TupleStruct(3.0, 4.0)), from_str("(3,4)"));
    }

    #[test]
    fn test_enum() {
        assert_eq!(Ok(MyEnum::A), from_str("A"));
        assert_eq!(Ok(MyEnum::B(true)), from_str("B(true,)"));
        assert_eq!(Ok(MyEnum::C(true, 3.5)), from_str("C(true,3.5,)"));
        assert_eq!(Ok(MyEnum::D { a: 2, b: 3 }), from_str("D(a:2,b:3,)"));
    }

    #[test]
    fn test_array() {
        let empty: [i32; 0] = [];
        assert_eq!(Ok(empty), from_str("()"));
        let empty_array = empty.to_vec();
        assert_eq!(Ok(empty_array), from_str("[]"));

        assert_eq!(Ok([2, 3, 4i32]), from_str("(2,3,4,)"));
        assert_eq!(Ok(([2, 3, 4i32].to_vec())), from_str("[2,3,4,]"));
    }

    #[test]
    fn test_map() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert((true, false), 4);
        map.insert((false, false), 123);

        assert_eq!(Ok(map), from_str("{
            (true,false,):4,
            (false,false,):123,
        }"));
    }
}
