#![allow(clippy::identity_op)]

use std::{
    char::from_u32 as char_from_u32,
    str::{self, from_utf8, FromStr, Utf8Error},
};

use unicode_ident::{is_xid_continue, is_xid_start};

use crate::{
    error::{Error, Position, Result, SpannedError, SpannedResult},
    extensions::Extensions,
    value::Number,
};

// We have the following char categories.
const INT_CHAR: u8 = 1 << 0; // [0-9A-Fa-f_]
const FLOAT_CHAR: u8 = 1 << 1; // [0-9\.Ee+-_]

// We encode each char as belonging to some number of these categories.
const DIGIT: u8 = INT_CHAR | FLOAT_CHAR; // [0-9]
const ABCDF: u8 = INT_CHAR; // [ABCDFabcdf]
const UNDER: u8 = INT_CHAR | FLOAT_CHAR; // [_]
const E____: u8 = INT_CHAR | FLOAT_CHAR; // [Ee]
const PUNCT: u8 = FLOAT_CHAR; // [\.+-]
const _____: u8 = 0; // everything else

// Table of encodings, for fast predicates. (Non-ASCII and special chars are
// shown with '·' in the comment.)
#[rustfmt::skip]
const ENCODINGS: [u8; 256] = [
/*                     0      1      2      3      4      5      6      7      8      9    */
/*   0+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/*  10+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/*  20+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/*  30+: ·· !"#$%&' */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/*  40+: ()*+,-./01 */ _____, _____, _____, PUNCT, _____, PUNCT, PUNCT, _____, DIGIT, DIGIT,
/*  50+: 23456789:; */ DIGIT, DIGIT, DIGIT, DIGIT, DIGIT, DIGIT, DIGIT, DIGIT, _____, _____,
/*  60+: <=>?@ABCDE */ _____, _____, _____, _____, _____, ABCDF, ABCDF, ABCDF, ABCDF, E____,
/*  70+: FGHIJKLMNO */ ABCDF, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/*  80+: PQRSTUVWZY */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/*  90+: Z[\]^_`abc */ _____, _____, _____, _____, _____, UNDER, _____, ABCDF, ABCDF, ABCDF,
/* 100+: defghijklm */ ABCDF, E____, ABCDF, _____, _____, _____, _____, _____, _____, _____,
/* 110+: nopqrstuvw */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 120+: xyz{|}~··· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 130+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 140+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 150+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 160+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 170+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 180+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 190+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 200+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 210+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 220+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 230+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 240+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/* 250+: ·········· */ _____, _____, _____, _____, _____, _____
];

const fn is_int_char(c: u8) -> bool {
    ENCODINGS[c as usize] & INT_CHAR != 0
}

const fn is_float_char(c: u8) -> bool {
    ENCODINGS[c as usize] & FLOAT_CHAR != 0
}

pub fn is_ident_first_char(c: char) -> bool {
    c == '_' || is_xid_start(c)
}

pub fn is_ident_raw_char(c: char) -> bool {
    is_xid_continue(c) || matches!(c, '.' | '+' | '-')
}

const fn is_whitespace_char(c: char) -> bool {
    // TODO compare to using a bitmap
    // " \t\n\r\x0A\x0B\x0C\u{85}" could be added to the above map
    // for 200E to 2029 this could be done by casting to u16 and comparing first and second byte individually
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '\x0B'
            | '\x0C'
            | '\u{85}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

#[derive(Clone, Copy, Debug)]
pub struct Parser<'a> {
    /// Bits set according to the [`Extensions`] enum.
    pub exts: Extensions,
    src: &'a str,
    pre_ws_src: &'a str,
    last_ws_len: usize,
    cursor: Position,
}

#[cfg(feature = "integer128")]
pub(crate) type LargeUInt = u128;
#[cfg(not(feature = "integer128"))]
pub(crate) type LargeUInt = u64;
#[cfg(feature = "integer128")]
pub(crate) type LargeSInt = i128;
#[cfg(not(feature = "integer128"))]
pub(crate) type LargeSInt = i64;

/// constructor and parsing utilities
impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> SpannedResult<Self> {
        let mut parser = Parser {
            exts: Extensions::empty(),
            src: source,
            pre_ws_src: source,
            last_ws_len: 0,
            cursor: Position { line: 1, col: 1 },
        };

        parser.skip_ws().map_err(|e| parser.span_error(e))?;

        // Loop over all extensions attributes
        loop {
            let attribute = parser.extensions().map_err(|e| parser.span_error(e))?;

            if attribute.is_empty() {
                break;
            }

            parser.exts |= attribute;
            parser.skip_ws().map_err(|e| parser.span_error(e))?;
        }

        Ok(parser)
    }

    pub fn span_error(&self, code: Error) -> SpannedError {
        SpannedError {
            code,
            position: self.cursor,
        }
    }

    pub fn advance(&mut self, bytes: usize) -> Result<()> {
        let mut idx = 0;
        while idx < bytes {
            idx += self.advance_char()?;
        }
        debug_assert_eq!(idx, bytes, "bytes are a valid unicode boundry");

        Ok(())
    }

    pub fn advance_char(&mut self) -> Result<usize> {
        self.next().map(char::len_utf8)
    }

    pub fn next(&mut self) -> Result<char> {
        let c = self.peek()?;
        if c == '\n' {
            self.cursor.line += 1;
            self.cursor.col = 1;
        } else {
            self.cursor.col += 1;
        }

        self.src = &self.src[c.len_utf8()..];

        Ok(c)
    }

    pub fn peek(&self) -> Result<char> {
        self.src.chars().next().ok_or(Error::Eof)
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.src.as_bytes()
    }

    pub fn source(&self) -> &'a str {
        self.src
    }

    pub fn pre_ws_source(&self) -> &'a str {
        self.pre_ws_src
    }

    pub fn consume_str(&mut self, s: &str) -> bool {
        if self.src.starts_with(s) {
            let _ = self.advance(s.len());

            true
        } else {
            false
        }
    }

    pub fn consume_char(&mut self, expected: char) -> bool {
        if let Ok(c) = self.peek() {
            if c == expected {
                _ = self.advance_char();
                return true;
            }
        }
        false
    }

    fn consume_all(&mut self, all: &[&str]) -> Result<bool> {
        all.iter()
            .map(|elem| {
                if self.consume_str(elem) {
                    self.skip_ws()?;

                    Ok(true)
                } else {
                    Ok(false)
                }
            })
            .fold(Ok(true), |acc, x| acc.and_then(|val| x.map(|x| x && val)))
    }

    pub fn expect_char(&mut self, expected: char, error: Error) -> Result<()> {
        self.consume_char(expected).then_some(()).ok_or(error)
    }

    #[must_use]
    pub fn next_bytes_while(&self, condition: fn(u8) -> bool) -> usize {
        self.next_bytes_while_from(0, condition)
    }

    #[must_use]
    pub fn next_bytes_while_max(&self, max: usize, condition: fn(u8) -> bool) -> usize {
        self.next_bytes_while_from_max(0, max, condition)
    }

    #[must_use]
    pub fn next_bytes_while_from(&self, from: usize, condition: fn(u8) -> bool) -> usize {
        self.src[from..]
            .as_bytes()
            .iter()
            .take_while(|&&b| condition(b))
            .count()
    }

    #[must_use]
    pub fn next_bytes_while_from_max(
        &self,
        from: usize,
        mut max: usize,
        condition: fn(u8) -> bool,
    ) -> usize {
        self.src[from..]
            .as_bytes()
            .iter()
            .take_while(|&&b| {
                max > 0 && {
                    max -= 1;
                    condition(b)
                }
            })
            .count()
    }

    #[must_use]
    pub fn next_chars_while(&self, condition: fn(char) -> bool) -> usize {
        self.next_chars_while_from(0, condition)
    }

    #[must_use]
    pub fn next_chars_while_from(&self, from: usize, condition: fn(char) -> bool) -> usize {
        self.src[from..]
            .find(|c| !condition(c))
            .unwrap_or(self.src.len() - from)
    }

    pub fn peek_char_or_eof(&self) -> Result<char> {
        self.src.chars().next().ok_or(Error::Eof)
    }
}

