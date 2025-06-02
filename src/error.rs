use alloc::string::{String, ToString};
use core::{
    fmt,
    str::{self, Utf8Error},
};

use serde::{
    de,
    ser::{self, StdError},
};
use unicode_ident::is_xid_continue;

use crate::parse::{is_ident_first_char, is_ident_raw_char};

#[cfg(feature = "std")]
use std::io;

/// This type represents all possible errors that can occur when
/// serializing or deserializing RON data.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpannedError {
    pub code: Error,
    pub span: Span,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
pub type SpannedResult<T> = core::result::Result<T, SpannedError>;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    Fmt,
    Io(String),
    Message(String),
    #[deprecated(
        since = "0.9.0",
        note = "ambiguous base64 byte strings are replaced by strongly typed Rusty b\"byte strings\""
    )]
    Base64Error(base64::DecodeError),
    Eof,
    ExpectedArray,
    ExpectedArrayEnd,
    ExpectedAttribute,
    ExpectedAttributeEnd,
    ExpectedBoolean,
    ExpectedComma,
    ExpectedChar,
    ExpectedByteLiteral,
    ExpectedFloat,
    FloatUnderscore,
    ExpectedInteger,
    ExpectedOption,
    ExpectedOptionEnd,
    ExpectedMap,
    ExpectedMapColon,
    ExpectedMapEnd,
    ExpectedDifferentStructName {
        expected: &'static str,
        found: String,
    },
    ExpectedStructLike,
    ExpectedNamedStructLike(&'static str),
    ExpectedStructLikeEnd,
    ExpectedUnit,
    ExpectedString,
    ExpectedByteString,
    ExpectedStringEnd,
    ExpectedIdentifier,

    InvalidEscape(&'static str),

    IntegerOutOfBounds,
    InvalidIntegerDigit {
        digit: char,
        base: u8,
    },

    NoSuchExtension(String),

    UnclosedBlockComment,
    UnclosedLineComment,
    UnderscoreAtBeginning,
    UnexpectedChar(char),

    Utf8Error(Utf8Error),
    TrailingCharacters,

    InvalidValueForType {
        expected: String,
        found: String,
    },
    ExpectedDifferentLength {
        expected: String,
        found: usize,
    },
    NoSuchEnumVariant {
        expected: &'static [&'static str],
        found: String,
        outer: Option<String>,
    },
    NoSuchStructField {
        expected: &'static [&'static str],
        found: String,
        outer: Option<String>,
    },
    MissingStructField {
        field: &'static str,
        outer: Option<String>,
    },
    DuplicateStructField {
        field: &'static str,
        outer: Option<String>,
    },
    InvalidIdentifier(String),
    SuggestRawIdentifier(String),
    ExpectedRawValue,
    ExceededRecursionLimit,
    ExpectedStructName(String),
}

impl fmt::Display for SpannedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.span, self.code)
    }
}

