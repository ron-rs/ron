/// Deserialization module.
use std::{
    borrow::Cow,
    io::{self, Write},
    str,
};

use base64::Engine;
use serde::{
    de::{self, DeserializeSeed, Deserializer as _, Visitor},
    Deserialize,
};

use self::{id::IdDeserializer, tag::TagDeserializer};
pub use crate::error::{Error, Position, SpannedError};
use crate::{
    error::{Result, SpannedResult},
    extensions::Extensions,
    options::Options,
    parse::{Bytes, NewtypeMode, ParsedStr, StructType, TupleMode, BASE64_ENGINE},
};

mod id;
mod tag;
#[cfg(test)]
mod tests;
mod value;

/// The RON deserializer.
///
/// If you just want to simply deserialize a value,
/// you can use the [`from_str`] convenience function.
pub struct Deserializer<'de> {
    bytes: Bytes<'de>,
    newtype_variant: bool,
    serde_content_newtype: bool,
    last_identifier: Option<&'de str>,
    recursion_limit: Option<usize>,
}

impl<'de> Deserializer<'de> {
    // Cannot implement trait here since output is tied to input lifetime 'de.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> SpannedResult<Self> {
        Self::from_str_with_options(input, Options::default())
    }

    pub fn from_bytes(input: &'de [u8]) -> SpannedResult<Self> {
        Self::from_bytes_with_options(input, Options::default())
    }

    pub fn from_str_with_options(input: &'de str, options: Options) -> SpannedResult<Self> {
        Self::from_bytes_with_options(input.as_bytes(), options)
    }

    pub fn from_bytes_with_options(input: &'de [u8], options: Options) -> SpannedResult<Self> {
        let mut deserializer = Deserializer {
            bytes: Bytes::new(input)?,
            newtype_variant: false,
            serde_content_newtype: false,
            last_identifier: None,
            recursion_limit: options.recursion_limit,
        };

        deserializer.bytes.exts |= options.default_extensions;

        Ok(deserializer)
    }

    pub fn remainder(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.bytes.bytes())
    }

    pub fn span_error(&self, code: Error) -> SpannedError {
        self.bytes.span_error(code)
    }

    pub fn extensions(&self) -> Extensions {
        self.bytes.exts
    }
}

/// A convenience function for building a deserializer
/// and deserializing a value of type `T` from a reader.
pub fn from_reader<R, T>(rdr: R) -> SpannedResult<T>
where
    R: io::Read,
    T: de::DeserializeOwned,
{
    Options::default().from_reader(rdr)
}

/// A convenience function for building a deserializer
/// and deserializing a value of type `T` from a string.
pub fn from_str<'a, T>(s: &'a str) -> SpannedResult<T>
where
    T: de::Deserialize<'a>,
{
    Options::default().from_str(s)
}

/// A convenience function for building a deserializer
/// and deserializing a value of type `T` from bytes.
pub fn from_bytes<'a, T>(s: &'a [u8]) -> SpannedResult<T>
where
    T: de::Deserialize<'a>,
{
    Options::default().from_bytes(s)
}

macro_rules! guard_recursion {
    ($self:expr => $expr:expr) => {{
        if let Some(limit) = &mut $self.recursion_limit {
            if let Some(new_limit) = limit.checked_sub(1) {
                *limit = new_limit;
            } else {
                return Err(Error::ExceededRecursionLimit);
            }
        }

        let result = $expr;

        if let Some(limit) = &mut $self.recursion_limit {
            *limit = limit.saturating_add(1);
        }

        result
    }};
}

impl<'de> Deserializer<'de> {
    /// Check if the remaining bytes are whitespace only,
    /// otherwise return an error.
    pub fn end(&mut self) -> Result<()> {
        self.bytes.skip_ws()?;

        if self.bytes.bytes().is_empty() {
            Ok(())
        } else {
            Err(Error::TrailingCharacters)
        }
    }

    /// Called from [`deserialize_any`][serde::Deserializer::deserialize_any]
    /// when a struct was detected. Decides if there is a unit, tuple or usual
    /// struct and deserializes it accordingly.
    ///
    /// This method assumes there is no identifier left.
    fn handle_any_struct<V>(&mut self, visitor: V, ident: Option<&str>) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // HACK: switch to JSON enum semantics for JSON content
        // Robust impl blocked on https://github.com/serde-rs/serde/pull/2420
        let is_serde_content =
            std::any::type_name::<V::Value>() == "serde::__private::de::content::Content";