/// actual parsing of ron tokens
impl<'a> Parser<'a> {
    fn parse_integer<T: Num>(&mut self, sign: i8) -> Result<T> {
        let base = if self.peek_char_or_eof()? == '0' {
            match self.src.chars().nth(1) {
                Some('x') => 16,
                Some('b') => 2,
                Some('o') => 8,
                _ => 10,
            }
        } else {
            10
        };

        if base != 10 {
            // If we have `0x45A` for example,
            // cut it to `45A`.
            let _ = self.advance(2);
        }

        let num_bytes = self.next_bytes_while(is_int_char);

        if num_bytes == 0 {
            return Err(Error::ExpectedInteger);
        }

        let s = &self.src[..num_bytes];

        if s.starts_with('_') {
            return Err(Error::UnderscoreAtBeginning);
        }

        fn calc_num<T: Num>(
            parser: &mut Parser,
            s: &str,
            base: u8,
            f: fn(&mut T, u8) -> bool,
        ) -> Result<T> {
            let mut num_acc = T::from_u8(0);

            for (i, byte) in s.chars().enumerate() {
                if byte == '_' {
                    continue;
                }

                if num_acc.checked_mul_ext(base) {
                    let _ = parser.advance(s.len());
                    return Err(Error::IntegerOutOfBounds);
                }

                let digit = parser.decode_hex(byte)?;

                if digit >= base {
                    let _ = parser.advance(i);
                    return Err(Error::InvalidIntegerDigit { digit: byte, base });
                }

                if f(&mut num_acc, digit) {
                    let _ = parser.advance(s.len());
                    return Err(Error::IntegerOutOfBounds);
                }
            }

            let _ = parser.advance(s.len());

            Ok(num_acc)
        }

        if sign > 0 {
            calc_num(self, s, base, T::checked_add_ext)
        } else {
            calc_num(self, s, base, T::checked_sub_ext)
        }
    }

    pub fn integer<T: Integer>(&mut self) -> Result<T> {
        let src_backup = self.src;

        let is_negative = match self.peek_char_or_eof()? {
            '+' => {
                let _ = self.advance_char();
                false
            }
            '-' => {
                let _ = self.advance_char();
                true
            }
            'b' if self.consume_str("b'") => {
                // Parse a byte literal
                let byte = match self.next()? {
                    '\\' => match self.parse_escape(EscapeEncoding::Binary, true)? {
                        // we know that this byte is an ASCII character
                        EscapeCharacter::Ascii(b) => b,
                        EscapeCharacter::Utf8(_) => {
                            return Err(Error::InvalidEscape(
                                "Unexpected Unicode escape in byte literal",
                            ))
                        }
                    },
                    b if b.is_ascii() => b as u8,
                    _ => return Err(Error::ExpectedByteLiteral),
                };

                if !self.consume_char('\'') {
                    return Err(Error::ExpectedByteLiteral);
                }

                let bytes_ron = &src_backup[..src_backup.len() - self.src.len()];

                return T::try_from_parsed_integer(ParsedInteger::U8(byte), bytes_ron);
            }
            _ => false,
        };
        let sign = if is_negative { -1 } else { 1 };

        let num_bytes = self.next_bytes_while(is_int_char);

        if self.src[num_bytes..].starts_with(&['i', 'u']) {
            let int_parser_backup = *self;
            let _ = self.advance(num_bytes);

            #[allow(clippy::never_loop)]
            loop {
                let (res, suffix_bytes) = if self.consume_ident("i8") {
                    let suffix_bytes = self.src;
                    *self = int_parser_backup;
                    (
                        self.parse_integer::<i8>(sign).map(ParsedInteger::I8),
                        suffix_bytes,
                    )
                } else if self.consume_ident("i16") {
                    let suffix_bytes = self.src;
                    *self = int_parser_backup;
                    (
                        self.parse_integer::<i16>(sign).map(ParsedInteger::I16),
                        suffix_bytes,
                    )
                } else if self.consume_ident("i32") {
                    let suffix_bytes = self.src;
                    *self = int_parser_backup;
                    (
                        self.parse_integer::<i32>(sign).map(ParsedInteger::I32),
                        suffix_bytes,
                    )
                } else if self.consume_ident("i64") {
                    let suffix_bytes = self.src;
                    *self = int_parser_backup;
                    (
                        self.parse_integer::<i64>(sign).map(ParsedInteger::I64),
                        suffix_bytes,
                    )
                } else if self.consume_ident("u8") {
                    let suffix_bytes = self.src;
                    *self = int_parser_backup;
                    (
                        self.parse_integer::<u8>(sign).map(ParsedInteger::U8),
                        suffix_bytes,
                    )
                } else if self.consume_ident("u16") {
                    let suffix_bytes = self.src;
                    *self = int_parser_backup;
                    (
                        self.parse_integer::<u16>(sign).map(ParsedInteger::U16),
                        suffix_bytes,
                    )
                } else if self.consume_ident("u32") {
                    let suffix_bytes = self.src;
                    *self = int_parser_backup;
                    (
                        self.parse_integer::<u32>(sign).map(ParsedInteger::U32),
                        suffix_bytes,
                    )
                } else if self.consume_ident("u64") {
                    let suffix_bytes = self.src;
                    *self = int_parser_backup;
                    (
                        self.parse_integer::<u64>(sign).map(ParsedInteger::U64),
                        suffix_bytes,
                    )
                } else {
                    #[cfg(feature = "integer128")]
                    if self.consume_ident("i128") {
                        let suffix_bytes = self.src;
                        *self = int_parser_backup;
                        (
                            self.parse_integer::<i128>(sign).map(ParsedInteger::I128),
                            suffix_bytes,
                        )
                    } else if self.consume_ident("u128") {
                        let suffix_bytes = self.src;
                        *self = int_parser_backup;
                        (
                            self.parse_integer::<u128>(sign).map(ParsedInteger::U128),
                            suffix_bytes,
                        )
                    } else {
                        break;
                    }
                    #[cfg(not(feature = "integer128"))]
                    {
                        break;
                    }
                };

                if !matches!(
                    &res,
                    Err(Error::UnderscoreAtBeginning | Error::InvalidIntegerDigit { .. })
                ) {
                    // Advance past the number suffix
                    let _ = self.identifier();
                }

                let integer_ron = &src_backup[..src_backup.len() - suffix_bytes.len()];

                return res.and_then(|parsed| T::try_from_parsed_integer(parsed, integer_ron));
            }

            *self = int_parser_backup;
        }

        T::parse(self, sign)
    }