impl fmt::Display for Error {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Fmt => f.write_str("Formatting RON failed"),
            Error::Io(ref s) | Error::Message(ref s) => f.write_str(s),
            #[allow(deprecated)]
            Error::Base64Error(ref e) => write!(f, "Invalid base64: {}", e),
            Error::Eof => f.write_str("Unexpected end of RON"),
            Error::ExpectedArray => f.write_str("Expected opening `[`"),
            Error::ExpectedArrayEnd => f.write_str("Expected closing `]`"),
            Error::ExpectedAttribute => f.write_str("Expected an `#![enable(...)]` attribute"),
            Error::ExpectedAttributeEnd => {
                f.write_str("Expected closing `)]` after the enable attribute")
            }
            Error::ExpectedBoolean => f.write_str("Expected boolean"),
            Error::ExpectedComma => f.write_str("Expected comma"),
            Error::ExpectedChar => f.write_str("Expected char"),
            Error::ExpectedByteLiteral => f.write_str("Expected byte literal"),
            Error::ExpectedFloat => f.write_str("Expected float"),
            Error::FloatUnderscore => f.write_str("Unexpected underscore in float"),
            Error::ExpectedInteger => f.write_str("Expected integer"),
            Error::ExpectedOption => f.write_str("Expected option"),
            Error::ExpectedOptionEnd | Error::ExpectedStructLikeEnd => {
                f.write_str("Expected closing `)`")
            }
            Error::ExpectedMap => f.write_str("Expected opening `{`"),
            Error::ExpectedMapColon => f.write_str("Expected colon"),
            Error::ExpectedMapEnd => f.write_str("Expected closing `}`"),
            Error::ExpectedDifferentStructName {
                expected,
                ref found,
            } => write!(
                f,
                "Expected struct {} but found {}",
                Identifier(expected),
                Identifier(found)
            ),
            Error::ExpectedStructLike => f.write_str("Expected opening `(`"),
            Error::ExpectedNamedStructLike(name) => {
                if name.is_empty() {
                    f.write_str("Expected only opening `(`, no name, for un-nameable struct")
                } else {
                    write!(f, "Expected opening `(` for struct {}", Identifier(name))
                }
            }
            Error::ExpectedUnit => f.write_str("Expected unit"),
            Error::ExpectedString => f.write_str("Expected string"),
            Error::ExpectedByteString => f.write_str("Expected byte string"),
            Error::ExpectedStringEnd => f.write_str("Expected end of string"),
            Error::ExpectedIdentifier => f.write_str("Expected identifier"),
            Error::InvalidEscape(s) => f.write_str(s),
            Error::IntegerOutOfBounds => f.write_str("Integer is out of bounds"),
            Error::InvalidIntegerDigit { digit, base } => {
                write!(f, "Invalid digit {:?} for base {} integers", digit, base)
            }
            Error::NoSuchExtension(ref name) => {
                write!(f, "No RON extension named {}", Identifier(name))
            }
            Error::Utf8Error(ref e) => fmt::Display::fmt(e, f),
            Error::UnclosedBlockComment => f.write_str("Unclosed block comment"),
            Error::UnclosedLineComment => f.write_str(
                "`ron::value::RawValue` cannot end in unclosed line comment, \
                try using a block comment or adding a newline",
            ),
            Error::UnderscoreAtBeginning => {
                f.write_str("Unexpected leading underscore in a number")
            }
            Error::UnexpectedChar(c) => write!(f, "Unexpected char {:?}", c),
            Error::TrailingCharacters => f.write_str("Non-whitespace trailing characters"),
            Error::InvalidValueForType {
                ref expected,
                ref found,
            } => {
                write!(f, "Expected {} but found {} instead", expected, found)
            }
            Error::ExpectedDifferentLength {
                ref expected,
                found,
            } => {
                write!(f, "Expected {} but found ", expected)?;

                match found {
                    0 => f.write_str("zero elements")?,
                    1 => f.write_str("one element")?,
                    n => write!(f, "{} elements", n)?,
                }

                f.write_str(" instead")
            }
            Error::NoSuchEnumVariant {
                expected,
                ref found,
                ref outer,
            } => {
                f.write_str("Unexpected ")?;

                if outer.is_none() {
                    f.write_str("enum ")?;
                }

                write!(f, "variant named {}", Identifier(found))?;

                if let Some(outer) = outer {
                    write!(f, " in enum {}", Identifier(outer))?;
                }

                write!(
                    f,
                    ", {}",
                    OneOf {
                        alts: expected,
                        none: "variants"
                    }
                )
            }
            Error::NoSuchStructField {
                expected,
                ref found,
                ref outer,
            } => {
                write!(f, "Unexpected field named {}", Identifier(found))?;

                if let Some(outer) = outer {
                    write!(f, " in {}", Identifier(outer))?;
                }

                write!(
                    f,
                    ", {}",
                    OneOf {
                        alts: expected,
                        none: "fields"
                    }
                )
            }
            Error::MissingStructField { field, ref outer } => {
                write!(f, "Unexpected missing field named {}", Identifier(field))?;

                match outer {
                    Some(outer) => write!(f, " in {}", Identifier(outer)),
                    None => Ok(()),
                }
            }
            Error::DuplicateStructField { field, ref outer } => {
                write!(f, "Unexpected duplicate field named {}", Identifier(field))?;

                match outer {
                    Some(outer) => write!(f, " in {}", Identifier(outer)),
                    None => Ok(()),
                }
            }
            Error::InvalidIdentifier(ref invalid) => write!(f, "Invalid identifier {:?}", invalid),
            Error::SuggestRawIdentifier(ref identifier) => write!(
                f,
                "Found invalid std identifier {:?}, try the raw identifier `r#{}` instead",
                identifier, identifier
            ),
            Error::ExpectedRawValue => f.write_str("Expected a `ron::value::RawValue`"),
            Error::ExceededRecursionLimit => f.write_str(
                "Exceeded recursion limit, try increasing `ron::Options::recursion_limit` \
                and using `serde_stacker` to protect against a stack overflow",
            ),
            Error::ExpectedStructName(ref name) => write!(
                f,
                "Expected the explicit struct name {}, but none was found",
                Identifier(name)
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub(crate) fn from_src_end(src: &str) -> Position {
        let line = 1 + src.chars().filter(|&c| c == '\n').count();
        let col = 1 + src.chars().rev().take_while(|&c| c != '\n').count();

        Self { line, col }
    }
}

#[cfg(test)]
impl Position {
    /// Given a Position and a string, return the 0-indexed grapheme index into the
    /// string at that position, or [None] if the Position is out of bounds of the string.
    pub fn grapheme_index(&self, s: &str) -> Option<usize> {
        use unicode_segmentation::UnicodeSegmentation;
        let mut line_no = 1;
        let mut col_no = 1;

        if (self.line, self.col) == (1, 1) {
            return Some(0);
        }

        let mut i = 0;

        // Slightly non-intuitive arithmetic: a zero-length string at line 1, col 1 -> 0

        if line_no == self.line {
            if col_no == self.col {
                return Some(i);
            }
        }

        for ch in s.graphemes(true) {
            if line_no == self.line {
                if col_no == self.col {
                    return Some(i);
                }
            }

            // "\n" and "\r\n" each come through the iterator as a single grapheme
            if matches!(ch, "\n" | "\r\n") {
                line_no += 1;
                col_no = 1;
            } else {
                col_no += 1;
            }

            i += 1;
        }

        // ...and a string of length 7 at line 1, col 8 -> 7
        if line_no == self.line {
            if col_no == self.col {
                return Some(i);
            }
        }

        None
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Spans select a range of text between two positions.
/// Spans are used in [SpannedError] to indicate the start and end positions
/// of the parser cursor before and after it encountered an error in parsing.
pub struct Span {
    pub start: Position,
    pub end: Position,
}

#[cfg(test)]
impl Span {
    /// Given a Span and a string, form the resulting string selected exclusively (as in \[start..end\]) by the Span
    /// or [None] if the span is out of bounds of the string at either end.
    pub fn substring_exclusive<'a>(&self, s: &'a str) -> Option<String> {
        use alloc::vec::Vec;
        use unicode_segmentation::UnicodeSegmentation;

        if let (Some(start), Some(end)) = (self.start.grapheme_index(s), self.end.grapheme_index(s))
        {
            Some(s.graphemes(true).collect::<Vec<&str>>()[start..end].concat())
        } else {
            None
        }
    }

    /// Given a Span and a string, form the resulting string selected inclusively (as in \[start..=end\]) by the Span
    /// or [None] if the span is out of bounds of the string at either end.
    pub fn substring_inclusive<'a>(&self, s: &'a str) -> Option<String> {
        use alloc::vec::Vec;
        use unicode_segmentation::UnicodeSegmentation;

        if let (Some(start), Some(end)) = (self.start.grapheme_index(s), self.end.grapheme_index(s))
        {
            Some(s.graphemes(true).collect::<Vec<&str>>()[start..=end].concat())
        } else {
            None
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

impl ser::Error for Error {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }

    #[cold]
    fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        // Invalid type and invalid value are merged given their similarity in ron
        Self::invalid_value(unexp, exp)
    }

    #[cold]
    fn invalid_value(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        struct UnexpectedSerdeTypeValue<'a>(de::Unexpected<'a>);

        impl<'a> fmt::Display for UnexpectedSerdeTypeValue<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    de::Unexpected::Bool(b) => write!(f, "the boolean `{}`", b),
                    de::Unexpected::Unsigned(i) => write!(f, "the unsigned integer `{}`", i),
                    de::Unexpected::Signed(i) => write!(f, "the signed integer `{}`", i),
                    de::Unexpected::Float(n) => write!(f, "the floating point number `{}`", n),
                    de::Unexpected::Char(c) => write!(f, "the UTF-8 character `{}`", c),
                    de::Unexpected::Str(s) => write!(f, "the string {:?}", s),
                    de::Unexpected::Bytes(b) => write!(f, "the byte string b\"{}\"", {
                        b.iter()
                            .flat_map(|c| core::ascii::escape_default(*c))
                            .map(char::from)
                            .collect::<String>()
                    }),
                    de::Unexpected::Unit => write!(f, "a unit value"),
                    de::Unexpected::Option => write!(f, "an optional value"),
                    de::Unexpected::NewtypeStruct => write!(f, "a newtype struct"),
                    de::Unexpected::Seq => write!(f, "a sequence"),
                    de::Unexpected::Map => write!(f, "a map"),
                    de::Unexpected::Enum => write!(f, "an enum"),
                    de::Unexpected::UnitVariant => write!(f, "a unit variant"),
                    de::Unexpected::NewtypeVariant => write!(f, "a newtype variant"),
                    de::Unexpected::TupleVariant => write!(f, "a tuple variant"),
                    de::Unexpected::StructVariant => write!(f, "a struct variant"),
                    de::Unexpected::Other(other) => f.write_str(other),
                }
            }
        }

        Error::InvalidValueForType {
            expected: exp.to_string(),
            found: UnexpectedSerdeTypeValue(unexp).to_string(),
        }
    }

    #[cold]
    fn invalid_length(len: usize, exp: &dyn de::Expected) -> Self {
        Error::ExpectedDifferentLength {
            expected: exp.to_string(),
            found: len,
        }
    }

    #[cold]
    fn unknown_variant(variant: &str, expected: &'static [&'static str]) -> Self {
        Error::NoSuchEnumVariant {
            expected,
            found: variant.to_string(),
            outer: None,
        }
    }

    #[cold]
    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
        Error::NoSuchStructField {
            expected,
            found: field.to_string(),
            outer: None,
        }
    }

    #[cold]
    fn missing_field(field: &'static str) -> Self {
        Error::MissingStructField { field, outer: None }
    }

    #[cold]
    fn duplicate_field(field: &'static str) -> Self {
        Error::DuplicateStructField { field, outer: None }
    }
}

