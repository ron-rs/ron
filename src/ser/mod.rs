use std::{borrow::Cow, fmt};

use serde::{ser, ser::Serialize};
use serde_derive::{Deserialize, Serialize};
use unicode_ident::is_xid_continue;

use crate::{
    error::{Error, Result},
    extensions::Extensions,
    options::Options,
    parse::{is_ident_first_char, is_ident_raw_char, is_whitespace_char, LargeSInt, LargeUInt},
};

pub mod path_meta;

mod raw;
#[cfg(test)]
mod tests;
mod value;

/// Serializes `value` into `writer`.
///
/// This function does not generate any newlines or nice formatting;
/// if you want that, you can use [`to_writer_pretty`] instead.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: fmt::Write,
    T: ?Sized + Serialize,
{
    Options::default().to_writer(writer, value)
}

/// Serializes `value` into `writer` in a pretty way.
pub fn to_writer_pretty<W, T>(writer: W, value: &T, config: PrettyConfig) -> Result<()>
where
    W: fmt::Write,
    T: ?Sized + Serialize,
{
    Options::default().to_writer_pretty(writer, value, config)
}

/// Serializes `value` and returns it as string.
///
/// This function does not generate any newlines or nice formatting;
/// if you want that, you can use [`to_string_pretty`] instead.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + Serialize,
{
    Options::default().to_string(value)
}

/// Serializes `value` in the recommended RON layout in a pretty way.
pub fn to_string_pretty<T>(value: &T, config: PrettyConfig) -> Result<String>
where
    T: ?Sized + Serialize,
{
    Options::default().to_string_pretty(value, config)
}

/// Pretty serializer state
struct Pretty {
    indent: usize,
}

/// Pretty serializer configuration.
///
/// # Examples
///
/// ```
/// use ron::ser::PrettyConfig;
///
/// let my_config = PrettyConfig::new()
///     .depth_limit(4)
///     // definitely superior (okay, just joking)
///     .indentor("\t");
/// ```
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct PrettyConfig {
    /// Limit the pretty-ness up to the given depth.
    pub depth_limit: usize,
    /// New line string
    pub new_line: Cow<'static, str>,
    /// Indentation string
    pub indentor: Cow<'static, str>,
    /// Separator string
    pub separator: Cow<'static, str>,
    // Whether to emit struct names
    pub struct_names: bool,
    /// Separate tuple members with indentation
    pub separate_tuple_members: bool,
    /// Enumerate array items in comments
    pub enumerate_arrays: bool,
    /// Enable extensions. Only configures `implicit_some`,
    ///  `unwrap_newtypes`, and `unwrap_variant_newtypes` for now.
    pub extensions: Extensions,
    /// Enable compact arrays, which do not insert new lines and indentation
    ///  between the elements of an array
    pub compact_arrays: bool,
    /// Whether to serialize strings as escaped strings,
    ///  or fall back onto raw strings if necessary.
    pub escape_strings: bool,
    /// Enable compact structs, which do not insert new lines and indentation
    ///  between the fields of a struct
    pub compact_structs: bool,
    /// Enable compact maps, which do not insert new lines and indentation
    ///  between the entries of a struct
    pub compact_maps: bool,
    /// Enable explicit number type suffixes like `1u16`
    pub number_suffixes: bool,
    /// Additional path-based field metadata to serialize
    pub path_meta: Option<path_meta::Field>,
}

impl PrettyConfig {
    /// Creates a default [`PrettyConfig`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Limits the pretty-formatting based on the number of indentations.
    /// I.e., with a depth limit of 5, starting with an element of depth
    /// (indentation level) 6, everything will be put into the same line,
    /// without pretty formatting.
    ///
    /// Default: [`usize::MAX`]
    #[must_use]
    pub fn depth_limit(mut self, depth_limit: usize) -> Self {
        self.depth_limit = depth_limit;

        self
    }

    /// Configures the newlines used for serialization.
    ///
    /// Default: `\r\n` on Windows, `\n` otherwise
    #[must_use]
    pub fn new_line(mut self, new_line: impl Into<Cow<'static, str>>) -> Self {
        self.new_line = new_line.into();

        self
    }

