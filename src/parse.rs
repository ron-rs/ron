#![allow(clippy::identity_op)]

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    char::from_u32 as char_from_u32,
    str::{self, from_utf8, FromStr, Utf8Error},
};

use unicode_ident::{is_xid_continue, is_xid_start};

use crate::{
    error::{Error, Position, Result, Span, SpannedError, SpannedResult},
    extensions::Extensions,
    value::Number,
};

const fn is_int_char(c: char) -> bool {
    c.is_ascii_hexdigit() || c == '_'
}

const fn is_float_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, 'e' | 'E' | '.' | '+' | '-' | '_')
}

pub fn is_ident_first_char(c: char) -> bool {
    c == '_' || is_xid_start(c)
}

pub fn is_ident_raw_char(c: char) -> bool {
    matches!(c, '.' | '+' | '-') | is_xid_continue(c)
}

pub const fn is_whitespace_char(c: char) -> bool {
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

#[cfg(feature = "integer128")]
pub(crate) type LargeUInt = u128;
#[cfg(not(feature = "integer128"))]
pub(crate) type LargeUInt = u64;
#[cfg(feature = "integer128")]
pub(crate) type LargeSInt = i128;
#[cfg(not(feature = "integer128"))]
pub(crate) type LargeSInt = i64;

pub struct Parser<'a> {
    /// Bits set according to the [`Extensions`] enum.
    pub exts: Extensions,
    src: &'a str,
    cursor: ParserCursor,
    prev_cursor: ParserCursor,
}

#[derive(Copy, Clone)] // GRCOV_EXCL_LINE
pub struct ParserCursor {
    cursor: usize,
    pre_ws_cursor: usize,
    last_ws_len: usize,
}

enum ParsedAttribute {
    None,
    Extensions(Extensions),
    Ignored,
}

const WS_CURSOR_UNCLOSED_LINE: usize = usize::MAX;

impl PartialEq for ParserCursor {
    fn eq(&self, other: &Self) -> bool {
        self.cursor == other.cursor
    }
}

impl PartialOrd for ParserCursor {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.cursor.partial_cmp(&other.cursor)
    }
}

/// constructor and parsing utilities
impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> SpannedResult<Self> {
        let mut parser = Parser {
            exts: Extensions::empty(),
            src,
            cursor: ParserCursor {
                cursor: 0,
                pre_ws_cursor: 0,
                last_ws_len: 0,
            },
            prev_cursor: ParserCursor {
                cursor: 0,
                pre_ws_cursor: 0,
                last_ws_len: 0,
            },
        };

        parser.skip_ws().map_err(|e| parser.span_error(e))?;

        // Loop over all document attributes
        loop {
            match parser.attribute().map_err(|e| parser.span_error(e))? {
                ParsedAttribute::None => break,
                ParsedAttribute::Extensions(extensions) => {
                    parser.exts |= extensions;
                }
                ParsedAttribute::Ignored => {}
            }

            parser.skip_ws().map_err(|e| parser.span_error(e))?;
        }

        Ok(parser)
    }

    fn set_cursor(&mut self, cursor: ParserCursor) {
        self.cursor = cursor;
    }

    pub fn span_error(&self, code: Error) -> SpannedError {
        SpannedError {
            code,
            span: Span {
                start: Position::from_src_end(&self.src[..self.prev_cursor.cursor]),
                end: Position::from_src_end(&self.src[..self.cursor.cursor]),
            },
        }
    }

    pub fn advance_bytes(&mut self, bytes: usize) {
        self.prev_cursor = self.cursor;
        self.cursor.cursor += bytes;
    }

    pub fn next_char(&mut self) -> Result<char> {
        let c = self.peek_char_or_eof()?;
        self.cursor.cursor += c.len_utf8();
        Ok(c)
    }

    pub fn skip_next_char(&mut self) {
        core::mem::drop(self.next_char());
    }

    pub fn peek_char(&self) -> Option<char> {
        self.src().chars().next()
    }

    pub fn peek_char_or_eof(&self) -> Result<char> {
        self.peek_char().ok_or(Error::Eof)
    }

    pub fn check_char(&self, c: char) -> bool {
        self.src().starts_with(c)
    }

    pub fn check_str(&self, s: &str) -> bool {
        self.src().starts_with(s)
    }

    pub fn src(&self) -> &'a str {
        &self.src[self.cursor.cursor..]
    }

    pub fn pre_ws_src(&self) -> &'a str {
        &self.src[self.cursor.pre_ws_cursor..]
    }

    pub fn consume_str(&mut self, s: &str) -> bool {
        if self.check_str(s) {
            self.advance_bytes(s.len());

            true
        } else {
            false
        }
    }

    pub fn consume_char(&mut self, c: char) -> bool {
        if self.check_char(c) {
            self.advance_bytes(c.len_utf8());

            true
        } else {
            false
        }
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
            .try_fold(true, |acc, x| x.map(|x| x && acc))
    }

    pub fn expect_char(&mut self, expected: char, error: Error) -> Result<()> {
        if self.consume_char(expected) {
            Ok(())
        } else {
            Err(error)
        }
    }

    #[must_use]
    pub fn next_chars_while_len(&self, condition: fn(char) -> bool) -> usize {
        self.next_chars_while_from_len(0, condition)
    }

    #[must_use]
    pub fn next_chars_while_from_len(&self, from: usize, condition: fn(char) -> bool) -> usize {
        self.src()[from..]
            .find(|c| !condition(c))
            .unwrap_or(self.src().len() - from)
    }
}