        let old_serde_content_newtype = self.serde_content_newtype;
        self.serde_content_newtype = false;

        match (
            self.bytes.check_struct_type(
                NewtypeMode::NoParensMeanUnit,
                if old_serde_content_newtype {
                    TupleMode::DifferentiateNewtype // separate match on NewtypeOrTuple below
                } else {
                    TupleMode::ImpreciseTupleOrNewtype // Tuple and NewtypeOrTuple match equally
                },
            )?,
            ident,
        ) {
            (StructType::Unit, Some(ident)) if is_serde_content => {
                // serde's Content type needs the ident for unit variants
                visitor.visit_str(ident)
            }
            (StructType::Unit, _) => visitor.visit_unit(),
            (_, Some(ident)) if is_serde_content => {
                // serde's Content type uses a singleton map encoding for enums
                visitor.visit_map(SerdeEnumContent {
                    de: self,
                    ident: Some(ident),
                })
            }
            (StructType::Named, _) => {
                // giving no name results in worse errors but is necessary here
                self.handle_struct_after_name("", visitor)
            }
            (StructType::NewtypeOrTuple, _) if old_serde_content_newtype => {
                // deserialize a newtype struct or variant
                self.bytes.consume("(");
                self.bytes.skip_ws()?;
                let result = self.deserialize_any(visitor);
                self.bytes.skip_ws()?;
                self.bytes.consume(")");

                result
            }
            (StructType::Tuple | StructType::NewtypeOrTuple, _) => {
                // first argument is technically incorrect, but ignored anyway
                self.deserialize_tuple(0, visitor)
            }
        }
    }

    /// Called from
    /// [`deserialize_struct`][serde::Deserializer::deserialize_struct],
    /// [`struct_variant`][serde::de::VariantAccess::struct_variant], and
    /// [`handle_any_struct`][Self::handle_any_struct]. Handles
    /// deserialising the enclosing parentheses and everything in between.
    ///
    /// This method assumes there is no struct name identifier left.
    fn handle_struct_after_name<V>(
        &mut self,
        name_for_pretty_errors_only: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.newtype_variant || self.bytes.consume("(") {
            let old_newtype_variant = self.newtype_variant;
            self.newtype_variant = false;

            let value = guard_recursion! { self =>
                visitor
                    .visit_map(CommaSeparated::new(Terminator::Struct, self))
                    .map_err(|err| {
                        struct_error_name(
                            err,
                            if !old_newtype_variant && !name_for_pretty_errors_only.is_empty() {
                                Some(name_for_pretty_errors_only)
                            } else {
                                None
                            },
                        )
                    })?
            };

            self.bytes.skip_ws()?;

            if old_newtype_variant || self.bytes.consume(")") {
                Ok(value)
            } else {
                Err(Error::ExpectedStructLikeEnd)
            }
        } else if name_for_pretty_errors_only.is_empty() {
            Err(Error::ExpectedStructLike)
        } else {
            Err(Error::ExpectedNamedStructLike(name_for_pretty_errors_only))
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.newtype_variant {
            if let Some(b')') = self.bytes.peek() {
                // newtype variant wraps the unit type / a unit struct without name
                return self.deserialize_unit(visitor);
            }

            match self
                .bytes
                .check_struct_type(NewtypeMode::InsideNewtype, TupleMode::DifferentiateNewtype)?
            {
                StructType::Named => {
                    // newtype variant wraps a named struct
                    // giving no name results in worse errors but is necessary here
                    return self.handle_struct_after_name("", visitor);
                }
                StructType::Tuple => {
                    // newtype variant wraps a tuple (struct)
                    // first argument is technically incorrect, but ignored anyway
                    return self.deserialize_tuple(0, visitor);
                }
                /* StructType::NewtypeOrTuple */
                // StructType::Unit is impossible with NewtypeMode::InsideNewtype
                _ => {
                    // continue as usual with the inner content of the newtype variant
                    self.newtype_variant = false;
                }
            }
        }

        if self.bytes.consume_ident("true") {
            return visitor.visit_bool(true);
        } else if self.bytes.consume_ident("false") {
            return visitor.visit_bool(false);
        } else if self.bytes.check_ident("Some") {
            return self.deserialize_option(visitor);
        } else if self.bytes.consume_ident("None") {
            return visitor.visit_none();
        } else if self.bytes.consume("()") {
            return visitor.visit_unit();
        } else if self.bytes.consume_ident("inf") || self.bytes.consume_ident("inff32") {
            return visitor.visit_f32(std::f32::INFINITY);
        } else if self.bytes.consume_ident("inff64") {
            return visitor.visit_f64(std::f64::INFINITY);
        } else if self.bytes.consume_ident("NaN") || self.bytes.consume_ident("NaNf32") {
            return visitor.visit_f32(std::f32::NAN);
        } else if self.bytes.consume_ident("NaNf64") {
            return visitor.visit_f64(std::f64::NAN);
        }

        // `identifier` does not change state if it fails
        let ident = self.bytes.identifier().ok();

        if let Some(ident) = ident {
            self.bytes.skip_ws()?;

            return self.handle_any_struct(visitor, Some(std::str::from_utf8(ident)?));
        }

        match self.bytes.peek_or_eof()? {
            b'(' => self.handle_any_struct(visitor, None),
            b'[' => self.deserialize_seq(visitor),
            b'{' => self.deserialize_map(visitor),
            b'0'..=b'9' | b'+' | b'-' | b'.' => self.bytes.any_number()?.visit(visitor),
            b'"' | b'r' => self.deserialize_string(visitor),
            b'\'' => self.deserialize_char(visitor),
            other => Err(Error::UnexpectedByte(other as char)),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.bytes.bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.bytes.integer()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.bytes.integer()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.bytes.integer()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.bytes.integer()?)
    }

    #[cfg(feature = "integer128")]
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i128(self.bytes.integer()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.bytes.integer()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.bytes.integer()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.bytes.integer()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.bytes.integer()?)
    }

    #[cfg(feature = "integer128")]
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u128(self.bytes.integer()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.bytes.float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.bytes.float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(self.bytes.char()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.bytes.string()? {
            ParsedStr::Allocated(s) => visitor.visit_string(s),
            ParsedStr::Slice(s) => visitor.visit_borrowed_str(s),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(b'[') = self.bytes.peek() {
            let bytes = Vec::<u8>::deserialize(self)?;
            return visitor.visit_byte_buf(bytes);
        }

        let res = {
            let string = self.bytes.string()?;
            let base64_str = match string {
                ParsedStr::Allocated(ref s) => s.as_str(),
                ParsedStr::Slice(s) => s,
            };
            BASE64_ENGINE.decode(base64_str)
        };

        match res {
            Ok(byte_buf) => visitor.visit_byte_buf(byte_buf),
            Err(err) => Err(Error::Base64Error(err)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("None") {
            visitor.visit_none()
        } else if self.bytes.consume("Some") && {
            self.bytes.skip_ws()?;
            self.bytes.consume("(")
        } {
            self.bytes.skip_ws()?;

            self.newtype_variant = self
                .bytes
                .exts
                .contains(Extensions::UNWRAP_VARIANT_NEWTYPES);

            let v = guard_recursion! { self => visitor.visit_some(&mut *self)? };

            self.newtype_variant = false;

            self.bytes.comma()?;

            if self.bytes.consume(")") {
                Ok(v)
            } else {
                Err(Error::ExpectedOptionEnd)
            }
        } else if self.bytes.exts.contains(Extensions::IMPLICIT_SOME) {
            guard_recursion! { self => visitor.visit_some(&mut *self) }
        } else {
            Err(Error::ExpectedOption)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.newtype_variant || self.bytes.consume("()") {
            self.newtype_variant = false;

            visitor.visit_unit()
        } else {
            Err(Error::ExpectedUnit)
        }
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.newtype_variant || self.bytes.consume_struct_name(name)? {
            self.newtype_variant = false;

            visitor.visit_unit()
        } else {
            self.deserialize_unit(visitor)
        }
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if name == crate::value::raw::RAW_VALUE_TOKEN {
            let bytes_before = self.bytes.pre_ws_bytes();
            self.bytes.skip_ws()?;
            let _ignored = self.deserialize_ignored_any(serde::de::IgnoredAny)?;

            if self.bytes.skip_ws_check_unclosed_line_comment()? {
                return Err(Error::UnclosedLineComment);
            }

            let bytes_after = self.bytes.bytes();

            let ron_bytes = &bytes_before[..bytes_before.len() - bytes_after.len()];
            let ron_str = str::from_utf8(ron_bytes).map_err(Error::from)?;

            return visitor
                .visit_borrowed_str::<Error>(ron_str)
                .map_err(|_| Error::ExpectedRawValue);
        }

        if self.bytes.exts.contains(Extensions::UNWRAP_NEWTYPES) || self.newtype_variant {
            self.newtype_variant = false;

            return guard_recursion! { self => visitor.visit_newtype_struct(&mut *self) };
        }

        self.bytes.consume_struct_name(name)?;

        self.bytes.skip_ws()?;

        if self.bytes.consume("(") {
            self.bytes.skip_ws()?;
            let value = guard_recursion! { self => visitor.visit_newtype_struct(&mut *self)? };
            self.bytes.comma()?;

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                Err(Error::ExpectedStructLikeEnd)
            }
        } else if name.is_empty() {
            Err(Error::ExpectedStructLike)
        } else {
            Err(Error::ExpectedNamedStructLike(name))
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.newtype_variant = false;

        if self.bytes.consume("[") {
            let value = guard_recursion! { self =>
                visitor.visit_seq(CommaSeparated::new(Terminator::Seq, self))?
            };
            self.bytes.skip_ws()?;

            if self.bytes.consume("]") {
                Ok(value)
            } else {
                Err(Error::ExpectedArrayEnd)
            }
        } else {
            Err(Error::ExpectedArray)
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.newtype_variant || self.bytes.consume("(") {
            let old_newtype_variant = self.newtype_variant;
            self.newtype_variant = false;

            let value = guard_recursion! { self =>
                visitor.visit_seq(CommaSeparated::new(Terminator::Tuple, self))?
            };
            self.bytes.skip_ws()?;

            if old_newtype_variant || self.bytes.consume(")") {
                Ok(value)
            } else {
                Err(Error::ExpectedStructLikeEnd)
            }
        } else {
            Err(Error::ExpectedStructLike)
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if !self.newtype_variant {
            self.bytes.consume_struct_name(name)?;
        }

        self.deserialize_tuple(len, visitor).map_err(|e| match e {
            Error::ExpectedStructLike if !name.is_empty() => Error::ExpectedNamedStructLike(name),
            e => e,
        })
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Detect `#[serde(flatten)]` as a struct deserialised as a map
        const SERDE_FLATTEN_CANARY: &[u8] = b"struct ";

        struct VisitorExpecting<V>(V);
        impl<'de, V: Visitor<'de>> std::fmt::Display for VisitorExpecting<&'_ V> {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.0.expecting(fmt)
            }
        }

        self.newtype_variant = false;

        let mut canary_buffer = [0u8; SERDE_FLATTEN_CANARY.len()];
        let _ = write!(canary_buffer.as_mut(), "{}", VisitorExpecting(&visitor));
        let terminator = if canary_buffer == SERDE_FLATTEN_CANARY {
            Terminator::MapAsStruct
        } else {
            Terminator::Map
        };

        if self.bytes.consume("{") {
            let value = guard_recursion! { self =>
                visitor.visit_map(CommaSeparated::new(terminator, self))?
            };
            self.bytes.skip_ws()?;

            if self.bytes.consume("}") {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if !self.newtype_variant {
            self.bytes.consume_struct_name(name)?;
        }

        self.bytes.skip_ws()?;

        self.handle_struct_after_name(name, visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.newtype_variant = false;

        match guard_recursion! { self => visitor.visit_enum(Enum::new(self)) } {
            Ok(value) => Ok(value),
            Err(Error::NoSuchEnumVariant {
                expected,
                found,
                outer: None,
            }) if !name.is_empty() => Err(Error::NoSuchEnumVariant {
                expected,
                found,
                outer: Some(String::from(name)),
            }),
            Err(e) => Err(e),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let identifier = str::from_utf8(self.bytes.identifier()?).map_err(Error::from)?;

        self.last_identifier = Some(identifier);

        visitor.visit_borrowed_str(identifier)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Terminator {
    Map,
    MapAsStruct,
    Tuple,
    Struct,
    Seq,
}

impl Terminator {
    fn as_byte(self) -> u8 {
        match self {
            Terminator::Map | Terminator::MapAsStruct => b'}',
            Terminator::Tuple | Terminator::Struct => b')',
            Terminator::Seq => b']',
        }
    }
}

struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    terminator: Terminator,
    had_comma: bool,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(terminator: Terminator, de: &'a mut Deserializer<'de>) -> Self {
        CommaSeparated {
            de,
            terminator,
            had_comma: true,
        }
    }

    fn has_element(&mut self) -> Result<bool> {
        self.de.bytes.skip_ws()?;

        match (
            self.had_comma,
            self.de.bytes.peek_or_eof()? != self.terminator.as_byte(),
        ) {
            // Trailing comma, maybe has a next element
            (true, has_element) => Ok(has_element),
            // No trailing comma but terminator
            (false, false) => Ok(false),
            // No trailing comma or terminator
            (false, true) => Err(Error::ExpectedComma),
        }
    }
}

impl<'de, 'a> de::SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.has_element()? {
            let res = guard_recursion! { self.de => seed.deserialize(&mut *self.de)? };

            self.had_comma = self.de.bytes.comma()?;

            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
}

impl<'de, 'a> de::MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.has_element()? {
            match self.terminator {
                Terminator::Struct => guard_recursion! { self.de =>
                    seed.deserialize(&mut IdDeserializer::new(false, &mut *self.de)).map(Some)
                },
                Terminator::MapAsStruct => guard_recursion! { self.de =>
                    seed.deserialize(&mut IdDeserializer::new(true, &mut *self.de)).map(Some)
                },
                _ => guard_recursion! { self.de => seed.deserialize(&mut *self.de).map(Some) },
            }
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.bytes.skip_ws()?;

        if self.de.bytes.consume(":") {
            self.de.bytes.skip_ws()?;

            let res = guard_recursion! { self.de =>
                seed.deserialize(&mut TagDeserializer::new(&mut *self.de))?
            };

            self.had_comma = self.de.bytes.comma()?;

            Ok(res)
        } else {
            Err(Error::ExpectedMapColon)
        }
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.bytes.skip_ws()?;

        let value = guard_recursion! { self.de => seed.deserialize(&mut *self.de)? };

        Ok((value, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let newtype_variant = self.de.last_identifier;

        self.de.bytes.skip_ws()?;

        if self.de.bytes.consume("(") {
            self.de.bytes.skip_ws()?;

            self.de.newtype_variant = self
                .de
                .bytes
                .exts
                .contains(Extensions::UNWRAP_VARIANT_NEWTYPES);

            let val = guard_recursion! { self.de =>
                seed
                    .deserialize(&mut *self.de)
                    .map_err(|err| struct_error_name(err, newtype_variant))?
            };

            self.de.newtype_variant = false;

            self.de.bytes.comma()?;

            if self.de.bytes.consume(")") {
                Ok(val)
            } else {
                Err(Error::ExpectedStructLikeEnd)
            }
        } else {
            Err(Error::ExpectedStructLike)
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.bytes.skip_ws()?;

        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let struct_variant = self.de.last_identifier;

        self.de.bytes.skip_ws()?;

        self.de
            .handle_struct_after_name("", visitor)
            .map_err(|err| struct_error_name(err, struct_variant))
    }
}

fn struct_error_name(error: Error, name: Option<&str>) -> Error {
    match error {
        Error::NoSuchStructField {
            expected,
            found,
            outer: None,
        } => Error::NoSuchStructField {
            expected,
            found,
            outer: name.map(ToOwned::to_owned),
        },
        Error::MissingStructField { field, outer: None } => Error::MissingStructField {
            field,
            outer: name.map(ToOwned::to_owned),
        },
        Error::DuplicateStructField { field, outer: None } => Error::DuplicateStructField {
            field,
            outer: name.map(ToOwned::to_owned),
        },
        e => e,
    }
}

struct SerdeEnumContent<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    ident: Option<&'a str>,
}

impl<'de, 'a> de::MapAccess<'de> for SerdeEnumContent<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        self.ident
            .take()
            .map(|ident| seed.deserialize(serde::de::value::StrDeserializer::new(ident)))
            .transpose()
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.bytes.skip_ws()?;

        let old_serde_content_newtype = self.de.serde_content_newtype;
        self.de.serde_content_newtype = true;
        let result = seed.deserialize(&mut *self.de);
        self.de.serde_content_newtype = old_serde_content_newtype;

        result
    }
}