    /// Configures the string sequence used for indentation.
    ///
    /// Default: 4 spaces
    #[must_use]
    pub fn indentor(mut self, indentor: impl Into<Cow<'static, str>>) -> Self {
        self.indentor = indentor.into();

        self
    }

    /// Configures the string sequence used to separate items inline.
    ///
    /// Default: 1 space
    #[must_use]
    pub fn separator(mut self, separator: impl Into<Cow<'static, str>>) -> Self {
        self.separator = separator.into();

        self
    }

    /// Configures whether to emit struct names.
    ///
    /// See also [`Extensions::EXPLICIT_STRUCT_NAMES`] for the extension equivalent.
    ///
    /// Default: `false`
    #[must_use]
    pub fn struct_names(mut self, struct_names: bool) -> Self {
        self.struct_names = struct_names;

        self
    }

    /// Configures whether tuples are single- or multi-line.
    /// If set to `true`, tuples will have their fields indented and in new
    /// lines. If set to `false`, tuples will be serialized without any
    /// newlines or indentations.
    ///
    /// Default: `false`
    #[must_use]
    pub fn separate_tuple_members(mut self, separate_tuple_members: bool) -> Self {
        self.separate_tuple_members = separate_tuple_members;

        self
    }

    /// Configures whether a comment shall be added to every array element,
    /// indicating the index.
    ///
    /// Default: `false`
    #[must_use]
    pub fn enumerate_arrays(mut self, enumerate_arrays: bool) -> Self {
        self.enumerate_arrays = enumerate_arrays;

        self
    }

    /// Configures whether every array should be a single line (`true`)
    /// or a multi line one (`false`).
    ///
    /// When `false`, `["a","b"]` will serialize to
    /// ```
    /// [
    ///   "a",
    ///   "b",
    /// ]
    /// # ;
    /// ```
    /// When `true`, `["a","b"]` will instead serialize to
    /// ```
    /// ["a","b"]
    /// # ;
    /// ```
    ///
    /// Default: `false`
    #[must_use]
    pub fn compact_arrays(mut self, compact_arrays: bool) -> Self {
        self.compact_arrays = compact_arrays;

        self
    }

    /// Configures extensions
    ///
    /// Default: [`Extensions::empty()`]
    #[must_use]
    pub fn extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;

        self
    }

    /// Configures whether strings should be serialized using escapes (true)
    /// or fall back to raw strings if the string contains a `"` (false).
    ///
    /// When `true`, `"a\nb"` will serialize to
    /// ```
    /// "a\nb"
    /// # ;
    /// ```
    /// When `false`, `"a\nb"` will instead serialize to
    /// ```
    /// "a
    /// b"
    /// # ;
    /// ```
    ///
    /// Default: `true`
    #[must_use]
    pub fn escape_strings(mut self, escape_strings: bool) -> Self {
        self.escape_strings = escape_strings;

        self
    }

    /// Configures whether every struct should be a single line (`true`)
    /// or a multi line one (`false`).
    ///
    /// When `false`, `Struct { a: 4, b: 2 }` will serialize to
    /// ```ignore
    /// Struct(
    ///     a: 4,
    ///     b: 2,
    /// )
    /// # ;
    /// ```
    /// When `true`, `Struct { a: 4, b: 2 }` will instead serialize to
    /// ```ignore
    /// Struct(a: 4, b: 2)
    /// # ;
    /// ```
    ///
    /// Default: `false`
    #[must_use]
    pub fn compact_structs(mut self, compact_structs: bool) -> Self {
        self.compact_structs = compact_structs;

        self
    }

    /// Configures whether every map should be a single line (`true`)
    /// or a multi line one (`false`).
    ///
    /// When `false`, a map with entries `{ "a": 4, "b": 2 }` will serialize to
    /// ```ignore
    /// {
    ///     "a": 4,
    ///     "b": 2,
    /// }
    /// # ;
    /// ```
    /// When `true`, a map with entries `{ "a": 4, "b": 2 }` will instead
    /// serialize to
    /// ```ignore
    /// {"a": 4, "b": 2}
    /// # ;
    /// ```
    ///
    /// Default: `false`
    #[must_use]
    pub fn compact_maps(mut self, compact_maps: bool) -> Self {
        self.compact_maps = compact_maps;

        self
    }

    /// Configures whether numbers should be printed without (`false`) or
    /// with (`true`) their explicit type suffixes.
    ///
    /// When `false`, the integer `12345u16` will serialize to
    /// ```ignore
    /// 12345
    /// # ;
    /// ```
    /// and the float `12345.6789f64` will serialize to
    /// ```ignore
    /// 12345.6789
    /// # ;
    /// ```
    /// When `true`, the integer `12345u16` will serialize to
    /// ```ignore
    /// 12345u16
    /// # ;
    /// ```
    /// and the float `12345.6789f64` will serialize to
    /// ```ignore
    /// 12345.6789f64
    /// # ;
    /// ```
    ///
    /// Default: `false`
    #[must_use]
    pub fn number_suffixes(mut self, number_suffixes: bool) -> Self {
        self.number_suffixes = number_suffixes;

        self
    }
}