/// actual parsing of ron tokens
impl<'a> Parser<'a> {
    fn parse_integer_digits<T: Num>(
        &mut self,
        s: &str,
        base: u8,
        f: fn(&mut T, u8) -> bool,
    ) -> Result<T> {
        let mut num_acc = T::from_u8(0);

        for (i, c) in s.char_indices() {
            if c == '_' {
                continue;
            }

            if num_acc.checked_mul_ext(base) {
                self.advance_bytes(s.len());
                return Err(Error::IntegerOutOfBounds);
            }

            let digit = Self::decode_hex(c)?;

            if digit >= base {
                self.advance_bytes(i);
                return Err(Error::InvalidIntegerDigit { digit: c, base });
            }

            if f(&mut num_acc, digit) {
                self.advance_bytes(s.len());
                return Err(Error::IntegerOutOfBounds);
            }
        }

        self.advance_bytes(s.len());

        Ok(num_acc)
    }

    fn parse_integer<T: Num>(&mut self, sign: i8) -> Result<T> {
        let base = match () {
            () if self.consume_str("0b") => 2,
            () if self.consume_str("0o") => 8,
            () if self.consume_str("0x") => 16,
            () => 10,
        };

        let num_bytes = self.next_chars_while_len(is_int_char);

        if num_bytes == 0 {
            return Err(Error::ExpectedInteger);
        }

        if self.check_char('_') {
            return Err(Error::UnderscoreAtBeginning);
        }

        let s = &self.src()[..num_bytes];

        if sign > 0 {
            self.parse_integer_digits(s, base, T::checked_add_ext)
        } else {
            self.parse_integer_digits(s, base, T::checked_sub_ext)
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn integer<T: Integer>(&mut self) -> Result<T> {
        let src_backup = self.src();

        let is_negative = match self.peek_char_or_eof()? {
            '+' => {
                self.skip_next_char();
                false
            }
            '-' => {
                self.skip_next_char();
                true
            }
            'b' if self.consume_str("b'") => {
                // Parse a byte literal
                let byte = match self.next_char()? {
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

                let bytes_ron = &src_backup[..src_backup.len() - self.src().len()];

                return T::try_from_parsed_integer(ParsedInteger::U8(byte), bytes_ron);
            }
            _ => false,
        };
        let sign = if is_negative { -1 } else { 1 };

        let num_bytes = self.next_chars_while_len(is_int_char);

        if self.src()[num_bytes..].starts_with(['i', 'u']) {
            let int_cursor = self.cursor;
            self.advance_bytes(num_bytes);

            #[allow(clippy::never_loop)]
            loop {
                let (res, suffix_bytes) = if self.consume_ident("i8") {
                    let suffix_bytes = self.src();
                    self.set_cursor(int_cursor);
                    (
                        self.parse_integer::<i8>(sign).map(ParsedInteger::I8),
                        suffix_bytes,
                    )
                } else if self.consume_ident("i16") {
                    let suffix_bytes = self.src();
                    self.set_cursor(int_cursor);
                    (
                        self.parse_integer::<i16>(sign).map(ParsedInteger::I16),
                        suffix_bytes,
                    )
                } else if self.consume_ident("i32") {
                    let suffix_bytes = self.src();
                    self.set_cursor(int_cursor);
                    (
                        self.parse_integer::<i32>(sign).map(ParsedInteger::I32),
                        suffix_bytes,
                    )
                } else if self.consume_ident("i64") {
                    let suffix_bytes = self.src();
                    self.set_cursor(int_cursor);
                    (
                        self.parse_integer::<i64>(sign).map(ParsedInteger::I64),
                        suffix_bytes,
                    )
                } else if self.consume_ident("u8") {
                    let suffix_bytes = self.src();
                    self.set_cursor(int_cursor);
                    (
                        self.parse_integer::<u8>(sign).map(ParsedInteger::U8),
                        suffix_bytes,
                    )
                } else if self.consume_ident("u16") {
                    let suffix_bytes = self.src();
                    self.set_cursor(int_cursor);
                    (
                        self.parse_integer::<u16>(sign).map(ParsedInteger::U16),
                        suffix_bytes,
                    )
                } else if self.consume_ident("u32") {
                    let suffix_bytes = self.src();
                    self.set_cursor(int_cursor);
                    (
                        self.parse_integer::<u32>(sign).map(ParsedInteger::U32),
                        suffix_bytes,
                    )
                } else if self.consume_ident("u64") {
                    let suffix_bytes = self.src();
                    self.set_cursor(int_cursor);
                    (
                        self.parse_integer::<u64>(sign).map(ParsedInteger::U64),
                        suffix_bytes,
                    )
                } else {
                    #[cfg(feature = "integer128")]
                    if self.consume_ident("i128") {
                        let suffix_bytes = self.src();
                        self.set_cursor(int_cursor);
                        (
                            self.parse_integer::<i128>(sign).map(ParsedInteger::I128),
                            suffix_bytes,
                        )
                    } else if self.consume_ident("u128") {
                        let suffix_bytes = self.src();
                        self.set_cursor(int_cursor);
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
                    self.skip_identifier();
                }

                let integer_ron = &src_backup[..src_backup.len() - suffix_bytes.len()];

                return res.and_then(|parsed| T::try_from_parsed_integer(parsed, integer_ron));
            }

            self.set_cursor(int_cursor);
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

        let backup_cursor = self.cursor;

        let (integer_err, integer_cursor) = match self.integer::<ParsedInteger>() {
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
            Err(err) => (err, self.cursor),
        };

        self.set_cursor(backup_cursor);

        // Fall-back to parse an out-of-range integer as a float
        match self.float::<ParsedFloat>() {
            Ok(ParsedFloat::F32(v)) if self.cursor >= integer_cursor => Ok(Number::F32(v.into())),
            Ok(ParsedFloat::F64(v)) if self.cursor >= integer_cursor => Ok(Number::F64(v.into())),
            _ => {
                // Return the more precise integer error
                self.set_cursor(integer_cursor);
                Err(integer_err)
            }
        }
    }

    pub fn bool(&mut self) -> Result<bool> {
        if self.consume_ident("true") {
            Ok(true)
        } else if self.consume_ident("false") {
            Ok(false)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    pub fn char(&mut self) -> Result<char> {
        self.expect_char('\'', Error::ExpectedChar)?;

        let c = self.next_char()?;

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
        self.check_str(ident) && !self.check_ident_other_char(ident.len())
    }

    fn check_ident_other_char(&self, index: usize) -> bool {
        self.src()[index..]
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

            // Check for `Ident()`, which could be
            // - a zero-field struct or tuple (variant)
            // - an unwrapped newtype around a unit
            if matches!(newtype, NewtypeMode::NoParensMeanUnit) && parser.check_char(')') {
                return Ok(StructType::EmptyTuple);
            }

            if parser.skip_identifier().is_some() {
                parser.skip_ws()?;

                match parser.peek_char() {
                    // Definitely a struct with named fields
                    Some(':') => return Ok(StructType::Named),
                    // Definitely a tuple-like struct with fields
                    Some(',') => {
                        parser.skip_next_char();
                        parser.skip_ws()?;
                        if parser.check_char(')') {
                            // A one-element tuple could be a newtype
                            return Ok(StructType::NewtypeTuple);
                        }
                        // Definitely a tuple struct with more than one field
                        return Ok(StructType::NonNewtypeTuple);
                    }
                    // Either a newtype or a tuple struct
                    Some(')') => return Ok(StructType::NewtypeTuple),
                    // Something else, let's investigate further
                    Some(_) | None => (),
                };
            }

            if matches!(tuple, TupleMode::ImpreciseTupleOrNewtype) {
                return Ok(StructType::AnyTuple);
            }

            let mut braces = 1_usize;
            let mut more_than_one = false;

            // Skip ahead to see if the value is followed by another value
            while braces > 0 {
                // Skip spurious braces in comments, strings, and characters
                parser.skip_ws()?;
                let cursor_backup = parser.cursor;
                if parser.char().is_err() {
                    parser.set_cursor(cursor_backup);
                }
                let cursor_backup = parser.cursor;
                match parser.string() {
                    Ok(_) => (),
                    // prevent quadratic complexity backtracking for unterminated string
                    Err(err @ (Error::ExpectedStringEnd | Error::Eof)) => return Err(err),
                    Err(_) => parser.set_cursor(cursor_backup),
                }
                let cursor_backup = parser.cursor;
                // we have already checked for strings, which subsume base64 byte strings
                match parser.byte_string_no_base64() {
                    Ok(_) => (),
                    // prevent quadratic complexity backtracking for unterminated byte string
                    Err(err @ (Error::ExpectedStringEnd | Error::Eof)) => return Err(err),
                    Err(_) => parser.set_cursor(cursor_backup),
                }

                let c = parser.next_char()?;
                if matches!(c, '(' | '[' | '{') {
                    braces += 1;
                } else if matches!(c, ')' | ']' | '}') {
                    braces -= 1;
                } else if c == ',' && braces == 1 {
                    parser.skip_ws()?;
                    more_than_one = !parser.check_char(')');
                    break;
                }
            }

            if more_than_one {
                Ok(StructType::NonNewtypeTuple)
            } else {
                Ok(StructType::NewtypeTuple)
            }
        }

        // Create a temporary working copy
        let backup_cursor = self.cursor;

        let result = check_struct_type_inner(self, newtype, tuple);

        if result.is_ok() {
            // Revert the parser to before the struct type check
            self.set_cursor(backup_cursor);
        }

        result
    }

    /// Only returns true if the char after `ident` cannot belong
    /// to an identifier.
    pub fn consume_ident(&mut self, ident: &str) -> bool {
        if self.check_ident(ident) {
            self.advance_bytes(ident.len());

            true
        } else {
            false
        }
    }

    pub fn consume_struct_name(&mut self, ident: &'static str) -> Result<bool> {
        if self.check_ident("") {
            if self.exts.contains(Extensions::EXPLICIT_STRUCT_NAMES) {
                return Err(Error::ExpectedStructName(ident.to_string()));
            }

            return Ok(false);
        }

        let found_ident = match self.identifier() {
            Ok(maybe_ident) => maybe_ident,
            Err(Error::SuggestRawIdentifier(found_ident)) if found_ident == ident => {
                return Err(Error::SuggestRawIdentifier(found_ident))
            }
            Err(_) => return Err(Error::ExpectedNamedStructLike(ident)),
        };

        if ident.is_empty() {
            return Err(Error::ExpectedNamedStructLike(ident));
        }

        if found_ident != ident {
            return Err(Error::ExpectedDifferentStructName {
                expected: ident,
                found: String::from(found_ident),
            });
        }

        Ok(true)
    }

    /// Parse a document attribute at the current cursor position.
    fn attribute(&mut self) -> Result<ParsedAttribute> {
        if !self.check_char('#') {
            return Ok(ParsedAttribute::None);
        }

        if !self.consume_all(&["#", "!", "["])? {
            return Err(Error::ExpectedAttribute);
        }

        self.skip_ws()?;
        if self.consume_ident("enable") {
            self.skip_ws()?;
            if !self.consume_str("(") {
                return Err(Error::ExpectedAttribute);
            }

            self.skip_ws()?;
            let extensions = self.extension_list()?;
            self.skip_ws()?;

            if self.consume_all(&[")", "]"])? {
                Ok(ParsedAttribute::Extensions(extensions))
            } else {
                Err(Error::ExpectedAttributeEnd)
            }
        } else if self.consume_ident("type") || self.consume_ident("schema") {
            self.skip_ws()?;
            if !self.consume_str("=") {
                return Err(Error::ExpectedAttribute);
            }

            self.skip_ws()?;
            self.string()?;
            self.skip_ws()?;

            if self.consume_str("]") {
                Ok(ParsedAttribute::Ignored)
            } else {
                Err(Error::ExpectedAttributeEnd)
            }
        } else {
            Err(Error::ExpectedAttribute)
        }
    }

    /// Returns the extensions bit mask.
    fn extension_list(&mut self) -> Result<Extensions> {
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

        Ok(extensions)
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

            if let Some(suffix) = self.src().strip_prefix(literal) {
                if let Some(post_suffix) = suffix.strip_prefix(F32_SUFFIX) {
                    if !post_suffix.chars().next().map_or(false, is_xid_continue) {
                        let float_ron = &self.src()[..literal.len() + F32_SUFFIX.len()];
                        self.advance_bytes(literal.len() + F32_SUFFIX.len());
                        return T::try_from_parsed_float(ParsedFloat::F32(*value_f32), float_ron);
                    }
                }

                if let Some(post_suffix) = suffix.strip_prefix(F64_SUFFIX) {
                    if !post_suffix.chars().next().map_or(false, is_xid_continue) {
                        let float_ron = &self.src()[..literal.len() + F64_SUFFIX.len()];
                        self.advance_bytes(literal.len() + F64_SUFFIX.len());
                        return T::try_from_parsed_float(ParsedFloat::F64(*value_f64), float_ron);
                    }
                }
            }
        }

        let num_bytes = self.next_chars_while_len(is_float_char);

        if num_bytes == 0 {
            return Err(Error::ExpectedFloat);
        }

        if self.check_char('_') {
            return Err(Error::UnderscoreAtBeginning);
        }

        let mut f = String::with_capacity(num_bytes);
        let mut allow_underscore = false;

        for (i, c) in self.src()[..num_bytes].char_indices() {
            match c {
                '_' if allow_underscore => continue,
                '_' => {
                    self.advance_bytes(i);
                    return Err(Error::FloatUnderscore);
                }
                '0'..='9' | 'e' | 'E' => allow_underscore = true,
                '.' => allow_underscore = false,
                _ => (),
            }

            // we know that the byte is an ASCII character here
            f.push(c);
        }

        if self.src()[num_bytes..].starts_with('f') {
            let backup_cursor = self.cursor;
            self.advance_bytes(num_bytes);

            #[allow(clippy::never_loop)]
            loop {
                let res = if self.consume_ident(F32_SUFFIX) {
                    f32::from_str(&f).map(ParsedFloat::F32)
                } else if self.consume_ident(F64_SUFFIX) {
                    f64::from_str(&f).map(ParsedFloat::F64)
                } else {
                    break;
                };

                let parsed = if let Ok(parsed) = res {
                    parsed
                } else {
                    self.set_cursor(backup_cursor);
                    return Err(Error::ExpectedFloat);
                };

                let float_ron = &self.src[backup_cursor.cursor..self.cursor.cursor];

                return T::try_from_parsed_float(parsed, float_ron);
            }

            self.set_cursor(backup_cursor);
        }

        let value = T::parse(&f)?;

        self.advance_bytes(num_bytes);

        Ok(value)
    }

    pub fn skip_identifier(&mut self) -> Option<&'a str> {
        #[allow(clippy::nonminimal_bool)]
        if self.check_str("b\"") // byte string
            || self.check_str("b'") // byte literal
            || self.check_str("br#") // raw byte string
            || self.check_str("br\"") // raw byte string
            || self.check_str("r\"") // raw string
            || self.check_str("r#\"") // raw string
            || self.check_str("r##") // raw string
            || false
        {
            return None;
        }

        if self.check_str("r#") {
            // maybe a raw identifier
            let len = self.next_chars_while_from_len(2, is_ident_raw_char);
            if len > 0 {
                let ident = &self.src()[2..2 + len];
                self.advance_bytes(2 + len);
                return Some(ident);
            }
            return None;
        }

        if let Some(c) = self.peek_char() {
            // maybe a normal identifier
            if is_ident_first_char(c) {
                let len =
                    c.len_utf8() + self.next_chars_while_from_len(c.len_utf8(), is_xid_continue);
                let ident = &self.src()[..len];
                self.advance_bytes(len);
                return Some(ident);
            }
        }

        None
    }

    pub fn identifier(&mut self) -> Result<&'a str> {
        let first = self.peek_char_or_eof()?;
        if !is_ident_first_char(first) {
            if is_ident_raw_char(first) {
                let ident_bytes = self.next_chars_while_len(is_ident_raw_char);
                return Err(Error::SuggestRawIdentifier(
                    self.src()[..ident_bytes].into(),
                ));
            }

            return Err(Error::ExpectedIdentifier);
        }

        // If the next 2-3 bytes signify the start of a (raw) (byte) string
        //  literal, return an error.
        #[allow(clippy::nonminimal_bool)]
        if self.check_str("b\"") // byte string
            || self.check_str("b'") // byte literal
            || self.check_str("br#") // raw byte string
            || self.check_str("br\"") // raw byte string
            || self.check_str("r\"") // raw string
            || self.check_str("r#\"") // raw string
            || self.check_str("r##") // raw string
            || false
        {
            return Err(Error::ExpectedIdentifier);
        }

        let length = if self.check_str("r#") {
            let cursor_backup = self.cursor;

            self.advance_bytes(2);

            // Note: it's important to check this before advancing forward, so that
            // the value-type deserializer can fall back to parsing it differently.
            if !matches!(self.peek_char(), Some(c) if is_ident_raw_char(c)) {
                self.set_cursor(cursor_backup);
                return Err(Error::ExpectedIdentifier);
            }

            self.next_chars_while_len(is_ident_raw_char)
        } else if first == 'r' {
            let std_ident_length = self.next_chars_while_len(is_xid_continue);
            let raw_ident_length = self.next_chars_while_len(is_ident_raw_char);

            if raw_ident_length > std_ident_length {
                return Err(Error::SuggestRawIdentifier(
                    self.src()[..raw_ident_length].into(),
                ));
            }

            std_ident_length
        } else {
            let std_ident_length = first.len_utf8()
                + self.next_chars_while_from_len(first.len_utf8(), is_xid_continue);
            let raw_ident_length = self.next_chars_while_len(is_ident_raw_char);

            if raw_ident_length > std_ident_length {
                return Err(Error::SuggestRawIdentifier(
                    self.src()[..raw_ident_length].into(),
                ));
            }

            std_ident_length
        };

        let ident = &self.src()[..length];
        self.advance_bytes(length);

        Ok(ident)
    }

    pub fn next_bytes_is_float(&mut self) -> bool {
        if let Some(c) = self.peek_char() {
            let skip = match c {
                '+' | '-' => 1,
                _ => 0,
            };
            let valid_float_len = self.next_chars_while_from_len(skip, is_float_char);
            let valid_int_len = self.next_chars_while_from_len(skip, is_int_char);
            valid_float_len > valid_int_len
        } else {
            false
        }
    }

    pub fn skip_ws(&mut self) -> Result<()> {
        if (self.cursor.last_ws_len != WS_CURSOR_UNCLOSED_LINE)
            && ((self.cursor.pre_ws_cursor + self.cursor.last_ws_len) < self.cursor.cursor)
        {
            // the last whitespace is disjoint from this one, we need to track a new one
            self.cursor.pre_ws_cursor = self.cursor.cursor;
        }

        if self.src().is_empty() {
            return Ok(());
        }

        loop {
            self.advance_bytes(self.next_chars_while_len(is_whitespace_char));

            match self.skip_comment()? {
                None => break,
                Some(Comment::UnclosedLine) => {
                    self.cursor.last_ws_len = WS_CURSOR_UNCLOSED_LINE;
                    return Ok(());
                }
                Some(Comment::ClosedLine | Comment::Block) => continue,
            }
        }

        self.cursor.last_ws_len = self.cursor.cursor - self.cursor.pre_ws_cursor;

        Ok(())
    }

    pub fn has_unclosed_line_comment(&self) -> bool {
        self.src().is_empty() && self.cursor.last_ws_len == WS_CURSOR_UNCLOSED_LINE
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
            .flat_map(|c| core::ascii::escape_default(*c))
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

        // FIXME @juntyr: remove in v0.13, since only byte_string_no_base64 will
        //                be used
        if self.consume_char('"') {
            let base64_str = self.escaped_string()?;
            let base64_result = ParsedByteStr::try_from_base64(&base64_str);

            match base64_result {
                Some(byte_str) => Err(expected_byte_string_found_base64(&base64_str, &byte_str)),
                None => Err(Error::ExpectedByteString),
            }
        } else if self.consume_char('r') {
            let base64_str = self.raw_string()?;
            let base64_result = ParsedByteStr::try_from_base64(&base64_str);

            match base64_result {
                Some(byte_str) => Err(expected_byte_string_found_base64(&base64_str, &byte_str)),
                None => Err(Error::ExpectedByteString),
            }
        } else {
            self.byte_string_no_base64()
        }
    }

    pub fn byte_string_no_base64(&mut self) -> Result<ParsedByteStr<'a>> {
        if self.consume_str("b\"") {
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
                self.advance_bytes(advance);
                Ok(bytes)
            }
            Err(err) => Err(err),
        }
    }

    fn raw_byte_string(&mut self) -> Result<ParsedByteStr<'a>> {
        match self.raw_byte_buf() {
            Ok((bytes, advance)) => {
                self.advance_bytes(advance);
                Ok(bytes)
            }
            Err(Error::ExpectedString) => Err(Error::ExpectedByteString),
            Err(err) => Err(err),
        }
    }

