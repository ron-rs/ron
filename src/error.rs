use serde::{de, ser};
use std::{error::Error as StdError, fmt, io, str::Utf8Error, string::FromUtf8Error};

/// This type represents all possible errors that can occur when
/// serializing or deserializing RON data.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub code: ErrorCode,
    pub position: Position,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ErrorCode {
    Io(String),
    Message(String),
    Base64Error(base64::DecodeError),
    Eof,
    ExpectedArray,
    ExpectedArrayEnd,
    ExpectedAttribute,
    ExpectedAttributeEnd,
    ExpectedBoolean,
    ExpectedComma,
    ExpectedChar,
    ExpectedFloat,
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
    ExpectedStringEnd,
    ExpectedIdentifier,

    InvalidEscape(&'static str),

    IntegerOutOfBounds,

    NoSuchExtension(String),

    UnclosedBlockComment,
    UnderscoreAtBeginning,
    UnexpectedByte(char),

    Utf8Error(Utf8Error),
    TrailingCharacters,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.position == Position { line: 0, col: 0 }) {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{}: {}", self.position, self.code)
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ErrorCode::Io(ref s) => f.write_str(s),
            ErrorCode::Message(ref s) => f.write_str(s),
            ErrorCode::Base64Error(ref e) => fmt::Display::fmt(e, f),
            ErrorCode::Eof => f.write_str("Unexpected end of RON"),
            ErrorCode::ExpectedArray => f.write_str("Expected opening `[`"),
            ErrorCode::ExpectedArrayEnd => f.write_str("Expected closing `]`"),
            ErrorCode::ExpectedAttribute => f.write_str("Expected an `#![enable(...)]` attribute"),
            ErrorCode::ExpectedAttributeEnd => {
                f.write_str("Expected closing `)]` after the enable attribute")
            }
            ErrorCode::ExpectedBoolean => f.write_str("Expected boolean"),
            ErrorCode::ExpectedComma => f.write_str("Expected comma"),
            ErrorCode::ExpectedChar => f.write_str("Expected char"),
            ErrorCode::ExpectedFloat => f.write_str("Expected float"),
            ErrorCode::ExpectedInteger => f.write_str("Expected integer"),
            ErrorCode::ExpectedOption => f.write_str("Expected option"),
            ErrorCode::ExpectedOptionEnd => f.write_str("Expected closing `)`"),
            ErrorCode::ExpectedMap => f.write_str("Expected opening `{`"),
            ErrorCode::ExpectedMapColon => f.write_str("Expected colon"),
            ErrorCode::ExpectedMapEnd => f.write_str("Expected closing `}`"),
            ErrorCode::ExpectedDifferentStructName {
                expected,
                ref found,
            } => write!(f, "Expected struct '{}' but found '{}'", expected, found),
            ErrorCode::ExpectedStructLike => f.write_str("Expected opening `(`"),
            ErrorCode::ExpectedNamedStructLike(name) => {
                write!(f, "Expected opening `(` for struct '{}'", name)
            }
            ErrorCode::ExpectedStructLikeEnd => f.write_str("Expected closing `)`"),
            ErrorCode::ExpectedUnit => f.write_str("Expected unit"),
            ErrorCode::ExpectedString => f.write_str("Expected string"),
            ErrorCode::ExpectedStringEnd => f.write_str("Expected end of string"),
            ErrorCode::ExpectedIdentifier => f.write_str("Expected identifier"),
            ErrorCode::InvalidEscape(s) => f.write_str(s),
            ErrorCode::IntegerOutOfBounds => f.write_str("Integer is out of bounds"),
            ErrorCode::NoSuchExtension(ref name) => write!(f, "No RON extension named '{}'", name),
            ErrorCode::Utf8Error(ref e) => fmt::Display::fmt(e, f),
            ErrorCode::UnclosedBlockComment => f.write_str("Unclosed block comment"),
            ErrorCode::UnderscoreAtBeginning => {
                f.write_str("Unexpected leading underscore in an integer")
            }
            ErrorCode::UnexpectedByte(ref byte) => write!(f, "Unexpected byte {:?}", byte),
            ErrorCode::TrailingCharacters => f.write_str("Non-whitespace trailing characters"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error {
            code: ErrorCode::Message(msg.to_string()),
            position: Position { line: 0, col: 0 },
        }
    }
}

impl de::Error for ErrorCode {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        ErrorCode::Message(msg.to_string())
    }
}

impl StdError for Error {}
impl StdError for ErrorCode {}

impl From<Utf8Error> for ErrorCode {
    fn from(e: Utf8Error) -> Self {
        ErrorCode::Utf8Error(e)
    }
}

impl From<FromUtf8Error> for ErrorCode {
    fn from(e: FromUtf8Error) -> Self {
        ErrorCode::Utf8Error(e.utf8_error())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error {
            code: ErrorCode::Io(e.to_string()),
            position: Position { line: 0, col: 0 },
        }
    }
}