impl Default for PrettyConfig {
    fn default() -> Self {
        PrettyConfig {
            depth_limit: usize::MAX,
            new_line: if cfg!(not(target_os = "windows")) {
                Cow::Borrowed("\n")
            } else {
                Cow::Borrowed("\r\n") // GRCOV_EXCL_LINE
            },
            indentor: Cow::Borrowed("    "),
            separator: Cow::Borrowed(" "),
            struct_names: false,
            separate_tuple_members: false,
            enumerate_arrays: false,
            extensions: Extensions::empty(),
            compact_arrays: false,
            escape_strings: true,
            compact_structs: false,
            compact_maps: false,
            number_suffixes: false,
            path_meta: None,
        }
    }
}

/// The RON serializer.
///
/// You can just use [`to_string`] for deserializing a value.
/// If you want it pretty-printed, take a look at [`to_string_pretty`].
pub struct Serializer<W: fmt::Write> {
    output: W,
    pretty: Option<(PrettyConfig, Pretty)>,
    default_extensions: Extensions,
    is_empty: Option<bool>,
    newtype_variant: bool,
    recursion_limit: Option<usize>,
    // Tracks the number of opened implicit `Some`s, set to 0 on backtracking
    implicit_some_depth: usize,
}

fn indent<W: fmt::Write>(output: &mut W, config: &PrettyConfig, pretty: &Pretty) -> fmt::Result {
    if pretty.indent <= config.depth_limit {
        for _ in 0..pretty.indent {
            output.write_str(&config.indentor)?;
        }
    }
    Ok(())
}

impl<W: fmt::Write> Serializer<W> {
    /// Creates a new [`Serializer`].
    ///
    /// Most of the time you can just use [`to_string`] or
    /// [`to_string_pretty`].
    pub fn new(writer: W, config: Option<PrettyConfig>) -> Result<Self> {
        Self::with_options(writer, config, &Options::default())
    }

    /// Creates a new [`Serializer`].
    ///
    /// Most of the time you can just use [`to_string`] or
    /// [`to_string_pretty`].
    pub fn with_options(
        mut writer: W,
        config: Option<PrettyConfig>,
        options: &Options,
    ) -> Result<Self> {
        if let Some(conf) = &config {
            if !conf.new_line.chars().all(is_whitespace_char) {
                return Err(Error::Message(String::from(
                    "Invalid non-whitespace `PrettyConfig::new_line`",
                )));
            }
            if !conf.indentor.chars().all(is_whitespace_char) {
                return Err(Error::Message(String::from(
                    "Invalid non-whitespace `PrettyConfig::indentor`",
                )));
            }
            if !conf.separator.chars().all(is_whitespace_char) {
                return Err(Error::Message(String::from(
                    "Invalid non-whitespace `PrettyConfig::separator`",
                )));
            }

            let non_default_extensions = !options.default_extensions;

            for (extension_name, _) in (non_default_extensions & conf.extensions).iter_names() {
                write!(writer, "#![enable({})]", extension_name.to_lowercase())?;
                writer.write_str(&conf.new_line)?;
            }
        };
        Ok(Serializer {
            output: writer,
            pretty: config.map(|conf| (conf, Pretty { indent: 0 })),
            default_extensions: options.default_extensions,
            is_empty: None,
            newtype_variant: false,
            recursion_limit: options.recursion_limit,
            implicit_some_depth: 0,
        })
    }