    pub fn any_number(&mut self) -> Result<Number> {
        if self.next_bytes_is_float() {
            return match self.float::<ParsedFloat>()? {
                ParsedFloat::F32(v) => Ok(Number::F32(v.into())),
                ParsedFloat::F64(v) => Ok(Number::F64(v.into())),
            };
        }

        let parser_backup = *self;

        let (integer_err, integer_bytes) = match self.integer::<ParsedInteger>() {
            Ok(integer) => {
                return match integer {
                    ParsedInteger::I8(v) => Ok(Number::I8(v)),
                    ParsedInteger::I16(v) => Ok(Number::I16(v)),
                    ParsedInteger::I32(v) => Ok(Number::I32(v)),
                    ParsedInteger::I64(v) => Ok(Number::I64(v)),
                    #[cfg(feature = "integer128")]
                    ParsedInteger::I128(v) => Ok(Number::I128(v)),
                    ParsedInteger::U8(v) => Ok(Number::U8(v)),
                    ParsedInteger::U16(v) => Ok(Number::U16(v)),
                    ParsedInteger::U32(v) => Ok(Number::U32(v)),
                    ParsedInteger::U64(v) => Ok(Number::U64(v)),
                    #[cfg(feature = "integer128")]
                    ParsedInteger::U128(v) => Ok(Number::U128(v)),
                }
            }
            Err(err) => (err, *self),
        };

        *self = parser_backup;

        // Fall-back to parse an out-of-range integer as a float
        match self.float::<ParsedFloat>() {
            Ok(ParsedFloat::F32(v)) if self.cursor >= integer_bytes.cursor => {
                Ok(Number::F32(v.into()))
            }
            Ok(ParsedFloat::F64(v)) if self.cursor >= integer_bytes.cursor => {
                Ok(Number::F64(v.into()))
            }
            _ => {
                // Return the more precise integer error
                *self = integer_bytes;
                Err(integer_err)
            }
        }
    }

