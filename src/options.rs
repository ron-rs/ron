use std::io;

use serde::{de, ser};

use crate::de::Deserializer;
use crate::error::Result;
use crate::extensions::Extensions;
use crate::ser::{PrettyConfig, Serializer};

pub struct Options {
    default_extensions: Extensions,
}

impl Options {
    #[must_use]
    pub fn build() -> Self {
        Self {
            default_extensions: Extensions::empty(),
        }
    }

    #[must_use]
    pub fn with_default_extension(mut self, default_extension: Extensions) -> Self {
        self.default_extensions |= default_extension;
        self
    }

    #[must_use]
    pub fn without_default_extension(mut self, default_extension: Extensions) -> Self {
        self.default_extensions &= !default_extension;
        self
    }
}

impl Options {
    /// A convenience function for reading data from a reader
    /// and feeding into a deserializer.
    pub fn from_reader<R, T>(&self, mut rdr: R) -> Result<T>
    where
        R: io::Read,
        T: de::DeserializeOwned,
    {
        let mut bytes = Vec::new();
        rdr.read_to_end(&mut bytes)?;

        self.from_bytes(&bytes)
    }

    /// A convenience function for building a deserializer
    /// and deserializing a value of type `T` from a string.
    pub fn from_str<'a, T>(&self, s: &'a str) -> Result<T>
    where
        T: de::Deserialize<'a>,
    {
        self.from_bytes(s.as_bytes())
    }

    /// A convenience function for building a deserializer
    /// and deserializing a value of type `T` from bytes.
    pub fn from_bytes<'a, T>(&self, s: &'a [u8]) -> Result<T>
    where
        T: de::Deserialize<'a>,
    {
        let mut deserializer =
            Deserializer::from_bytes(s)?.with_default_extensions(self.default_extensions);
        let t = T::deserialize(&mut deserializer)?;

        deserializer.end()?;

        Ok(t)
    }

    /// Serializes `value` into `writer`
    pub fn to_writer<W, T>(&self, writer: W, value: &T) -> Result<()>
    where
        W: io::Write,
        T: ?Sized + ser::Serialize,
    {
        let mut s = Serializer::new_with_default_extensions(writer, None, self.default_extensions)?;
        value.serialize(&mut s)
    }

    /// Serializes `value` into `writer` in a pretty way.
    pub fn to_writer_pretty<W, T>(&self, writer: W, value: &T, config: PrettyConfig) -> Result<()>
    where
        W: io::Write,
        T: ?Sized + ser::Serialize,
    {
        let mut s =
            Serializer::new_with_default_extensions(writer, Some(config), self.default_extensions)?;
        value.serialize(&mut s)
    }

    /// Serializes `value` and returns it as string.
    ///
    /// This function does not generate any newlines or nice formatting;
    /// if you want that, you can use `to_string_pretty` instead.
    pub fn to_string<T>(&self, value: &T) -> Result<String>
    where
        T: ?Sized + ser::Serialize,
    {
        let mut output = Vec::new();
        let mut s =
            Serializer::new_with_default_extensions(&mut output, None, self.default_extensions)?;
        value.serialize(&mut s)?;
        Ok(String::from_utf8(output).expect("Ron should be utf-8"))
    }

    /// Serializes `value` in the recommended RON layout in a pretty way.
    pub fn to_string_pretty<T>(&self, value: &T, config: PrettyConfig) -> Result<String>
    where
        T: ?Sized + ser::Serialize,
    {
        let mut output = Vec::new();
        let mut s = Serializer::new_with_default_extensions(
            &mut output,
            Some(config),
            self.default_extensions,
        )?;
        value.serialize(&mut s)?;
        Ok(String::from_utf8(output).expect("Ron should be utf-8"))
    }
}