    fn separate_tuple_members(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |(ref config, _)| config.separate_tuple_members)
    }

    fn compact_arrays(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |(ref config, _)| config.compact_arrays)
    }

    fn compact_structs(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |(ref config, _)| config.compact_structs)
    }

    fn compact_maps(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |(ref config, _)| config.compact_maps)
    }

    fn number_suffixes(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |(ref config, _)| config.number_suffixes)
    }

    fn extensions(&self) -> Extensions {
        self.default_extensions
            | self
                .pretty
                .as_ref()
                .map_or(Extensions::empty(), |(ref config, _)| config.extensions)
    }

    fn escape_strings(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(true, |(ref config, _)| config.escape_strings)
    }

    fn start_indent(&mut self) -> Result<()> {
        if let Some((ref config, ref mut pretty)) = self.pretty {
            pretty.indent += 1;
            if pretty.indent <= config.depth_limit {
                let is_empty = self.is_empty.unwrap_or(false);

                if !is_empty {
                    self.output.write_str(&config.new_line)?;
                }
            }
        }
        Ok(())
    }

    fn indent(&mut self) -> fmt::Result {
        if let Some((ref config, ref pretty)) = self.pretty {
            indent(&mut self.output, config, pretty)?;
        }
        Ok(())
    }

    fn end_indent(&mut self) -> fmt::Result {
        if let Some((ref config, ref mut pretty)) = self.pretty {
            if pretty.indent <= config.depth_limit {
                let is_empty = self.is_empty.unwrap_or(false);

                if !is_empty {
                    for _ in 1..pretty.indent {
                        self.output.write_str(&config.indentor)?;
                    }
                }
            }
            pretty.indent -= 1;

            self.is_empty = None;
        }
        Ok(())
    }

    fn serialize_escaped_str(&mut self, value: &str) -> fmt::Result {
        self.output.write_char('"')?;
        let mut scalar = [0u8; 4];
        for c in value.chars().flat_map(char::escape_debug) {
            self.output.write_str(c.encode_utf8(&mut scalar))?;
        }
        self.output.write_char('"')?;
        Ok(())
    }

    fn serialize_unescaped_or_raw_str(&mut self, value: &str) -> fmt::Result {
        if value.contains('"') || value.contains('\\') {
            let (_, num_consecutive_hashes) =
                value.chars().fold((0, 0), |(count, max), c| match c {
                    '#' => (count + 1, max.max(count + 1)),
                    _ => (0_usize, max),
                });
            let hashes: String = "#".repeat(num_consecutive_hashes + 1);
            self.output.write_char('r')?;
            self.output.write_str(&hashes)?;
            self.output.write_char('"')?;
            self.output.write_str(value)?;
            self.output.write_char('"')?;
            self.output.write_str(&hashes)?;
        } else {
            self.output.write_char('"')?;
            self.output.write_str(value)?;
            self.output.write_char('"')?;
        }
        Ok(())
    }

    fn serialize_escaped_byte_str(&mut self, value: &[u8]) -> fmt::Result {
        self.output.write_str("b\"")?;
        for c in value.iter().flat_map(|c| std::ascii::escape_default(*c)) {
            self.output.write_char(char::from(c))?;
        }
        self.output.write_char('"')?;
        Ok(())
    }

    fn serialize_unescaped_or_raw_byte_str(&mut self, value: &str) -> fmt::Result {
        if value.contains('"') || value.contains('\\') {
            let (_, num_consecutive_hashes) =
                value.chars().fold((0, 0), |(count, max), c| match c {
                    '#' => (count + 1, max.max(count + 1)),
                    _ => (0_usize, max),
                });
            let hashes: String = "#".repeat(num_consecutive_hashes + 1);
            self.output.write_str("br")?;
            self.output.write_str(&hashes)?;
            self.output.write_char('"')?;
            self.output.write_str(value)?;
            self.output.write_char('"')?;
            self.output.write_str(&hashes)?;
        } else {
            self.output.write_str("b\"")?;
            self.output.write_str(value)?;
            self.output.write_char('"')?;
        }
        Ok(())
    }

    fn serialize_sint(&mut self, value: impl Into<LargeSInt>, suffix: &str) -> Result<()> {
        // TODO optimize
        write!(self.output, "{}", value.into())?;

        if self.number_suffixes() {
            write!(self.output, "{}", suffix)?;
        }

        Ok(())
    }

    fn serialize_uint(&mut self, value: impl Into<LargeUInt>, suffix: &str) -> Result<()> {
        // TODO optimize
        write!(self.output, "{}", value.into())?;

        if self.number_suffixes() {
            write!(self.output, "{}", suffix)?;
        }

        Ok(())
    }

    fn write_identifier(&mut self, name: &str) -> Result<()> {
        self.validate_identifier(name)?;
        let mut chars = name.chars();
        if !chars.next().map_or(false, is_ident_first_char)
            || !chars.all(is_xid_continue)
            || [
                "true", "false", "Some", "None", "inf", "inff32", "inff64", "NaN", "NaNf32",
                "NaNf64",
            ]
            .contains(&name)
        {
            self.output.write_str("r#")?;
        }
        self.output.write_str(name)?;
        Ok(())
    }

    #[allow(clippy::unused_self)]
    fn validate_identifier(&self, name: &str) -> Result<()> {
        if name.is_empty() || !name.chars().all(is_ident_raw_char) {
            return Err(Error::InvalidIdentifier(name.into()));
        }
        Ok(())
    }

    /// Checks if struct names should be emitted
    ///
    /// Note that when using the `explicit_struct_names` extension, this method will use an OR operation on the extension and the [`PrettyConfig::struct_names`] option. See also [`Extensions::EXPLICIT_STRUCT_NAMES`] for the extension equivalent.
    fn struct_names(&self) -> bool {
        self.extensions()
            .contains(Extensions::EXPLICIT_STRUCT_NAMES)
            || self
                .pretty
                .as_ref()
                .map_or(false, |(pc, _)| pc.struct_names)
    }
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