    pub fn bool(&mut self) -> Result<bool> {
        if self.consume_str("true") {
            Ok(true)
        } else if self.consume_str("false") {
            Ok(false)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    pub fn char(&mut self) -> Result<char> {
        self.expect_char('\'', Error::ExpectedChar)?;

        let c = self.next()?;

        let c = if c == '\\' {
            match self.parse_escape(EscapeEncoding::Utf8, true)? {
                // we know that this byte is an ASCII character
                EscapeCharacter::Ascii(b) => char::from(b),
                EscapeCharacter::Utf8(c) => c,
            }
        } else {
            c
        };

        self.expect_char('\'', Error::ExpectedChar)?;

        Ok(c)
    }

    pub fn comma(&mut self) -> Result<bool> {
        self.skip_ws()?;

        if self.consume_char(',') {
            self.skip_ws()?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Only returns true if the char after `ident` cannot belong
    /// to an identifier.
    pub fn check_ident(&mut self, ident: &str) -> bool {
        self.src.starts_with(ident) && !self.check_ident_other_char(ident.len())
    }

    fn check_ident_other_char(&self, index: usize) -> bool {
        self.src[index..]
            .chars()
            .next()
            .map_or(false, is_xid_continue)
    }

    /// Check which type of struct we are currently parsing. The parsing state
    ///  is only changed in case of an error, to provide a better position.
    ///
    /// [`NewtypeMode::NoParensMeanUnit`] detects (tuple) structs by a leading
    ///  opening bracket and reports a unit struct otherwise.
    /// [`NewtypeMode::InsideNewtype`] skips an initial check for unit structs,
    ///  and means that any leading opening bracket is not considered to open
    ///  a (tuple) struct but to be part of the structs inner contents.
    ///
    /// [`TupleMode::ImpreciseTupleOrNewtype`] only performs a cheap, O(1),
    ///  single-identifier lookahead check to distinguish tuple structs from
    ///  non-tuple structs.
    /// [`TupleMode::DifferentiateNewtype`] performs an expensive, O(N), look-
    ///  ahead over the entire next value tree, which can span the entirety of
    ///  the remaining document in the worst case.
    pub fn check_struct_type(
        &mut self,
        newtype: NewtypeMode,
        tuple: TupleMode,
    ) -> Result<StructType> {
        fn check_struct_type_inner(
            parser: &mut Parser,
            newtype: NewtypeMode,
            tuple: TupleMode,
        ) -> Result<StructType> {
            if matches!(newtype, NewtypeMode::NoParensMeanUnit) && !parser.consume_char('(') {
                return Ok(StructType::Unit);
            }

            parser.skip_ws()?;

            if parser.identifier().is_ok() {
                parser.skip_ws()?;

                match parser.peek() {
                    // Definitely a struct with named fields
                    Ok(':') => return Ok(StructType::Named),
                    // Definitely a tuple struct with fields
                    Ok(',') => return Ok(StructType::Tuple),
                    // Either a newtype or a tuple struct
                    Ok(')') => return Ok(StructType::NewtypeOrTuple),
                    // Something else, let's investigate further
                    _ => (),
                };
            }

            if matches!(tuple, TupleMode::ImpreciseTupleOrNewtype) {
                return Ok(StructType::NewtypeOrTuple);
            }

            let mut braces = 1_usize;
            let mut comma = false;

            // Skip ahead to see if the value is followed by a comma
            while braces > 0 {
                // Skip spurious braces in comments, strings, and characters
                parser.skip_ws()?;
                let mut bytes_copy = *parser;
                if bytes_copy.char().is_ok() {
                    *parser = bytes_copy;
                }
                let mut bytes_copy = *parser;
                if bytes_copy.string().is_ok() {
                    *parser = bytes_copy;
                }
                let mut bytes_copy = *parser;
                if bytes_copy.byte_string().is_ok() {
                    *parser = bytes_copy;
                }

                let c = parser.next()?;
                if matches!(c, '(' | '[' | '{') {
                    braces += 1;
                } else if matches!(c, ')' | ']' | '}') {
                    braces -= 1;
                } else if c == ',' && braces == 1 {
                    comma = true;
                    break;
                }
            }

            if comma {
                Ok(StructType::Tuple)
            } else {
                Ok(StructType::NewtypeOrTuple)
            }
        }

        // Create a temporary working copy
        let mut bytes = *self;

        let result = check_struct_type_inner(&mut bytes, newtype, tuple);

        if result.is_err() {
            // Adjust the error span to fit the working copy
            *self = bytes;
        }

        result
    }

    /// Only returns true if the char after `ident` cannot belong
    /// to an identifier.
    pub fn consume_ident(&mut self, ident: &str) -> bool {
        if self.check_ident(ident) {
            let _ = self.advance(ident.len());

            true
        } else {
            false
        }
    }

    pub fn consume_struct_name(&mut self, ident: &'static str) -> Result<bool> {
        if self.check_ident("") {
            return Ok(false);
        }

        let found_ident = match self.identifier() {
            Ok(maybe_ident) => maybe_ident,
            Err(Error::SuggestRawIdentifier(found_ident)) if found_ident == ident => {
                return Err(Error::SuggestRawIdentifier(found_ident))
            }
            Err(_) => return Err(Error::ExpectedNamedStructLike(ident)),
        };

        if found_ident != ident {
            return Err(Error::ExpectedDifferentStructName {
                expected: ident,
                found: String::from(found_ident),
            });
        }

        Ok(true)
    }

    /// Returns the extensions bit mask.
    fn extensions(&mut self) -> Result<Extensions> {
        if self.peek() != Ok('#') {
            return Ok(Extensions::empty());
        }

        if !self.consume_all(&["#", "!", "[", "enable", "("])? {
            return Err(Error::ExpectedAttribute);
        }

        self.skip_ws()?;
        let mut extensions = Extensions::empty();

        loop {
            let ident = self.identifier()?;
            let extension = Extensions::from_ident(ident)
                .ok_or_else(|| Error::NoSuchExtension(ident.into()))?;

            extensions |= extension;

            let comma = self.comma()?;

            // If we have no comma but another item, return an error
            if !comma && self.check_ident_other_char(0) {
                return Err(Error::ExpectedComma);
            }

            // If there's no comma, assume the list ended.
            // If there is, it might be a trailing one, thus we only
            // continue the loop if we get an ident char.
            if !comma || !self.check_ident_other_char(0) {
                break;
            }
        }

        self.skip_ws()?;

        if self.consume_all(&[")", "]"])? {
            Ok(extensions)
        } else {
            Err(Error::ExpectedAttributeEnd)
        }
    }

    pub fn float<T: Float>(&mut self) -> Result<T> {
        const F32_SUFFIX: &str = "f32";
        const F64_SUFFIX: &str = "f64";

        for (literal, value_f32, value_f64) in &[
            ("inf", f32::INFINITY, f64::INFINITY),
            ("+inf", f32::INFINITY, f64::INFINITY),
            ("-inf", f32::NEG_INFINITY, f64::NEG_INFINITY),
            ("NaN", f32::NAN, f64::NAN),
            ("+NaN", f32::NAN, f64::NAN),
            ("-NaN", -f32::NAN, -f64::NAN),
        ] {
            if self.consume_ident(literal) {
                return T::parse(literal);
            }

            if self.src.starts_with(literal)
                && self.src[literal.len()..].starts_with(F32_SUFFIX)
                && !self.check_ident_other_char(literal.len() + F32_SUFFIX.len())
            {
                let float_ron = &self.src[..literal.len() + F32_SUFFIX.len()];
                let _ = self.advance(literal.len() + F32_SUFFIX.len());
                return T::try_from_parsed_float(ParsedFloat::F32(*value_f32), float_ron);
            }

            if self.src.starts_with(literal)
                && self.src[literal.len()..].starts_with(F64_SUFFIX)
                && !self.check_ident_other_char(literal.len() + F64_SUFFIX.len())
            {
                let float_ron = &self.src[..literal.len() + F64_SUFFIX.len()];
                let _ = self.advance(literal.len() + F64_SUFFIX.len());
                return T::try_from_parsed_float(ParsedFloat::F64(*value_f64), float_ron);
            }
        }

        let num_bytes = self.next_bytes_while(is_float_char);

        if num_bytes == 0 {
            return Err(Error::ExpectedFloat);
        }

        if self.peek_char_or_eof()? == '_' {
            return Err(Error::UnderscoreAtBeginning);
        }

        let mut f = String::with_capacity(num_bytes);
        let mut allow_underscore = false;

        for (i, b) in self.src.as_bytes()[0..num_bytes].iter().enumerate() {
            match *b {
                b'_' if allow_underscore => continue,
                b'_' => {
                    let _ = self.advance(i);
                    return Err(Error::FloatUnderscore);
                }
                b'0'..=b'9' | b'e' | b'E' => allow_underscore = true,
                b'.' => allow_underscore = false,
                _ => (),
            }

            // we know that the byte is an ASCII character here
            f.push(char::from(*b));
        }

        if self.src[num_bytes..].starts_with('f') {
            let parser_backup = *self;
            let _ = self.advance(num_bytes);

            #[allow(clippy::never_loop)]
            loop {
                let res = if self.consume_ident(F32_SUFFIX) {
                    f32::from_str(&f).map(ParsedFloat::F32)
                } else if self.consume_ident(F64_SUFFIX) {
                    f64::from_str(&f).map(ParsedFloat::F64)
                } else {
                    break;
                };

                let parsed = match res {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        *self = parser_backup;
                        return Err(Error::ExpectedFloat);
                    }
                };

                let float_ron = &parser_backup.src[..parser_backup.src.len() - self.src.len()];

                return T::try_from_parsed_float(parsed, float_ron);
            }

            *self = parser_backup;
        }

        let value = T::parse(&f)?;

        let _ = self.advance(num_bytes);

        Ok(value)
    }

    pub fn identifier(&mut self) -> Result<&'a str> {
        let first = self.peek()?;
        if !is_ident_first_char(first) {
            if is_ident_raw_char(first) {
                let ident_bytes = self.next_chars_while(is_ident_raw_char);
                return Err(Error::SuggestRawIdentifier(self.src[..ident_bytes].into()));
            }

            return Err(Error::ExpectedIdentifier);
        }

        // If the next two bytes signify the start of a (raw) byte string
        //  literal, return an error.
        if first == 'b' {
            match self.src.chars().nth(1) {
                Some('"' | '\'') => return Err(Error::ExpectedIdentifier),
                Some('r') => match self.src.chars().nth(2) {
                    Some('#' | '"') => return Err(Error::ExpectedIdentifier),
                    Some(_) | None => (),
                },
                Some(_) | None => (),
            }
        };

        let length = if first == 'r' {
            match self.src.chars().nth(1) {
                Some('"') => return Err(Error::ExpectedIdentifier),
                Some('#') => {
                    let after_next = self.src.chars().nth(2).unwrap_or_default();
                    // Note: it's important to check this before advancing forward, so that
                    // the value-type deserializer can fall back to parsing it differently.
                    if !is_ident_raw_char(after_next) {
                        return Err(Error::ExpectedIdentifier);
                    }
                    // skip "r#"
                    let _ = self.advance(2);
                    self.next_chars_while(is_ident_raw_char)
                }
                _ => {
                    let std_ident_length = self.next_chars_while(is_xid_continue);
                    let raw_ident_length = self.next_chars_while(is_ident_raw_char);

                    if raw_ident_length > std_ident_length {
                        return Err(Error::SuggestRawIdentifier(
                            self.src[..raw_ident_length].into(),
                        ));
                    }

                    std_ident_length
                }
            }
        } else {
            let std_ident_length =
                first.len_utf8() + self.next_chars_while_from(first.len_utf8(), is_xid_continue);
            let raw_ident_length = self.next_chars_while(is_ident_raw_char);

            if raw_ident_length > std_ident_length {
                return Err(Error::SuggestRawIdentifier(
                    self.src[..raw_ident_length].into(),
                ));
            }

            std_ident_length
        };

        let ident = &self.src[..length];
        let _ = self.advance(length);

        Ok(ident)
    }

    pub fn next_bytes_is_float(&self) -> bool {
        if let Ok(c) = self.peek() {
            let skip = match c {
                '+' | '-' => 1,
                _ => 0,
            };
            let flen = self.next_bytes_while_from(skip, is_float_char);
            let ilen = self.next_bytes_while_from(skip, is_int_char);
            flen > ilen
        } else {
            false
        }
    }

    pub fn skip_ws(&mut self) -> Result<()> {
        if self.src.is_empty() {
            return Ok(());
        }

        if (self.src.len() + self.last_ws_len) < self.pre_ws_src.len() {
            // [[last whitespace] ... [src]] means the last whitespace
            //  is disjoint from this one and we need to reset the pre ws
            self.pre_ws_src = self.src;
        }

        loop {
            while self.peek().map_or(false, is_whitespace_char) {
                let _ = self.advance_char();
            }

            match self.skip_comment()? {
                None => break,
                Some(Comment::UnclosedLine) => {
                    self.last_ws_len = usize::MAX;
                    return Ok(());
                }
                Some(Comment::ClosedLine | Comment::Block) => continue,
            }
        }

        self.last_ws_len = self.pre_ws_src.len() - self.src.len();

        Ok(())
    }

    pub fn has_unclosed_line_comment(&self) -> bool {
        self.src.is_empty() && self.last_ws_len == usize::MAX
    }

    pub fn byte_string(&mut self) -> Result<ParsedByteStr<'a>> {
        fn expected_byte_string_found_base64(
            base64_str: &ParsedStr,
            byte_str: &ParsedByteStr,
        ) -> Error {
            let byte_str = match &byte_str {
                ParsedByteStr::Allocated(b) => b.as_slice(),
                ParsedByteStr::Slice(b) => b,
            }
            .iter()
            .flat_map(|c| std::ascii::escape_default(*c))
            .map(char::from)
            .collect::<String>();
            let base64_str = match &base64_str {
                ParsedStr::Allocated(s) => s.as_str(),
                ParsedStr::Slice(s) => s,
            };

            Error::InvalidValueForType {
                expected: format!("the Rusty byte string b\"{}\"", byte_str),
                found: format!("the ambiguous base64 string {:?}", base64_str),
            }
        }

        if self.consume_char('"') {
            let base64_str = self.escaped_string()?;
            let base64_result = ParsedByteStr::try_from_base64(base64_str.clone());

            if cfg!(not(test)) {
                // FIXME @juntyr: remove in v0.10
                #[allow(deprecated)]
                base64_result.map_err(Error::Base64Error)
            } else {
                match base64_result {
                    // FIXME @juntyr: enable in v0.10
                    Ok(byte_str) => Err(expected_byte_string_found_base64(&base64_str, &byte_str)),
                    Err(_) => Err(Error::ExpectedByteString),
                }
            }
        } else if self.consume_char('r') {
            let base64_str = self.raw_string()?;
            let base64_result = ParsedByteStr::try_from_base64(base64_str.clone());

            if cfg!(not(test)) {
                // FIXME @juntyr: remove in v0.10
                #[allow(deprecated)]
                base64_result.map_err(Error::Base64Error)
            } else {
                match base64_result {
                    // FIXME @juntyr: enable in v0.10
                    Ok(byte_str) => Err(expected_byte_string_found_base64(&base64_str, &byte_str)),
                    Err(_) => Err(Error::ExpectedByteString),
                }
            }
        } else if self.consume_str("b\"") {
            self.escaped_byte_string()
        } else if self.consume_str("br") {
            self.raw_byte_string()
        } else {
            Err(Error::ExpectedByteString)
        }
    }

    fn escaped_byte_string(&mut self) -> Result<ParsedByteStr<'a>> {
        match self.escaped_byte_buf(EscapeEncoding::Binary) {
            Ok((bytes, advance)) => {
                let _ = self.advance(advance);
                Ok(bytes)
            }
            Err(err) => Err(err),
        }
    }

