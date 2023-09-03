use std::{
    error::Error as StdError,
    fmt, io,
    str::{self, Utf8Error},
};

use serde::{de, ser};
use unicode_ident::is_xid_continue;

use crate::parse::{is_ident_first_char, is_ident_raw_char};

/// This type represents all possible errors that can occur when
/// serializing or deserializing RON data.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpannedError {
    pub code: Error,
    pub position: Position,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
pub type SpannedResult<T> = std::result::Result<T, SpannedError>;

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
}

impl fmt::Display for SpannedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.position == Position { line: 0, col: 0 }) {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{}: {}", self.position, self.code)
        }
    }
}

impl fmt::Display for Error {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Fmt => f.write_str("Formatting RON failed"),
            Error::Io(ref s) | Error::Message(ref s) => f.write_str(s),
            #[allow(deprecated)]
            Error::Base64Error(ref e) => fmt::Display::fmt(e, f),
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
            Error::ExpectedOptionEnd | Error::ExpectedStructLikeEnd => f.write_str("Expected closing `)`"),
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
            },
            Error::NoSuchExtension(ref name) => {
                write!(f, "No RON extension named {}", Identifier(name))
            }
            Error::Utf8Error(ref e) => fmt::Display::fmt(e, f),
            Error::UnclosedBlockComment => f.write_str("Unclosed block comment"),
            Error::UnclosedLineComment => f.write_str(
                "`ron::value::RawValue` cannot end in unclosed line comment, \
                try using a block comment or adding a newline"
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
                    write!(f, "in enum {}", Identifier(outer))?;
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
                    write!(f, "in {}", Identifier(outer))?;
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
                write!(f, "Unexpected missing field {}", Identifier(field))?;

                match outer {
                    Some(outer) => write!(f, " in {}", Identifier(outer)),
                    None => Ok(()),
                }
            }
            Error::DuplicateStructField { field, ref outer } => {
                write!(f, "Unexpected duplicate field {}", Identifier(field))?;

                match outer {
                    Some(outer) => write!(f, " in {}", Identifier(outer)),
                    None => Ok(()),
                }
            }
            Error::InvalidIdentifier(ref invalid) => write!(f, "Invalid identifier {:?}", invalid),
            Error::SuggestRawIdentifier(ref identifier) => write!(
                f,
                "Found invalid std identifier `{}`, try the raw identifier `r#{}` instead",
                identifier, identifier
            ),
            Error::ExpectedRawValue => f.write_str("Expected a `ron::value::RawValue`"),
            Error::ExceededRecursionLimit => f.write_str("Exceeded recursion limit, try increasing the limit and using `serde_stacker` to protect against a stack overflow"),
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

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
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
                            .flat_map(|c| std::ascii::escape_default(*c))
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

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<io::Error> for SpannedError {
    fn from(e: io::Error) -> Self {
        SpannedError {
            code: e.into(),
            position: Position { line: 0, col: 0 },
        }
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
            [a1, ref alts @ ..] => {
                write!(f, "expected one of {}", Identifier(a1))?;

                for alt in alts {
                    write!(f, ", {}", Identifier(alt))?;
                }

                f.write_str(" instead")
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
    use super::Error;

    #[test]
    fn error_messages() {
        check_error_message(&Error::from(std::fmt::Error), "Formatting RON failed");
        check_error_message(
            &Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "my-error",
            )),
            "my-error",
        );
        check_error_message(&Error::Message(String::from("my-error")), "my-error");
        check_error_message(&Error::ExpectedOptionEnd, "Expected closing `)`");
        check_error_message(&Error::ExpectedStructLikeEnd, "Expected closing `)`");
    }

    fn check_error_message<T: std::fmt::Display>(err: &T, msg: &str) {
        assert_eq!(format!("{}", err), msg);
    }
}