impl<'a, W: fmt::Write> ser::Serializer for &'a mut Serializer<W> {
    type Error = Error;
    type Ok = ();
    type SerializeMap = Compound<'a, W>;
    type SerializeSeq = Compound<'a, W>;
    type SerializeStruct = Compound<'a, W>;
    type SerializeStructVariant = Compound<'a, W>;
    type SerializeTuple = Compound<'a, W>;
    type SerializeTupleStruct = Compound<'a, W>;
    type SerializeTupleVariant = Compound<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output.write_str(if v { "true" } else { "false" })?;
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_sint(v, "i8")
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_sint(v, "i16")
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_sint(v, "i32")
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_sint(v, "i64")
    }

    #[cfg(feature = "integer128")]
    fn serialize_i128(self, v: i128) -> Result<()> {
        self.serialize_sint(v, "i128")
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_uint(v, "u8")
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_uint(v, "u16")
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_uint(v, "u32")
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.serialize_uint(v, "u64")
    }

    #[cfg(feature = "integer128")]
    fn serialize_u128(self, v: u128) -> Result<()> {
        self.serialize_uint(v, "u128")
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        if v.is_nan() && v.is_sign_negative() {
            write!(self.output, "-")?;
        }

        write!(self.output, "{}", v)?;

        if v.fract() == 0.0 {
            write!(self.output, ".0")?;
        }

        if self.number_suffixes() {
            write!(self.output, "f32")?;
        }

        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        if v.is_nan() && v.is_sign_negative() {
            write!(self.output, "-")?;
        }

        write!(self.output, "{}", v)?;

        if v.fract() == 0.0 {
            write!(self.output, ".0")?;
        }

        if self.number_suffixes() {
            write!(self.output, "f64")?;
        }

        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.output.write_char('\'')?;
        if v == '\\' || v == '\'' {
            self.output.write_char('\\')?;
        }
        write!(self.output, "{}", v)?;
        self.output.write_char('\'')?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        if self.escape_strings() {
            self.serialize_escaped_str(v)?;
        } else {
            self.serialize_unescaped_or_raw_str(v)?;
        }

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        // We need to fall back to escaping if the byte string would be invalid UTF-8
        if !self.escape_strings() {
            if let Ok(v) = std::str::from_utf8(v) {
                return self
                    .serialize_unescaped_or_raw_byte_str(v)
                    .map_err(Error::from);
            }
        }

        self.serialize_escaped_byte_str(v)?;

        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        // We no longer need to keep track of the depth
        let implicit_some_depth = self.implicit_some_depth;
        self.implicit_some_depth = 0;

        for _ in 0..implicit_some_depth {
            self.output.write_str("Some(")?;
        }
        self.output.write_str("None")?;
        for _ in 0..implicit_some_depth {
            self.output.write_char(')')?;
        }

        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let implicit_some = self.extensions().contains(Extensions::IMPLICIT_SOME);
        if implicit_some {
            self.implicit_some_depth += 1;
        } else {
            self.newtype_variant = self
                .extensions()
                .contains(Extensions::UNWRAP_VARIANT_NEWTYPES);
            self.output.write_str("Some(")?;
        }
        guard_recursion! { self => value.serialize(&mut *self)? };
        if implicit_some {
            self.implicit_some_depth = 0;
        } else {
            self.output.write_char(')')?;
            self.newtype_variant = false;
        }

        Ok(())
    }

    fn serialize_unit(self) -> Result<()> {
        if !self.newtype_variant {
            self.output.write_str("()")?;
        }

        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        if self.struct_names() && !self.newtype_variant {
            self.write_identifier(name)?;

            Ok(())
        } else {
            self.validate_identifier(name)?;
            self.serialize_unit()
        }
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.validate_identifier(name)?;
        self.write_identifier(variant)?;

        Ok(())
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if name == crate::value::raw::RAW_VALUE_TOKEN {
            let implicit_some_depth = self.implicit_some_depth;
            self.implicit_some_depth = 0;

            for _ in 0..implicit_some_depth {
                self.output.write_str("Some(")?;
            }

            guard_recursion! { self => value.serialize(raw::Serializer::new(self)) }?;

            for _ in 0..implicit_some_depth {
                self.output.write_char(')')?;
            }

            return Ok(());
        }

        if self.extensions().contains(Extensions::UNWRAP_NEWTYPES) || self.newtype_variant {
            self.newtype_variant = false;

            self.validate_identifier(name)?;

            return guard_recursion! { self => value.serialize(&mut *self) };
        }

        if self.struct_names() {
            self.write_identifier(name)?;
        } else {
            self.validate_identifier(name)?;
        }

        self.implicit_some_depth = 0;

        self.output.write_char('(')?;
        guard_recursion! { self => value.serialize(&mut *self)? };
        self.output.write_char(')')?;

        Ok(())
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.validate_identifier(name)?;
        self.write_identifier(variant)?;
        self.output.write_char('(')?;

        self.newtype_variant = self
            .extensions()
            .contains(Extensions::UNWRAP_VARIANT_NEWTYPES);
        self.implicit_some_depth = 0;

        guard_recursion! { self => value.serialize(&mut *self)? };

        self.newtype_variant = false;

        self.output.write_char(')')?;
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.newtype_variant = false;
        self.implicit_some_depth = 0;

        self.output.write_char('[')?;

        if !self.compact_arrays() {
            if let Some(len) = len {
                self.is_empty = Some(len == 0);
            }

            self.start_indent()?;
        }

        Ok(Compound::new(self, false))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        let old_newtype_variant = self.newtype_variant;
        self.newtype_variant = false;
        self.implicit_some_depth = 0;

        if !old_newtype_variant {
            self.output.write_char('(')?;
        }

        if self.separate_tuple_members() {
            self.is_empty = Some(len == 0);

            self.start_indent()?;
        }

        Ok(Compound::new(self, old_newtype_variant))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        if self.struct_names() && !self.newtype_variant {
            self.write_identifier(name)?;
        } else {
            self.validate_identifier(name)?;
        }

        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.newtype_variant = false;
        self.implicit_some_depth = 0;

        self.validate_identifier(name)?;
        self.write_identifier(variant)?;
        self.output.write_char('(')?;

        if self.separate_tuple_members() {
            self.is_empty = Some(len == 0);

            self.start_indent()?;
        }

        Ok(Compound::new(self, false))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        self.newtype_variant = false;
        self.implicit_some_depth = 0;

        self.output.write_char('{')?;

        if !self.compact_maps() {
            if let Some(len) = len {
                self.is_empty = Some(len == 0);
            }

            self.start_indent()?;
        }

        Ok(Compound::new(self, false))
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        let old_newtype_variant = self.newtype_variant;
        self.newtype_variant = false;
        self.implicit_some_depth = 0;

        if old_newtype_variant {
            self.validate_identifier(name)?;
        } else {
            if self.struct_names() {
                self.write_identifier(name)?;
            } else {
                self.validate_identifier(name)?;
            }
            self.output.write_char('(')?;
        }

        if !self.compact_structs() {
            self.is_empty = Some(len == 0);
            self.start_indent()?;
        }

        Ok(Compound::new(self, old_newtype_variant))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.newtype_variant = false;
        self.implicit_some_depth = 0;

        self.validate_identifier(name)?;
        self.write_identifier(variant)?;
        self.output.write_char('(')?;

        if !self.compact_structs() {
            self.is_empty = Some(len == 0);
            self.start_indent()?;
        }

        Ok(Compound::new(self, false))
    }
}