    fn raw_byte_string(&mut self) -> Result<ParsedByteStr<'a>> {
        match self.raw_byte_buf() {
            Ok((bytes, advance)) => {
                let _ = self.advance(advance);
                Ok(bytes)
            }
            Err(Error::ExpectedString) => Err(Error::ExpectedByteString),
            Err(err) => Err(err),
        }
    }

    pub fn string(&mut self) -> Result<ParsedStr<'a>> {
        if self.consume_str("\"") {
            self.escaped_string()
        } else if self.consume_str("r") {
            self.raw_string()
        } else {
            Err(Error::ExpectedString)
        }
    }

    fn escaped_string(&mut self) -> Result<ParsedStr<'a>> {
        match self.escaped_byte_buf(EscapeEncoding::Utf8) {
            Ok((bytes, advance)) => {
                let string = ParsedStr::try_from_bytes(bytes).map_err(Error::from)?;
                let _ = self.advance(advance);
                Ok(string)
            }
            Err(err) => Err(err),
        }
    }

    fn raw_string(&mut self) -> Result<ParsedStr<'a>> {
        match self.raw_byte_buf() {
            Ok((bytes, advance)) => {
                let string = ParsedStr::try_from_bytes(bytes).map_err(Error::from)?;
                let _ = self.advance(advance);
                Ok(string)
            }
            Err(err) => Err(err),
        }
    }

    fn escaped_byte_buf(&mut self, encoding: EscapeEncoding) -> Result<(ParsedByteStr<'a>, usize)> {
        use std::iter::repeat;

        let (i, end_or_escape) = self
            .bytes()
            .iter()
            .enumerate()
            .find(|&(_, &b)| b == b'\\' || b == b'"')
            .ok_or(Error::ExpectedStringEnd)?;

        if *end_or_escape == b'"' {
            let s = &self.src.as_bytes()[..i];

            // Advance by the number of bytes of the string
            // + 1 for the `"`.
            Ok((ParsedByteStr::Slice(s), i + 1))
        } else {
            let mut i = i;
            let mut s = self.src.as_bytes()[..i].to_vec();

            loop {
                let _ = self.advance(i + 1);

                match self.parse_escape(encoding, false)? {
                    EscapeCharacter::Ascii(c) => s.push(c),
                    EscapeCharacter::Utf8(c) => match c.len_utf8() {
                        1 => s.push(c as u8),
                        len => {
                            let start = s.len();
                            s.extend(repeat(0).take(len));
                            c.encode_utf8(&mut s[start..]);
                        }
                    },
                }

                let (new_i, end_or_escape) = self
                    .bytes()
                    .iter()
                    .enumerate()
                    .find(|&(_, &b)| b == b'\\' || b == b'"')
                    .ok_or(Error::ExpectedStringEnd)?;

                i = new_i;
                s.extend_from_slice(&self.src.as_bytes()[..i]);

                if *end_or_escape == b'"' {
                    // Advance to the end of the string + 1 for the `"`.
                    break Ok((ParsedByteStr::Allocated(s), i + 1));
                }
            }
        }
    }

    fn raw_byte_buf(&mut self) -> Result<(ParsedByteStr<'a>, usize)> {
        let num_hashes = self.next_bytes_while(|b| b == b'#');
        let hashes = &self.src[..num_hashes];
        let _ = self.advance(num_hashes);

        self.expect_char('"', Error::ExpectedString)?;

        let ending = ["\"", hashes].concat();
        let i = self.src.find(&ending).ok_or(Error::ExpectedStringEnd)?;

        let s = &self.src.as_bytes()[..i];

        // Advance by the number of bytes of the byte string
        // + `num_hashes` + 1 for the `"`.
        Ok((ParsedByteStr::Slice(s), i + num_hashes + 1))
    }

    fn decode_ascii_escape(&mut self) -> Result<u8> {
        let mut n = 0;
        for _ in 0..2 {
            n <<= 4;
            let byte = self.next()?;
            let decoded = self.decode_hex(byte)?;
            n |= decoded;
        }

        Ok(n)
    }

    #[inline]
    fn decode_hex(&self, c: char) -> Result<u8> {
        if !c.is_ascii() {
            return Err(Error::InvalidEscape("Non-hex digit found"));
        }

        match c as u8 {
            c @ b'0'..=b'9' => Ok(c - b'0'),
            c @ b'a'..=b'f' => Ok(10 + c - b'a'),
            c @ b'A'..=b'F' => Ok(10 + c - b'A'),
            _ => Err(Error::InvalidEscape("Non-hex digit found")),
        }
    }

    fn parse_escape(&mut self, encoding: EscapeEncoding, is_char: bool) -> Result<EscapeCharacter> {
        let c = match self.next()? {
            '\'' => EscapeCharacter::Ascii(b'\''),
            '"' => EscapeCharacter::Ascii(b'"'),
            '\\' => EscapeCharacter::Ascii(b'\\'),
            'n' => EscapeCharacter::Ascii(b'\n'),
            'r' => EscapeCharacter::Ascii(b'\r'),
            't' => EscapeCharacter::Ascii(b'\t'),
            '0' => EscapeCharacter::Ascii(b'\0'),
            'x' => {
                // Fast exit for ascii escape in byte string
                let b: u8 = self.decode_ascii_escape()?;
                if let EscapeEncoding::Binary = encoding {
                    return Ok(EscapeCharacter::Ascii(b));
                }

                // Fast exit for ascii character in UTF-8 string
                let mut bytes = [b, 0, 0, 0];
                if let Ok(Some(c)) = from_utf8(&bytes[..=0]).map(|s| s.chars().next()) {
                    return Ok(EscapeCharacter::Utf8(c));
                }

                if is_char {
                    // Character literals are not allowed to use multiple byte
                    //  escapes to build a unicode character
                    return Err(Error::InvalidEscape(
                        "Not a valid byte-escaped Unicode character",
                    ));
                }

                // UTF-8 character needs up to four bytes and we have already
                //  consumed one, so at most three to go
                for i in 1..4 {
                    if !self.consume_str(r"\x") {
                        return Err(Error::InvalidEscape(
                            "Not a valid byte-escaped Unicode character",
                        ));
                    }

                    bytes[i] = self.decode_ascii_escape()?;

                    // Check if we now have a valid UTF-8 character
                    if let Ok(Some(c)) = from_utf8(&bytes[..=i]).map(|s| s.chars().next()) {
                        return Ok(EscapeCharacter::Utf8(c));
                    }
                }

                return Err(Error::InvalidEscape(
                    "Not a valid byte-escaped Unicode character",
                ));
            }
            'u' => {
                self.expect_char('{', Error::InvalidEscape("Missing { in Unicode escape"))?;

                let mut bytes: u32 = 0;
                let mut num_digits = 0;

                while num_digits < 6 {
                    let byte = self.peek_char_or_eof()?;

                    if byte == '}' {
                        break;
                    } else {
                        num_digits += self.advance_char()?;
                    }

                    let byte = self.decode_hex(byte)?;
                    bytes <<= 4;
                    bytes |= u32::from(byte);
                }

                if num_digits == 0 {
                    return Err(Error::InvalidEscape(
                        "Expected 1-6 digits, got 0 digits in Unicode escape",
                    ));
                }

                self.expect_char(
                    '}',
                    Error::InvalidEscape("No } at the end of Unicode escape"),
                )?;
                let c = char_from_u32(bytes).ok_or(Error::InvalidEscape(
                    "Not a valid Unicode-escaped character",
                ))?;

                EscapeCharacter::Utf8(c)
            }
            _ => return Err(Error::InvalidEscape("Unknown escape character")),
        };

        Ok(c)
    }

    fn skip_comment(&mut self) -> Result<Option<Comment>> {
        if self.consume_char('/') {
            match self.next()? {
                '/' => {
                    let bytes = self.next_bytes_while(|b| b != b'\n');

                    let _ = self.advance(bytes);

                    if self.bytes().is_empty() {
                        Ok(Some(Comment::UnclosedLine))
                    } else {
                        Ok(Some(Comment::ClosedLine))
                    }
                }
                '*' => {
                    let mut level = 1;

                    while level > 0 {
                        let bytes = self.next_bytes_while(|b| !matches!(b, b'/' | b'*'));

                        if self.src.is_empty() {
                            return Err(Error::UnclosedBlockComment);
                        }

                        let _ = self.advance(bytes);

                        // check whether / or * and take action
                        if self.consume_str("/*") {
                            level += 1;
                        } else if self.consume_str("*/") {
                            level -= 1;
                        } else {
                            self.advance_char()
                                .map_err(|_| Error::UnclosedBlockComment)?;
                        }
                    }

                    Ok(Some(Comment::Block))
                }
                c => Err(Error::UnexpectedChar(c)),
            }
        } else {
            Ok(None)
        }
    }
}