impl StdError for SpannedError {}

impl StdError for Error {}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::Utf8Error(e)
    }
}

impl From<fmt::Error> for Error {
    fn from(_: fmt::Error) -> Self {
        Error::Fmt
    }
}

#[cfg(feature = "std")]
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<SpannedError> for Error {
    fn from(e: SpannedError) -> Self {
        e.code
    }
}

struct OneOf {
    alts: &'static [&'static str],
    none: &'static str,
}

impl fmt::Display for OneOf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.alts {
            [] => write!(f, "there are no {}", self.none),
            [a1] => write!(f, "expected {} instead", Identifier(a1)),
            [a1, a2] => write!(
                f,
                "expected either {} or {} instead",
                Identifier(a1),
                Identifier(a2)
            ),
            [a1, ref alts @ .., an] => {
                write!(f, "expected one of {}", Identifier(a1))?;

                for alt in alts {
                    write!(f, ", {}", Identifier(alt))?;
                }

                write!(f, ", or {} instead", Identifier(an))
            }
        }
    }
}

struct Identifier<'a>(&'a str);

impl<'a> fmt::Display for Identifier<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() || !self.0.chars().all(is_ident_raw_char) {
            return write!(f, "{:?}_[invalid identifier]", self.0);
        }

        let mut chars = self.0.chars();

        if !chars.next().map_or(false, is_ident_first_char) || !chars.all(is_xid_continue) {
            write!(f, "`r#{}`", self.0)
        } else {
            write!(f, "`{}`", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};

    use serde::{de::Error as DeError, de::Unexpected, ser::Error as SerError};

    use super::{Error, Position, Span, SpannedError};

    #[test]
    fn error_messages() {
        check_error_message(&Error::from(core::fmt::Error), "Formatting RON failed");
        #[cfg(feature = "std")]
        check_error_message(
            &Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "my-error",
            )),
            "my-error",
        );
        check_error_message(&<Error as SerError>::custom("my-ser-error"), "my-ser-error");
        check_error_message(&<Error as DeError>::custom("my-de-error"), "my-de-error");
        #[allow(deprecated)]
        check_error_message(
            &Error::Base64Error(base64::DecodeError::InvalidPadding),
            "Invalid base64: Invalid padding",
        );
        check_error_message(&Error::Eof, "Unexpected end of RON");
        check_error_message(&Error::ExpectedArray, "Expected opening `[`");
        check_error_message(&Error::ExpectedArrayEnd, "Expected closing `]`");
        check_error_message(
            &Error::ExpectedAttribute,
            "Expected an `#![enable(...)]` attribute",
        );
        check_error_message(
            &Error::ExpectedAttributeEnd,
            "Expected closing `)]` after the enable attribute",
        );
        check_error_message(&Error::ExpectedBoolean, "Expected boolean");
        check_error_message(&Error::ExpectedComma, "Expected comma");
        check_error_message(&Error::ExpectedChar, "Expected char");
        check_error_message(&Error::ExpectedByteLiteral, "Expected byte literal");
        check_error_message(&Error::ExpectedFloat, "Expected float");
        check_error_message(&Error::FloatUnderscore, "Unexpected underscore in float");
        check_error_message(&Error::ExpectedInteger, "Expected integer");
        check_error_message(&Error::ExpectedOption, "Expected option");
        check_error_message(&Error::ExpectedOptionEnd, "Expected closing `)`");
        check_error_message(&Error::ExpectedStructLikeEnd, "Expected closing `)`");
        check_error_message(&Error::ExpectedMap, "Expected opening `{`");
        check_error_message(&Error::ExpectedMapColon, "Expected colon");
        check_error_message(&Error::ExpectedMapEnd, "Expected closing `}`");
        check_error_message(
            &Error::ExpectedDifferentStructName {
                expected: "raw+identifier",
                found: String::from("identifier"),
            },
            "Expected struct `r#raw+identifier` but found `identifier`",
        );
        check_error_message(&Error::ExpectedStructLike, "Expected opening `(`");
        check_error_message(
            &Error::ExpectedNamedStructLike(""),
            "Expected only opening `(`, no name, for un-nameable struct",
        );
        check_error_message(
            &Error::ExpectedNamedStructLike("_ident"),
            "Expected opening `(` for struct `_ident`",
        );
        check_error_message(&Error::ExpectedUnit, "Expected unit");
        check_error_message(&Error::ExpectedString, "Expected string");
        check_error_message(&Error::ExpectedByteString, "Expected byte string");
        check_error_message(&Error::ExpectedStringEnd, "Expected end of string");
        check_error_message(&Error::ExpectedIdentifier, "Expected identifier");
        check_error_message(&Error::InvalidEscape("Invalid escape"), "Invalid escape");
        check_error_message(&Error::IntegerOutOfBounds, "Integer is out of bounds");
        check_error_message(
            &Error::InvalidIntegerDigit {
                digit: 'q',
                base: 16,
            },
            "Invalid digit 'q' for base 16 integers",
        );
        check_error_message(
            &Error::NoSuchExtension(String::from("unknown")),
            "No RON extension named `unknown`",
        );
        check_error_message(&Error::UnclosedBlockComment, "Unclosed block comment");
        check_error_message(
            &Error::UnclosedLineComment,
            "`ron::value::RawValue` cannot end in unclosed line comment, \
        try using a block comment or adding a newline",
        );
        check_error_message(
            &Error::UnderscoreAtBeginning,
            "Unexpected leading underscore in a number",
        );
        check_error_message(&Error::UnexpectedChar('🦀'), "Unexpected char \'🦀\'");
        #[allow(invalid_from_utf8)]
        check_error_message(
            &Error::Utf8Error(core::str::from_utf8(b"error: \xff\xff\xff\xff").unwrap_err()),
            "invalid utf-8 sequence of 1 bytes from index 7",
        );
        check_error_message(
            &Error::TrailingCharacters,
            "Non-whitespace trailing characters",
        );
        check_error_message(
            &Error::invalid_value(Unexpected::Enum, &"struct `Hi`"),
            "Expected struct `Hi` but found an enum instead",
        );
        check_error_message(
            &Error::invalid_length(0, &"two bees"),
            "Expected two bees but found zero elements instead",
        );
        check_error_message(
            &Error::invalid_length(1, &"two bees"),
            "Expected two bees but found one element instead",
        );
        check_error_message(
            &Error::invalid_length(3, &"two bees"),
            "Expected two bees but found 3 elements instead",
        );
        check_error_message(
            &Error::unknown_variant("unknown", &[]),
            "Unexpected enum variant named `unknown`, there are no variants",
        );
        check_error_message(
            &Error::NoSuchEnumVariant {
                expected: &["A", "B+C"],
                found: String::from("D"),
                outer: Some(String::from("E")),
            },
            "Unexpected variant named `D` in enum `E`, \
            expected either `A` or `r#B+C` instead",
        );
        check_error_message(
            &Error::unknown_field("unknown", &[]),
            "Unexpected field named `unknown`, there are no fields",
        );
        check_error_message(
            &Error::NoSuchStructField {
                expected: &["a"],
                found: String::from("b"),
                outer: Some(String::from("S")),
            },
            "Unexpected field named `b` in `S`, expected `a` instead",
        );
        check_error_message(
            &Error::NoSuchStructField {
                expected: &["a", "b+c", "d"],
                found: String::from("e"),
                outer: Some(String::from("S")),
            },
            "Unexpected field named `e` in `S`, \
            expected one of `a`, `r#b+c`, or `d` instead",
        );
        check_error_message(
            &Error::missing_field("a"),
            "Unexpected missing field named `a`",
        );
        check_error_message(
            &Error::MissingStructField {
                field: "",
                outer: Some(String::from("S+T")),
            },
            "Unexpected missing field named \"\"_[invalid identifier] in `r#S+T`",
        );
        check_error_message(
            &Error::duplicate_field("a"),
            "Unexpected duplicate field named `a`",
        );
        check_error_message(
            &Error::DuplicateStructField {
                field: "b+c",
                outer: Some(String::from("S+T")),
            },
            "Unexpected duplicate field named `r#b+c` in `r#S+T`",
        );
        check_error_message(
            &Error::InvalidIdentifier(String::from("why+🦀+not")),
            "Invalid identifier \"why+🦀+not\"",
        );
        check_error_message(
            &Error::SuggestRawIdentifier(String::from("raw+ident")),
            "Found invalid std identifier \"raw+ident\", \
            try the raw identifier `r#raw+ident` instead",
        );
        check_error_message(
            &Error::ExpectedRawValue,
            "Expected a `ron::value::RawValue`",
        );
        check_error_message(
            &Error::ExceededRecursionLimit,
            "Exceeded recursion limit, try increasing `ron::Options::recursion_limit` \
            and using `serde_stacker` to protect against a stack overflow",
        );
        check_error_message(
            &Error::ExpectedStructName(String::from("Struct")),
            "Expected the explicit struct name `Struct`, but none was found",
        );
    }

    fn check_error_message<T: core::fmt::Display>(err: &T, msg: &str) {
        assert_eq!(format!("{}", err), msg);
    }

    fn span(start: Position, end: Position) -> Span {
        Span { start, end }
    }

    fn pos(line: usize, col: usize) -> Position {
        Position { line, col }
    }

    #[test]
    fn ascii_basics() {
        let text = "hello\nworld";

        // first char / first col
        assert_eq!(pos(1, 1).grapheme_index(text), Some(0));

        // last char on first line ('o')
        assert_eq!(pos(1, 5).grapheme_index(text), Some(4));

        // start of second line ('w')
        assert_eq!(pos(2, 1).grapheme_index(text), Some(6));

        // span across the `\n`
        assert_eq!(
            span(pos(1, 4), pos(2, 2))
                .substring_exclusive(text)
                .unwrap(),
            "lo\nw"
        );
    }

    #[test]
    fn multibyte_greek() {
        let text = "αβγ\ndeux\n三四五\r\nend";

        // Beta
        assert_eq!(pos(1, 2).grapheme_index(text), Some(1));

        // 三
        assert_eq!(pos(3, 1).grapheme_index(text), Some(9));

        // e
        assert_eq!(pos(4, 1).grapheme_index(text), Some(13));

        // span from α to start of “deux”
        assert_eq!(
            span(pos(1, 1), pos(2, 1))
                .substring_exclusive(text)
                .unwrap(),
            "αβγ\n"
        );
    }

    #[test]
    fn combining_mark_cluster() {
        // é  ==  [0x65, 0xCC, 0x81] in UTF-8
        let text = "e\u{0301}x\n";

        // grapheme #1 (“é”)
        assert_eq!(pos(1, 1).grapheme_index(text), Some(0));

        // grapheme #2 (“x”)
        assert_eq!(pos(1, 2).grapheme_index(text), Some(1));

        // column 4 is past EOL
        assert_eq!(pos(1, 4).grapheme_index(text), None);

        // full span
        assert_eq!(
            span(pos(1, 1), pos(1, 2))
                .substring_exclusive(text)
                .unwrap(),
            "e\u{0301}"
        );
    }

    #[test]
    fn zwj_emoji_cluster() {
        let text = "👩‍👩‍👧‍👧 and 👨‍👩‍👦";

        // The family emoji is the first grapheme on the line.
        assert_eq!(pos(1, 1).grapheme_index(text), Some(0));

        assert_eq!(pos(1, 2).grapheme_index(text), Some(1));

        // Span selecting only the first emoji
        assert_eq!(
            span(pos(1, 1), pos(1, 2))
                .substring_exclusive(text)
                .unwrap(),
            "👩‍👩‍👧‍👧"
        );

        // Span selecting only the second emoji
        assert_eq!(
            span(pos(1, 7), pos(1, 8))
                .substring_exclusive(text)
                .unwrap(),
            "👨‍👩‍👦"
        );
    }

    #[test]
    fn mixed_newlines() {
        let text = "one\r\ntwo\nthree\r\n";

        // start of “two” (line numbers are 1-based)
        assert_eq!(pos(2, 1).grapheme_index(text), Some(4));

        // “three”
        assert_eq!(pos(3, 1).grapheme_index(text), Some(8));

        // span “two\n”
        assert_eq!(
            span(pos(2, 1), pos(3, 1))
                .substring_exclusive(text)
                .unwrap(),
            "two\n"
        );

        // span “two\nthree”
        assert_eq!(
            span(pos(2, 1), pos(3, 6))
                .substring_exclusive(text)
                .unwrap(),
            "two\nthree"
        );
    }

    #[test]
    fn oob_and_error_paths() {
        let text = "short";

        // line past EOF
        assert_eq!(pos(2, 1).grapheme_index(text), None);

        // column past EOL
        assert_eq!(pos(1, 10).grapheme_index(text), None);

        // span with either endpoint oob → None
        assert_eq!(span(pos(1, 1), pos(2, 1)).substring_exclusive(text), None);
    }

    #[test]
    fn whole_text_span() {
        let text = "αβγ\nδεζ";
        let all = span(pos(1, 1), pos(2, 4));
        assert_eq!(&all.substring_exclusive(text).unwrap(), text);
    }

    #[test]
    fn span_substring_helper() {
        assert_eq!(
            Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 2, col: 1 },
            }
            .substring_exclusive(
                "In the first place, there are two sorts of bets, or toh.11 There is the
single axial bet in the center between the principals (toh ketengah), and
there is the cloud of peripheral ones around the ring between members
of the audience (toh kesasi). ",
            )
            .unwrap(),
            "In the first place, there are two sorts of bets, or toh.11 There is the\n"
        );

        assert_eq!(
            Span {
                start: Position { line: 2, col: 1 },
                end: Position { line: 3, col: 1 },
            }
            .substring_exclusive(
                "In the first place, there are two sorts of bets, or toh.11 There is the
single axial bet in the center between the principals (toh ketengah), and
there is the cloud of peripheral ones around the ring between members
of the audience (toh kesasi). ",
            )
            .unwrap(),
            "single axial bet in the center between the principals (toh ketengah), and\n"
        );
    }

    #[test]
    fn spanned_error_into_code() {
        assert_eq!(
            Error::from(SpannedError {
                code: Error::Eof,
                span: Span {
                    start: Position { line: 1, col: 1 },
                    end: Position { line: 1, col: 5 },
                }
            }),
            Error::Eof
        );
        assert_eq!(
            Error::from(SpannedError {
                code: Error::ExpectedRawValue,
                span: Span {
                    start: Position { line: 1, col: 1 },
                    end: Position { line: 1, col: 5 },
                }
            }),
            Error::ExpectedRawValue
        );
    }
}