enum State {
    First,
    Rest,
}

#[doc(hidden)]
pub struct Compound<'a, W: fmt::Write> {
    ser: &'a mut Serializer<W>,
    state: State,
    newtype_variant: bool,
    sequence_index: usize,
}

impl<'a, W: fmt::Write> Compound<'a, W> {
    fn new(ser: &'a mut Serializer<W>, newtype_variant: bool) -> Self {
        Compound {
            ser,
            state: State::First,
            newtype_variant,
            sequence_index: 0,
        }
    }
}

impl<'a, W: fmt::Write> Drop for Compound<'a, W> {
    fn drop(&mut self) {
        if let Some(limit) = &mut self.ser.recursion_limit {
            *limit = limit.saturating_add(1);
        }
    }
}

impl<'a, W: fmt::Write> ser::SerializeSeq for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_char(',')?;
            if let Some((ref config, ref mut pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && !config.compact_arrays {
                    self.ser.output.write_str(&config.new_line)?;
                } else {
                    self.ser.output.write_str(&config.separator)?;
                }
            }
        }

        if !self.ser.compact_arrays() {
            self.ser.indent()?;
        }

        if let Some((ref mut config, ref mut pretty)) = self.ser.pretty {
            if pretty.indent <= config.depth_limit && config.enumerate_arrays {
                write!(self.ser.output, "/*[{}]*/ ", self.sequence_index)?;
                self.sequence_index += 1;
            }
        }

        guard_recursion! { self.ser => value.serialize(&mut *self.ser)? };

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref mut pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && !config.compact_arrays {
                    self.ser.output.write_char(',')?;
                    self.ser.output.write_str(&config.new_line)?;
                }
            }
        }

        if !self.ser.compact_arrays() {
            self.ser.end_indent()?;
        }

        // seq always disables `self.newtype_variant`
        self.ser.output.write_char(']')?;
        Ok(())
    }
}