enum Comment {
    ClosedLine,
    UnclosedLine,
    Block,
}

pub trait Num {
    fn from_u8(x: u8) -> Self;

    /// Returns `true` on overflow
    fn checked_mul_ext(&mut self, x: u8) -> bool;

    /// Returns `true` on overflow
    fn checked_add_ext(&mut self, x: u8) -> bool;

    /// Returns `true` on overflow
    fn checked_sub_ext(&mut self, x: u8) -> bool;
}

macro_rules! impl_num {
    ($ty:ty) => {
        impl Num for $ty {
            fn from_u8(x: u8) -> Self {
                x as $ty
            }

            fn checked_mul_ext(&mut self, x: u8) -> bool {
                match self.checked_mul(Self::from_u8(x)) {
                    Some(n) => {
                        *self = n;
                        false
                    }
                    None => true,
                }
            }

            fn checked_add_ext(&mut self, x: u8) -> bool {
                match self.checked_add(Self::from_u8(x)) {
                    Some(n) => {
                        *self = n;
                        false
                    }
                    None => true,
                }
            }

            fn checked_sub_ext(&mut self, x: u8) -> bool {
                match self.checked_sub(Self::from_u8(x)) {
                    Some(n) => {
                        *self = n;
                        false
                    }
                    None => true,
                }
            }
        }
    };
    ($($tys:ty)*) => {
        $( impl_num!($tys); )*
    };
}