    pub fn string(&mut self) -> Result<ParsedStr<'a>> {
        if self.consume_char('"') {
            self.escaped_string()
        } else if self.consume_char('r') {
            self.raw_string()
        } else {
            Err(Error::ExpectedString)
        }
    }

    fn escaped_string(&mut self) -> Result<ParsedStr<'a>> {
        match self.escaped_byte_buf(EscapeEncoding::Utf8) {
            Ok((bytes, advance)) => {
                let string = ParsedStr::try_from_bytes(bytes).map_err(Error::from)?;
                self.advance_bytes(advance);
                Ok(string)
            }
            Err(err) => Err(err),
        }
    }

    fn raw_string(&mut self) -> Result<ParsedStr<'a>> {
        match self.raw_byte_buf() {
            Ok((bytes, advance)) => {
                let string = ParsedStr::try_from_bytes(bytes).map_err(Error::from)?;
                self.advance_bytes(advance);
                Ok(string)
            }
            Err(err) => Err(err),
        }
    }

    fn escaped_byte_buf(&mut self, encoding: EscapeEncoding) -> Result<(ParsedByteStr<'a>, usize)> {
        // Checking for '"' and '\\' separately is faster than searching for both at the same time
        let str_end = self.src().find('"').ok_or(Error::ExpectedStringEnd)?;
        let escape = self.src()[..str_end].find('\\');

        if let Some(escape) = escape {
            // Now check if escaping is used inside the string
            let mut i = escape;
            let mut s = self.src().as_bytes()[..i].to_vec();

            loop {
                self.advance_bytes(i + 1);

                match self.parse_escape(encoding, false)? {
                    EscapeCharacter::Ascii(c) => s.push(c),
                    EscapeCharacter::Utf8(c) => match c.len_utf8() {
                        1 => s.push(c as u8),
                        len => {
                            let start = s.len();
                            s.extend(core::iter::repeat(0).take(len));
                            c.encode_utf8(&mut s[start..]);
                        }
                    },
                }

                // Checking for '"' and '\\' separately is faster than searching for both at the same time
                let new_str_end = self.src().find('"').ok_or(Error::ExpectedStringEnd)?;
                let new_escape = self.src()[..new_str_end].find('\\');

                if let Some(new_escape) = new_escape {
                    s.extend_from_slice(&self.src().as_bytes()[..new_escape]);
                    i = new_escape;
                } else {
                    s.extend_from_slice(&self.src().as_bytes()[..new_str_end]);
                    // Advance to the end of the string + 1 for the `"`.
                    break Ok((ParsedByteStr::Allocated(s), new_str_end + 1));
                }
            }
        } else {
            let s = &self.src().as_bytes()[..str_end];

            // Advance by the number of bytes of the string + 1 for the `"`.
            Ok((ParsedByteStr::Slice(s), str_end + 1))
        }
    }

    fn raw_byte_buf(&mut self) -> Result<(ParsedByteStr<'a>, usize)> {
        let num_hashes = self.next_chars_while_len(|c| c == '#');
        let hashes = &self.src()[..num_hashes];
        self.advance_bytes(num_hashes);

        self.expect_char('"', Error::ExpectedString)?;

        let ending = ["\"", hashes].concat();
        let i = self.src().find(&ending).ok_or(Error::ExpectedStringEnd)?;

        let s = &self.src().as_bytes()[..i];

        // Advance by the number of bytes of the byte string
        // + `num_hashes` + 1 for the `"`.
        Ok((ParsedByteStr::Slice(s), i + num_hashes + 1))
    }

    fn decode_ascii_escape(&mut self) -> Result<u8> {
        let mut n = 0;
        for _ in 0..2 {
            n <<= 4;
            let byte = self.next_char()?;
            let decoded = Self::decode_hex(byte)?;
            n |= decoded;
        }

        Ok(n)
    }

    #[inline]
    fn decode_hex(c: char) -> Result<u8> {
        if !c.is_ascii() {
            return Err(Error::InvalidEscape("Non-hex digit found"));
        }

        // c is an ASCII character that can be losslessly cast to u8
        match c as u8 {
            c @ b'0'..=b'9' => Ok(c - b'0'),
            c @ b'a'..=b'f' => Ok(10 + c - b'a'),
            c @ b'A'..=b'F' => Ok(10 + c - b'A'),
            _ => Err(Error::InvalidEscape("Non-hex digit found")),
        }
    }

    fn parse_escape(&mut self, encoding: EscapeEncoding, is_char: bool) -> Result<EscapeCharacter> {
        let c = match self.next_char()? {
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
                    }

                    self.skip_next_char();
                    num_digits += 1;

                    let byte = Self::decode_hex(byte)?;
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
            match self.next_char()? {
                '/' => {
                    let bytes = self.next_chars_while_len(|c| c != '\n');

                    self.advance_bytes(bytes);

                    if self.src().is_empty() {
                        Ok(Some(Comment::UnclosedLine))
                    } else {
                        Ok(Some(Comment::ClosedLine))
                    }
                }
                '*' => {
                    let mut level = 1;

                    while level > 0 {
                        let bytes = self.next_chars_while_len(|c| !matches!(c, '/' | '*'));

                        if self.src().is_empty() {
                            return Err(Error::UnclosedBlockComment);
                        }

                        self.advance_bytes(bytes);

                        // check whether / or * and take action
                        if self.consume_str("/*") {
                            level += 1;
                        } else if self.consume_str("*/") {
                            level -= 1;
                        } else {
                            self.next_char().map_err(|_| Error::UnclosedBlockComment)?;
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

        #[allow(clippy::cast_possible_truncation)]
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
    AnyTuple,
    EmptyTuple,
    NewtypeTuple,
    NonNewtypeTuple,
    Named,
    Unit,
}

#[derive(Copy, Clone)] // GRCOV_EXCL_LINE
pub enum NewtypeMode {
    NoParensMeanUnit,
    InsideNewtype,
}

#[derive(Copy, Clone)] // GRCOV_EXCL_LINE
pub enum TupleMode {
    ImpreciseTupleOrNewtype,
    DifferentiateNewtype,
}

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
    pub fn try_from_base64(str: &ParsedStr<'a>) -> Option<Self> {
        // Adapted from MIT licensed Jenin Sutradhar's base 64 decoder
        // https://github.com/JeninSutradhar/base64-Rust-Encoder-Decoder/blob/ee1fb08cbb78024ec8cf5e786815acb239169f02/src/lib.rs#L84-L128
        fn try_decode_base64(str: &str) -> Option<Vec<u8>> {
            const CHARSET: &[u8; 64] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            const PADDING: u8 = b'=';

            // fast reject for missing padding
            if (str.len() % 4) != 0 {
                return None;
            }

            let bstr_no_padding = str.trim_end_matches(char::from(PADDING)).as_bytes();

            // fast reject for excessive padding
            if (str.len() - bstr_no_padding.len()) > 2 {
                return None;
            }

            // fast reject for extraneous bytes after padding
            if bstr_no_padding.contains(&PADDING) {
                return None;
            }

            // fast reject for non-ASCII
            if !str.is_ascii() {
                return None;
            }

            let mut collected_bits = 0_u8;
            let mut byte_buffer = 0_u16;
            let mut bytes = bstr_no_padding.iter().copied();
            let mut binary = Vec::new();

            'decodeloop: loop {
                while collected_bits < 8 {
                    if let Some(nextbyte) = bytes.next() {
                        #[allow(clippy::cast_possible_truncation)]
                        if let Some(idx) = CHARSET.iter().position(|&x| x == nextbyte) {
                            byte_buffer |= ((idx & 0b0011_1111) as u16) << (10 - collected_bits);
                            collected_bits += 6;
                        } else {
                            return None;
                        }
                    } else {
                        break 'decodeloop;
                    }
                }

                binary.push(((0b1111_1111_0000_0000 & byte_buffer) >> 8) as u8);
                byte_buffer &= 0b0000_0000_1111_1111;
                byte_buffer <<= 8;
                collected_bits -= 8;
            }

            if usize::from(collected_bits) != ((str.len() - bstr_no_padding.len()) * 2) {
                return None;
            }

            Some(binary)
        }

        let base64_str = match str {
            ParsedStr::Allocated(string) => string.as_str(),
            ParsedStr::Slice(str) => str,
        };

        try_decode_base64(base64_str).map(ParsedByteStr::Allocated)
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

        assert_eq!(bytes.src(), "42       /*bye*/ 24  ");
        assert_eq!(bytes.pre_ws_src(), SOURCE);

        bytes.skip_ws().unwrap();

        assert_eq!(bytes.src(), "42       /*bye*/ 24  ");
        assert_eq!(bytes.pre_ws_src(), SOURCE);

        assert_eq!(bytes.integer::<u8>().unwrap(), 42);

        assert_eq!(bytes.src(), "       /*bye*/ 24  ");
        assert_eq!(bytes.pre_ws_src(), SOURCE);

        bytes.skip_ws().unwrap();
        bytes.skip_ws().unwrap();

        assert_eq!(bytes.src(), "24  ");
        assert_eq!(bytes.pre_ws_src(), "       /*bye*/ 24  ");

        let mut bytes = Parser::new("42").unwrap();
        bytes.skip_ws().unwrap();
        bytes.skip_ws().unwrap();
        assert_eq!(bytes.src(), "42");
        assert_eq!(bytes.pre_ws_src(), "42");
        assert_eq!(bytes.integer::<u8>().unwrap(), 42);
        bytes.skip_ws().unwrap();
        bytes.skip_ws().unwrap();
        assert_eq!(bytes.src(), "");
        assert_eq!(bytes.pre_ws_src(), "");

        let mut bytes = Parser::new("  42  ").unwrap();
        bytes.skip_ws().unwrap();
        bytes.skip_ws().unwrap();
        assert_eq!(bytes.src(), "42  ");
        assert_eq!(bytes.pre_ws_src(), "  42  ");
        assert_eq!(bytes.integer::<u8>().unwrap(), 42);
        bytes.skip_ws().unwrap();
        bytes.skip_ws().unwrap();
        assert_eq!(bytes.src(), "");
        assert_eq!(bytes.pre_ws_src(), "  ");

        let mut bytes = Parser::new("  42  //").unwrap();
        bytes.skip_ws().unwrap();
        bytes.skip_ws().unwrap();
        assert_eq!(bytes.src(), "42  //");
        assert_eq!(bytes.pre_ws_src(), "  42  //");
        assert_eq!(bytes.integer::<u8>().unwrap(), 42);
        bytes.skip_ws().unwrap();
        bytes.skip_ws().unwrap();
        assert_eq!(bytes.src(), "");
        assert_eq!(bytes.pre_ws_src(), "  //");
    }

    #[test]
    fn parser_cursor_eq_cmp() {
        assert!(
            ParserCursor {
                cursor: 42,
                pre_ws_cursor: 42,
                last_ws_len: 42
            } == ParserCursor {
                cursor: 42,
                pre_ws_cursor: 24,
                last_ws_len: 24
            }
        );
        assert!(
            ParserCursor {
                cursor: 42,
                pre_ws_cursor: 42,
                last_ws_len: 42
            } != ParserCursor {
                cursor: 24,
                pre_ws_cursor: 42,
                last_ws_len: 42
            }
        );

        assert!(
            ParserCursor {
                cursor: 42,
                pre_ws_cursor: 42,
                last_ws_len: 42
            } < ParserCursor {
                cursor: 43,
                pre_ws_cursor: 24,
                last_ws_len: 24
            }
        );
        assert!(
            ParserCursor {
                cursor: 42,
                pre_ws_cursor: 42,
                last_ws_len: 42
            } > ParserCursor {
                cursor: 41,
                pre_ws_cursor: 24,
                last_ws_len: 24
            }
        );
    }

    #[test]
    fn empty_src_is_not_a_float() {
        assert!(!Parser::new("").unwrap().next_bytes_is_float());
    }

    #[test]
    fn base64_deprecation_error() {
        let err = crate::from_str::<bytes::Bytes>("\"SGVsbG8gcm9uIQ==\"").unwrap_err();

        assert_eq!(
            err,
            SpannedError {
                code: Error::InvalidValueForType {
                    expected: String::from("the Rusty byte string b\"Hello ron!\""),
                    found: String::from("the ambiguous base64 string \"SGVsbG8gcm9uIQ==\"")
                },
                span: Span {
                    start: Position { line: 1, col: 2 },
                    end: Position { line: 1, col: 19 },
                }
            }
        );

        let err = crate::from_str::<bytes::Bytes>("r\"SGVsbG8gcm9uIQ==\"").unwrap_err();

        assert_eq!(format!("{}", err.code), "Expected the Rusty byte string b\"Hello ron!\" but found the ambiguous base64 string \"SGVsbG8gcm9uIQ==\" instead");

        assert_eq!(
            crate::from_str::<bytes::Bytes>("\"invalid=\"").unwrap_err(),
            SpannedError {
                code: Error::InvalidValueForType {
                    expected: String::from("the Rusty byte string b\"\\x8a{\\xda\\x96\\'\""),
                    found: String::from("the ambiguous base64 string \"invalid=\"")
                },
                span: Span {
                    start: Position { line: 1, col: 2 },
                    end: Position { line: 1, col: 11 },
                }
            }
        );

        assert_eq!(
            crate::from_str::<bytes::Bytes>("r\"invalid=\"").unwrap_err(),
            SpannedError {
                code: Error::InvalidValueForType {
                    expected: String::from("the Rusty byte string b\"\\x8a{\\xda\\x96\\'\""),
                    found: String::from("the ambiguous base64 string \"invalid=\"")
                },
                span: Span {
                    start: Position { line: 1, col: 3 },
                    end: Position { line: 1, col: 12 },
                }
            }
        );

        assert_eq!(
            crate::from_str::<bytes::Bytes>("r\"invalid\"").unwrap_err(),
            SpannedError {
                code: Error::ExpectedByteString,
                span: Span {
                    start: Position { line: 1, col: 3 },
                    end: Position { line: 1, col: 11 },
                }
            }
        );
    }
}