impl<'a, W: fmt::Write> ser::SerializeTuple for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_char(',')?;
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && self.ser.separate_tuple_members() {
                    self.ser.output.write_str(&config.new_line)?;
                } else {
                    self.ser.output.write_str(&config.separator)?;
                }
            }
        }

        if self.ser.separate_tuple_members() {
            self.ser.indent()?;
        }

        guard_recursion! { self.ser => value.serialize(&mut *self.ser)? };

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if self.ser.separate_tuple_members() && pretty.indent <= config.depth_limit {
                    self.ser.output.write_char(',')?;
                    self.ser.output.write_str(&config.new_line)?;
                }
            }
        }
        if self.ser.separate_tuple_members() {
            self.ser.end_indent()?;
        }

        if !self.newtype_variant {
            self.ser.output.write_char(')')?;
        }

        Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a, W: fmt::Write> ser::SerializeTupleStruct for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeTuple::end(self)
    }
}

impl<'a, W: fmt::Write> ser::SerializeTupleVariant for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeTuple::end(self)
    }
}

impl<'a, W: fmt::Write> ser::SerializeMap for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_char(',')?;

            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && !config.compact_maps {
                    self.ser.output.write_str(&config.new_line)?;
                } else {
                    self.ser.output.write_str(&config.separator)?;
                }
            }
        }

        if !self.ser.compact_maps() {
            self.ser.indent()?;
        }

        guard_recursion! { self.ser => key.serialize(&mut *self.ser) }
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.ser.output.write_char(':')?;

        if let Some((ref config, _)) = self.ser.pretty {
            self.ser.output.write_str(&config.separator)?;
        }

        guard_recursion! { self.ser => value.serialize(&mut *self.ser)? };

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && !config.compact_maps {
                    self.ser.output.write_char(',')?;
                    self.ser.output.write_str(&config.new_line)?;
                }
            }
        }

        if !self.ser.compact_maps() {
            self.ser.end_indent()?;
        }

        // map always disables `self.newtype_variant`
        self.ser.output.write_char('}')?;
        Ok(())
    }
}