impl_num! { i8 i16 i32 i64 u8 u16 u32 u64 }

#[cfg(feature = "integer128")]
impl_num! { i128 u128 }

pub trait Integer: Sized {
    fn parse(parser: &mut Parser, sign: i8) -> Result<Self>;

    fn try_from_parsed_integer(parsed: ParsedInteger, ron: &str) -> Result<Self>;
}

macro_rules! impl_integer {
    ($wrap:ident($ty:ty)) => {
        impl Integer for $ty {
            fn parse(parser: &mut Parser, sign: i8) -> Result<Self> {
                parser.parse_integer(sign)
            }

            fn try_from_parsed_integer(parsed: ParsedInteger, ron: &str) -> Result<Self> {
                match parsed {
                    ParsedInteger::$wrap(v) => Ok(v),
                    _ => Err(Error::InvalidValueForType {
                        expected: format!(
                            "a{} {}-bit {}signed integer",
                            if <$ty>::BITS == 8 { "n" } else { "n" },
                            <$ty>::BITS,
                            if <$ty>::MIN == 0 { "un" } else { "" },
                        ),
                        found: String::from(ron),
                    }),
                }
            }
        }
    };
    ($($wraps:ident($tys:ty))*) => {
        $( impl_integer!($wraps($tys)); )*
    };
}

impl_integer! {
    I8(i8) I16(i16) I32(i32) I64(i64)
    U8(u8) U16(u16) U32(u32) U64(u64)
}

#[cfg(feature = "integer128")]
impl_integer! { I128(i128) U128(u128) }

pub enum ParsedInteger {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    #[cfg(feature = "integer128")]
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    #[cfg(feature = "integer128")]
    U128(u128),
}

impl Integer for ParsedInteger {
    fn parse(parser: &mut Parser, sign: i8) -> Result<Self> {
        if sign < 0 {
            let signed = parser.parse_integer::<LargeSInt>(-1)?;

            return if let Ok(x) = i8::try_from(signed) {
                Ok(ParsedInteger::I8(x))
            } else if let Ok(x) = i16::try_from(signed) {
                Ok(ParsedInteger::I16(x))
            } else if let Ok(x) = i32::try_from(signed) {
                Ok(ParsedInteger::I32(x))
            } else {
                #[cfg(not(feature = "integer128"))]
                {
                    Ok(ParsedInteger::I64(signed))
                }
                #[cfg(feature = "integer128")]
                if let Ok(x) = i64::try_from(signed) {
                    Ok(ParsedInteger::I64(x))
                } else {
                    Ok(ParsedInteger::I128(signed))
                }
            };
        }

        let unsigned = parser.parse_integer::<LargeUInt>(1)?;

        if let Ok(x) = u8::try_from(unsigned) {
            Ok(ParsedInteger::U8(x))
        } else if let Ok(x) = u16::try_from(unsigned) {
            Ok(ParsedInteger::U16(x))
        } else if let Ok(x) = u32::try_from(unsigned) {
            Ok(ParsedInteger::U32(x))
        } else {
            #[cfg(not(feature = "integer128"))]
            {
                Ok(ParsedInteger::U64(unsigned))
            }
            #[cfg(feature = "integer128")]
            if let Ok(x) = u64::try_from(unsigned) {
                Ok(ParsedInteger::U64(x))
            } else {
                Ok(ParsedInteger::U128(unsigned))
            }
        }
    }

