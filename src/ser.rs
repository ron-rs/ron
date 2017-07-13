use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::ser::{self, Serialize};

/// Serializes `value` and returns it as string.
pub fn to_string<T>(value: &T) -> Result<String>
    where T: Serialize
{
    let mut s = Serializer { output: String::new() };
    value.serialize(&mut s)?;
    Ok(s.output)
}

type Result<T> = ::std::result::Result<T, Error>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// A custom error emitted by a serialized value.
    Message(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            Error::Message(ref e) => write!(f, "Cusom message: {}", e),
        }
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Message(ref e) => e,
        }
    }
}

pub struct Serializer {
    output: String,
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output += if v { "true" } else { "false" };
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        // TODO optimize
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.output += "\"";
        self.output += v;
        self.output += "\"";
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        use serde::ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<()> {
        self.output += "None";

        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        self.output += "Some(";
        value.serialize(&mut *self)?;
        self.output += ")";

        Ok(())
    }

    fn serialize_unit(self) -> Result<()> {
        self.output += "()";

        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        self.output += name;

        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str
    ) -> Result<()> {
        self.output += variant;

        Ok(())
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        self.output += name;
        self.output += "(";
        value.serialize(&mut *self)?;
        self.output += ")";

        Ok(())
    }

    fn serialize_newtype_variant<T>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T
    ) -> Result<()>
        where T: ?Sized + Serialize
    {
        self.output += variant;
        self.output += "(";
        value.serialize(&mut *self)?;
        self.output += ",)";
        Ok(())
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq> {
        self.output += "[";

        Ok(self)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple> {
        self.output += "(";

        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize
    ) -> Result<Self::SerializeTupleStruct> {
        self.output += name;

        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize
    ) -> Result<Self::SerializeTupleVariant> {
        self.output += variant;
        self.output += "(";

        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.output += "{";

        Ok(self)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _: usize
    ) -> Result<Self::SerializeStruct> {
        self.output += name;
        self.output += "(";

        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize
    ) -> Result<Self::SerializeStructVariant> {
        self.output += variant;
        self.output += "(";
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        value.serialize(&mut **self)?;
        self.output += ",";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += "]";
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        value.serialize(&mut **self)?;
        self.output += ",";

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += ")";

        Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        value.serialize(&mut **self)?;
        self.output += ",";

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += ")";

        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        value.serialize(&mut **self)?;
        self.output += ",";

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        self.output += ":";
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += "}";
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        self.output += key;
        self.output += ":";
        value.serialize(&mut **self)?;
        self.output += ",";

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
        where T: ?Sized + Serialize
    {
        self.output += key;
        self.output += ":";
        value.serialize(&mut **self)?;
        self.output += ",";

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct EmptyStruct1;

    #[derive(Serialize)]
    struct EmptyStruct2 {}

    #[derive(Serialize)]
    struct MyStruct { x: f32, y: f32 }

    #[derive(Serialize)]
    enum MyEnum {
        A,
        B(bool),
        C(bool, f32),
        D { a: i32, b: i32 }
    }

    #[test]
    fn test_empty_struct() {
        assert_eq!(to_string(&EmptyStruct1).unwrap(), "EmptyStruct1");
        assert_eq!(to_string(&EmptyStruct2 {}).unwrap(), "EmptyStruct2()");
    }

    #[test]
    fn test_struct() {
        let my_struct = MyStruct { x: 4.0, y: 7.0 };

        assert_eq!(to_string(&my_struct).unwrap(), "MyStruct(x:4,y:7,)");
    }

    #[test]
    fn test_enum() {
        assert_eq!(to_string(&MyEnum::A).unwrap(), "A");
        assert_eq!(to_string(&MyEnum::B(true)).unwrap(), "B(true,)");
        assert_eq!(to_string(&MyEnum::C(true, 3.5)).unwrap(), "C(true,3.5,)");
        assert_eq!(to_string(&MyEnum::D { a: 2, b: 3 }).unwrap(), "D(a:2,b:3,)");
    }

    #[test]
    fn test_array() {
        let empty: [i32; 0] = [];
        assert_eq!(to_string(&empty).unwrap(), "()");
        let empty_ref: &[i32] = &empty;
        assert_eq!(to_string(&empty_ref).unwrap(), "[]");
    }
}