impl<'a, W: fmt::Write> ser::SerializeStruct for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut restore_field = self.ser.pretty.as_mut().and_then(|(config, _)| {
            config.path_meta.take().map(|mut field| {
                if let Some(fields) = field.fields_mut() {
                    config.path_meta = fields.remove(key);
                }
                field
            })
        });

        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_char(',')?;

            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && !config.compact_structs {
                    self.ser.output.write_str(&config.new_line)?;
                } else {
                    self.ser.output.write_str(&config.separator)?;
                }
            }
        }

        if !self.ser.compact_structs() {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                indent(&mut self.ser.output, config, pretty)?;

                if let Some(ref field) = config.path_meta {
                    for doc_line in field.doc().lines() {
                        self.ser.output.write_str("/// ")?;
                        self.ser.output.write_str(doc_line)?;
                        self.ser.output.write_char('\n')?;
                        indent(&mut self.ser.output, config, pretty)?;
                    }
                }
            }
        }

        self.ser.write_identifier(key)?;
        self.ser.output.write_char(':')?;

        if let Some((ref config, _)) = self.ser.pretty {
            self.ser.output.write_str(&config.separator)?;
        }

        guard_recursion! { self.ser => value.serialize(&mut *self.ser)? };

        if let Some((ref mut config, _)) = self.ser.pretty {
            std::mem::swap(&mut config.path_meta, &mut restore_field);

            if let Some(ref mut field) = config.path_meta {
                if let Some(fields) = field.fields_mut() {
                    if let Some(restore_field) = restore_field {
                        fields.insert(key, restore_field);
                    }
                }
            }
        };

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && !config.compact_structs {
                    self.ser.output.write_char(',')?;
                    self.ser.output.write_str(&config.new_line)?;
                }
            }
        }

        if !self.ser.compact_structs() {
            self.ser.end_indent()?;
        }

        if !self.newtype_variant {
            self.ser.output.write_char(')')?;
        }

        Ok(())
    }
}

impl<'a, W: fmt::Write> ser::SerializeStructVariant for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeStruct::end(self)
    }
}