    fn try_from_parsed_integer(parsed: ParsedInteger, _ron: &str) -> Result<Self> {
        Ok(parsed)
    }
}

pub trait Float: Sized {
    fn parse(float: &str) -> Result<Self>;

    fn try_from_parsed_float(parsed: ParsedFloat, ron: &str) -> Result<Self>;
}

macro_rules! impl_float {
    ($wrap:ident($ty:ty: $bits:expr)) => {
        impl Float for $ty {
            fn parse(float: &str) -> Result<Self> {
                <$ty>::from_str(float).map_err(|_| Error::ExpectedFloat)
            }

            fn try_from_parsed_float(parsed: ParsedFloat, ron: &str) -> Result<Self> {
                match parsed {
                    ParsedFloat::$wrap(v) => Ok(v),
                    _ => Err(Error::InvalidValueForType {
                        expected: format!(
                            "a {}-bit floating point number", $bits,
                        ),
                        found: String::from(ron),
                    }),
                }
            }
        }
    };
    ($($wraps:ident($tys:ty: $bits:expr))*) => {
        $( impl_float!($wraps($tys: $bits)); )*
    };
}

impl_float! { F32(f32: 32) F64(f64: 64) }

pub enum ParsedFloat {
    F32(f32),
    F64(f64),
}

impl Float for ParsedFloat {
    fn parse(float: &str) -> Result<Self> {
        let value = f64::from_str(float).map_err(|_| Error::ExpectedFloat)?;

        if value.total_cmp(&f64::from(value as f32)).is_eq() {
            Ok(ParsedFloat::F32(value as f32))
        } else {
            Ok(ParsedFloat::F64(value))
        }
    }

    fn try_from_parsed_float(parsed: ParsedFloat, _ron: &str) -> Result<Self> {
        Ok(parsed)
    }
}

pub enum StructType {
    NewtypeOrTuple,
    Tuple,
    Named,
    Unit,
}

pub enum NewtypeMode {
    NoParensMeanUnit,
    InsideNewtype,
}

pub enum TupleMode {
    ImpreciseTupleOrNewtype,
    DifferentiateNewtype,
}

#[derive(Clone)]
pub enum ParsedStr<'a> {
    Allocated(String),
    Slice(&'a str),
}

pub enum ParsedByteStr<'a> {
    Allocated(Vec<u8>),
    Slice(&'a [u8]),
}

impl<'a> ParsedStr<'a> {
    pub fn try_from_bytes(bytes: ParsedByteStr<'a>) -> Result<Self, Utf8Error> {
        match bytes {
            ParsedByteStr::Allocated(byte_buf) => Ok(ParsedStr::Allocated(
                String::from_utf8(byte_buf).map_err(|e| e.utf8_error())?,
            )),
            ParsedByteStr::Slice(bytes) => Ok(ParsedStr::Slice(from_utf8(bytes)?)),
        }
    }
}

impl<'a> ParsedByteStr<'a> {
    pub fn try_from_base64(str: ParsedStr<'a>) -> Result<Self, base64::DecodeError> {
        let base64_str = match &str {
            ParsedStr::Allocated(string) => string.as_str(),
            ParsedStr::Slice(str) => str,
        };

        base64::engine::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_str)
            .map(ParsedByteStr::Allocated)
    }
}

#[derive(Copy, Clone)] // GRCOV_EXCL_LINE
enum EscapeEncoding {
    Binary,
    Utf8,
}

enum EscapeCharacter {
    Ascii(u8),
    Utf8(char),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_x10() {
        let mut bytes = Parser::new("10").unwrap();
        assert_eq!(bytes.decode_ascii_escape(), Ok(b'\x10'));
    }

    #[test]
    fn track_prior_ws() {
        const SOURCE: &str = "   /*hey*/ 42       /*bye*/ 24  ";
        let mut bytes = Parser::new(SOURCE).unwrap();

        assert_eq!(bytes.source(), "42       /*bye*/ 24  ");
        assert_eq!(bytes.pre_ws_source(), SOURCE);

        bytes.skip_ws().unwrap();

        assert_eq!(bytes.source(), "42       /*bye*/ 24  ");
        assert_eq!(bytes.pre_ws_source(), SOURCE);

        assert_eq!(bytes.integer::<u8>().unwrap(), 42);

        assert_eq!(bytes.source(), "       /*bye*/ 24  ");
        assert_eq!(bytes.pre_ws_source(), SOURCE);

        bytes.skip_ws().unwrap();
        bytes.skip_ws().unwrap();

        assert_eq!(bytes.source(), "24  ");
        assert_eq!(bytes.pre_ws_source(), "       /*bye*/ 24  ");
    }

    #[test]
    fn v0_10_base64_deprecation_error() {
        let err = crate::from_str::<bytes::Bytes>("\"SGVsbG8gcm9uIQ==\"").unwrap_err();

        assert_eq!(
            err,
            SpannedError {
                code: Error::InvalidValueForType {
                    expected: String::from("the Rusty byte string b\"Hello ron!\""),
                    found: String::from("the ambiguous base64 string \"SGVsbG8gcm9uIQ==\"")
                },
                position: Position { line: 1, col: 19 },
            }
        );

        let err = crate::from_str::<bytes::Bytes>("r\"SGVsbG8gcm9uIQ==\"").unwrap_err();

        assert_eq!(format!("{}", err.code), "Expected the Rusty byte string b\"Hello ron!\" but found the ambiguous base64 string \"SGVsbG8gcm9uIQ==\" instead");

        assert_eq!(
            crate::from_str::<bytes::Bytes>("\"invalid=\"").unwrap_err(),
            SpannedError {
                code: Error::ExpectedByteString,
                position: Position { line: 1, col: 11 },
            }
        );

        assert_eq!(
            crate::from_str::<bytes::Bytes>("r\"invalid=\"").unwrap_err(),
            SpannedError {
                code: Error::ExpectedByteString,
                position: Position { line: 1, col: 12 },
            }
        );
    }
}
