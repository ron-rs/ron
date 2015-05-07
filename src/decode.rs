extern crate rustc_serialize;

use std::io;
use std::collections::HashMap;


pub type Line = usize;
pub type Column = usize;

/// The errors that can arise while parsing a RON stream.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ErrorCode {
    InvalidSyntax,
    InvalidNumber,
    EOFWhileParsingObject,
    EOFWhileParsingArray,
    EOFWhileParsingMap,
    EOFWhileParsingValue,
    KeyMustBeAValue,
    ExpectedColon,
    TrailingCharacters,
    InvalidEscape,
    InvalidUnicodeCodePoint,
    LoneLeadingSurrogateInHexEscape,
    NotUtf8,
}

#[derive(Debug)]
pub enum ParserError {
    Syntax(ErrorCode, Line, Column),
    Io(io::Error),
}

impl PartialEq for ParserError {
    fn eq(&self, other: &ParserError) -> bool {
        match (self, other) {
            (&ParserError::Syntax(msg0, line0, col0), &ParserError::Syntax(msg1, line1, col1)) =>
                msg0 == msg1 && line0 == line1 && col0 == col1,
            (&ParserError::Io(_), _) => false,
            (_, &ParserError::Io(_)) => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Parse(ParserError),
    Expectation(String, String),
    MissingField(String),
    UnknownVariant(String),
    Application(String),
    EOF,
}

/// The output of the streaming parser.
#[derive(PartialEq, Debug)]
enum RonEvent {
    ObjectStart,
    ObjectEnd,
    ArrayStart,
    ArrayEnd,
    MapStart,
    MapEnd,
    BooleanValue(bool),
    I64Value(i64),
    U64Value(u64),
    F64Value(f64),
    StringValue(String),
    Error(ParserError),
}


#[derive(PartialEq, Clone, Debug)]
enum InternalStackElement {
    Index(u32),
    Key(u16, u16), // start, size
}

struct Stack {
    elements: Vec<InternalStackElement>,
    str_buffer: Vec<u8>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { elements: Vec::new(), str_buffer: Vec::new() }
    }
}


struct Parser<T> {
    input: T,
    ch: Option<char>,
    line: Line,
    col: Column,
    stack: Stack,
    //state: ParserState,
}

impl<T: Iterator<Item = char>> Parser<T> {
    /// Create the RON parser.
    fn new(input: T) -> Parser<T> {
        let mut p = Parser {
            input: input,
            ch: Some('\x00'),
            line: 1,
            col: 0,
            stack: Stack::new(),
            //state: ParseStart,
        };
        p.bump();
        p
    }

    fn ch_is(&self, c: char) -> bool {
        self.ch == Some(c)
    }

    fn bump(&mut self) {
        self.ch = self.input.next();
        if self.ch_is('\n') {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
    }

    fn error<E>(&self, reason: ErrorCode) -> Result<E, ParserError> {
        Err(ParserError::Syntax(reason, self.line, self.col))
    }

    fn parse_whitespace(&mut self) {
        while self.ch_is(' ') ||
            self.ch_is('\n') ||
            self.ch_is('\t') ||
            self.ch_is('\r') { self.bump(); }
    }
}

pub struct Decoder<I> {
    parser: Parser<I>,
}

impl<I: Iterator<Item = char>> Decoder<I> {
    pub fn new(input: I) -> Decoder<I> {
        Decoder { parser: Parser::new(input) }
    }
}

macro_rules! read_primitive {
    ($name:ident, $ty:ident) => {
        fn $name(&mut self) -> Result<$ty, Error> {
            Err(Error::EOF)
        }
    }
}

impl<I: Iterator<Item = char>> rustc_serialize::Decoder for Decoder<I> {
    type Error = Error;

    fn read_nil(&mut self) -> Result<(), Error> { Err(Error::EOF) }

    read_primitive! { read_usize, usize }
    read_primitive! { read_u8, u8 }
    read_primitive! { read_u16, u16 }
    read_primitive! { read_u32, u32 }
    read_primitive! { read_u64, u64 }
    read_primitive! { read_isize, isize }
    read_primitive! { read_i8, i8 }
    read_primitive! { read_i16, i16 }
    read_primitive! { read_i32, i32 }
    read_primitive! { read_i64, i64 }

    fn read_f32(&mut self) -> Result<f32, Error> { Err(Error::EOF) }

    fn read_f64(&mut self) -> Result<f64, Error> { Err(Error::EOF) }

    fn read_bool(&mut self) -> Result<bool, Error> { Err(Error::EOF) }

    fn read_char(&mut self) -> Result<char, Error> { Err(Error::EOF) }

    fn read_str(&mut self) -> Result<String, Error> { Err(Error::EOF) }

    fn read_enum<T, F>(&mut self, _name: &str, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_enum_variant<T, F>(&mut self, _names: &[&str], _f: F) -> Result<T, Error> where
        F: FnMut(&mut Decoder<I>, usize) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_enum_variant_arg<T, F>(&mut self, _idx: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_enum_struct_variant<T, F>(&mut self, _names: &[&str], _f: F) -> Result<T, Error> where
        F: FnMut(&mut Decoder<I>, usize) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_enum_struct_variant_field<T, F>(&mut self, _name: &str, _idx: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_struct<T, F>(&mut self, _name: &str, _len: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_struct_field<T, F>(&mut self, _name: &str, _idx: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_tuple<T, F>(&mut self, _len: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_tuple_arg<T, F>(&mut self, _idx: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_tuple_struct<T, F>(&mut self, _name: &str, _len: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_tuple_struct_arg<T, F>(&mut self, _idx: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_option<T, F>(&mut self, _f: F) -> Result<T, Error> where
        F: FnMut(&mut Decoder<I>, bool) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_seq<T, F>(&mut self, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>, usize) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_seq_elt<T, F>(&mut self, _idx: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_map<T, F>(&mut self, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>, usize) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_map_elt_key<T, F>(&mut self, _idx: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn read_map_elt_val<T, F>(&mut self, _idx: usize, _f: F) -> Result<T, Error> where
        F: FnOnce(&mut Decoder<I>) -> Result<T, Error>
    {
        Err(Error::EOF)
    }

    fn error(&mut self, err: &str) -> Error {
        Error::Application(err.to_string())
    }
}
