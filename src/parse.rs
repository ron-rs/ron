#![allow(clippy::identity_op)]

use std::str::{self, FromStr};

use base64::engine::general_purpose::{GeneralPurpose, STANDARD};
use unicode_ident::{is_xid_continue, is_xid_start};

use crate::{
    error::{Error, Position, Result, SpannedError, SpannedResult},
    extensions::Extensions,
};

pub const BASE64_ENGINE: GeneralPurpose = STANDARD;

const fn is_int_char(c: u8) -> bool {
    c.is_ascii_hexdigit() || c == b'_'
}

const fn is_float_char(c: u8) -> bool {
    c.is_ascii_digit() || matches!(c, b'e' | b'E' | b'.' | b'+' | b'-' | b'_')
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

#[derive(Clone, Debug, PartialEq)]
pub enum AnyNum {
    F32(f32),
    F64(f64),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    #[cfg(feature = "integer128")]
    I128(i128),
    #[cfg(feature = "integer128")]
    U128(u128),
}

#[derive(Clone, Copy, Debug)]
pub struct Parser<'a> {
    /// Bits set according to the [`Extensions`] enum.
    pub exts: Extensions,
    src: &'a str,
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

    pub fn advance(&mut self, mut bytes: usize) -> Result<()> {
        while bytes > 0 {
            bytes -= self.advance_char()?;
        }

        Ok(())
    }

    pub fn advance_char(&mut self) -> Result<usize> {
        self.next().map(char::len_utf8)
    }

    pub fn next(&mut self) -> Result<char> {
        let c = match self.peek() {
            Ok(it) => it,
            Err(err) => return Err(err),
        };
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
        if let Some(c) = self.src.chars().next() {
            Ok(c)
        } else {
            Err(Error::Eof)
        }
    }

    pub fn src(&self) -> &'a str {
        self.src
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

    #[must_use]
    pub fn find_index(&self, condition: fn(char) -> bool) -> Option<(usize, char)> {
        self.src.char_indices().find(|&(_, c)| condition(c))
    }
}

/// actual parsing
impl<'a> Parser<'a> {
    fn any_integer<T: Num>(&mut self, sign: i8) -> Result<T> {
        let base = if self.peek() == Ok('0') {
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

        fn calc_num<T: Num>(s: &str, base: u8, f: fn(&mut T, u8) -> bool) -> Result<T> {
            let mut num_acc = T::from_u8(0);

            for &byte in s.as_bytes() {
                if byte == b'_' {
                    continue;
                }

                if num_acc.checked_mul_ext(base) {
                    return Err(Error::IntegerOutOfBounds);
                }

                let digit = if byte.is_ascii_digit() {
                    byte - b'0'
                } else {
                    debug_assert!(byte.is_ascii_alphabetic());
                    byte.to_ascii_lowercase() - b'a' + 10
                };

                if digit >= base {
                    return Err(Error::ExpectedInteger);
                }

                if f(&mut num_acc, digit) {
                    return Err(Error::IntegerOutOfBounds);
                }
            }

            Ok(num_acc)
        }

        let res = if sign > 0 {
            calc_num(s, base, T::checked_add_ext)
        } else {
            calc_num(s, base, T::checked_sub_ext)
        };

        let _ = self.advance(num_bytes);

        res
    }

    pub fn any_num(&mut self) -> Result<AnyNum> {
        // We are not doing float comparisons here in the traditional sense.
        // Instead, this code checks if a f64 fits inside an f32.
        #[allow(clippy::float_cmp)]
        fn any_float(f: f64) -> Result<AnyNum> {
            if f == f64::from(f as f32) {
                Ok(AnyNum::F32(f as f32))
            } else {
                Ok(AnyNum::F64(f))
            }
        }

        let source_backup = self.src;

        let first = self.peek()?;
        let is_signed = matches!(first, '-' | '+');
        let is_float = self.next_bytes_is_float();

        if is_float {
            let f = self.float::<f64>()?;

            any_float(f)
        } else {
            let max_u8 = LargeUInt::from(std::u8::MAX);
            let max_u16 = LargeUInt::from(std::u16::MAX);
            let max_u32 = LargeUInt::from(std::u32::MAX);
            #[cfg_attr(not(feature = "integer128"), allow(clippy::useless_conversion))]
            let max_u64 = LargeUInt::from(std::u64::MAX);

            let min_i8 = LargeSInt::from(std::i8::MIN);
            let max_i8 = LargeSInt::from(std::i8::MAX);
            let min_i16 = LargeSInt::from(std::i16::MIN);
            let max_i16 = LargeSInt::from(std::i16::MAX);
            let min_i32 = LargeSInt::from(std::i32::MIN);
            let max_i32 = LargeSInt::from(std::i32::MAX);
            #[cfg_attr(not(feature = "integer128"), allow(clippy::useless_conversion))]
            let min_i64 = LargeSInt::from(std::i64::MIN);
            #[cfg_attr(not(feature = "integer128"), allow(clippy::useless_conversion))]
            let max_i64 = LargeSInt::from(std::i64::MAX);

            if is_signed {
                // TODO should a `+ u128::MAX` be allowed?
                match self.signed_integer::<LargeSInt>() {
                    Ok(x) => {
                        if x >= min_i8 && x <= max_i8 {
                            Ok(AnyNum::I8(x as i8))
                        } else if x >= min_i16 && x <= max_i16 {
                            Ok(AnyNum::I16(x as i16))
                        } else if x >= min_i32 && x <= max_i32 {
                            Ok(AnyNum::I32(x as i32))
                        } else if x >= min_i64 && x <= max_i64 {
                            Ok(AnyNum::I64(x as i64))
                        } else {
                            #[cfg(feature = "integer128")]
                            {
                                Ok(AnyNum::I128(x))
                            }
                            #[cfg(not(feature = "integer128"))]
                            {
                                Ok(AnyNum::I64(x))
                            }
                        }
                    }
                    Err(_) => {
                        self.src = source_backup;

                        any_float(self.float::<f64>()?)
                    }
                }
            } else {
                match self.unsigned_integer::<LargeUInt>() {
                    Ok(x) => {
                        if x <= max_u8 {
                            Ok(AnyNum::U8(x as u8))
                        } else if x <= max_u16 {
                            Ok(AnyNum::U16(x as u16))
                        } else if x <= max_u32 {
                            Ok(AnyNum::U32(x as u32))
                        } else if x <= max_u64 {
                            Ok(AnyNum::U64(x as u64))
                        } else {
                            #[cfg(feature = "integer128")]
                            {
                                Ok(AnyNum::U128(x))
                            }
                            #[cfg(not(feature = "integer128"))]
                            {
                                Ok(AnyNum::U64(x))
                            }
                        }
                    }
                    Err(_) => {
                        self.src = source_backup;

                        any_float(self.float::<f64>()?)
                    }
                }
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

        let c = if c == '\\' { self.parse_escape()? } else { c };

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

    /// Should only be used on a working copy
    pub fn check_tuple_struct(mut self) -> Result<bool> {
        if self.identifier().is_err() {
            // if there's no field ident, this is a tuple struct
            return Ok(true);
        }

        self.skip_ws()?;

        // if there is no colon after the ident, this can only be a unit struct
        self.peek().map(|c| c != ':')
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

    pub fn float<T>(&mut self) -> Result<T>
    where
        T: FromStr,
    {
        for literal in &["inf", "+inf", "-inf", "NaN", "+NaN", "-NaN"] {
            if self.consume_ident(literal) {
                return FromStr::from_str(literal).map_err(|_| unreachable!()); // must not fail
            }
        }

        let num_bytes = self.next_bytes_while(is_float_char);

        // Since `rustc` allows `1_0.0_1`, lint against underscores in floats
        if let Some(err_bytes) = self.src[..num_bytes].find('_') {
            let _ = self.advance(err_bytes);

            return Err(Error::FloatUnderscore);
        }

        let res = FromStr::from_str(&self.src[..num_bytes]).map_err(|_| Error::ExpectedFloat);

        let _ = self.advance(num_bytes);

        res
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

        // If the next two bytes signify the start of a raw string literal,
        // return an error.
        let length = if first == 'r' {
            match self.src.chars().nth(1).ok_or(Error::Eof)? {
                '"' => return Err(Error::ExpectedIdentifier),
                '#' => {
                    let after_next = self.src[2..].chars().next().unwrap_or_default();
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
        loop {
            while self.peek().map_or(false, is_whitespace_char) {
                let _ = self.advance_char();
            }

            if !self.skip_comment()? {
                return Ok(());
            }
        }
    }

    pub fn signed_integer<T>(&mut self) -> Result<T>
    where
        T: Num,
    {
        match self.peek()? {
            '+' => {
                let _ = self.advance_char();

                self.any_integer(1)
            }
            '-' => {
                let _ = self.advance_char();

                self.any_integer(-1)
            }
            _ => self.any_integer(1),
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
        let (i, end_or_escape) = self
            .find_index(|c| matches!(c, '\\' | '"'))
            .ok_or(Error::ExpectedStringEnd)?;

        if end_or_escape == '"' {
            let s = &self.src[..i];

            // Advance by the number of bytes of the string
            // + 1 for the `"`.
            let _ = self.advance(i + 1);

            Ok(ParsedStr::Slice(s))
        } else {
            let mut i = i;
            let mut s = self.src[..i].to_owned();

            loop {
                let _ = self.advance(i + 1);
                let character = self.parse_escape()?;
                s.push(character);

                let (new_i, end_or_escape) = self
                    .find_index(|c| matches!(c, '\\' | '"'))
                    .ok_or(Error::ExpectedStringEnd)?;

                i = new_i;
                s.push_str(&self.src[..i]);

                if end_or_escape == '"' {
                    let _ = self.advance(i + 1);

                    break Ok(ParsedStr::Allocated(s));
                }
            }
        }
    }

    /// Parses after the `r`
    fn raw_string(&mut self) -> Result<ParsedStr<'a>> {
        let num_hashes = self.next_bytes_while(|b| b == b'#');
        let hashes = &self.src[..num_hashes];
        let _ = self.advance(num_hashes);

        self.expect_char('"', Error::ExpectedString)?;

        let ending = [&"\"", hashes].concat();
        let i = self.src.find(&ending).ok_or(Error::ExpectedStringEnd)?;

        // SAFETY: Self::new verifies bytes is valid utf8
        let s = &self.src[..i];

        // Advance by the number of bytes of the string
        // + `num_hashes` + 1 for the `"`.
        let _ = self.advance(i + num_hashes + 1);

        Ok(ParsedStr::Slice(s))
    }

    pub fn unsigned_integer<T: Num>(&mut self) -> Result<T> {
        self.any_integer(1)
    }

    fn decode_char(&mut self, len: usize) -> Result<char> {
        let src = self.src;
        self.advance(len)?;
        u32::from_str_radix(&src[..len], 16)
            .ok()
            .and_then(char::from_u32)
            .ok_or(Error::InvalidEscape("Invalid hex escape"))
    }

    fn parse_escape(&mut self) -> Result<char> {
        let c = match self.next()? {
            e @ ('\'' | '"' | '\\') => e,
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            'x' => self.decode_char(2)?,
            'u' => {
                self.expect_char('{', Error::InvalidEscape("Missing { in Unicode escape"))?;
                let num_digits = self.next_bytes_while_max(6, |b| b.is_ascii_hexdigit());

                if num_digits == 0 {
                    return Err(Error::InvalidEscape(
                        "Expected 1-6 digits, got 0 digits in Unicode escape",
                    ));
                }
                let c = self.decode_char(num_digits)?;
                self.expect_char(
                    '}',
                    Error::InvalidEscape("No } at the end of Unicode escape"),
                )?;
                c
            }
            _ => {
                return Err(Error::InvalidEscape("Unknown escape character"));
            }
        };

        Ok(c)
    }

    fn skip_comment(&mut self) -> Result<bool> {
        if self.consume_char('/') {
            match self.next()? {
                '/' => {
                    let bytes = self.next_bytes_while(|b| b != b'\n');

                    let _ = self.advance(bytes);
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
                }
                c => return Err(Error::UnexpectedChar(c)),
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
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
    ($ty:ident) => {
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
    ($($tys:ident)*) => {
        $( impl_num!($tys); )*
    };
}

#[cfg(feature = "integer128")]
impl_num!(u8 u16 u32 u64 u128 i8 i16 i32 i64 i128);
#[cfg(not(feature = "integer128"))]
impl_num!(u8 u16 u32 u64 i8 i16 i32 i64);

#[derive(Clone, Debug)]
pub enum ParsedStr<'a> {
    Allocated(String),
    Slice(&'a str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_x10() {
        let mut bytes = Parser::new("10").unwrap();
        assert_eq!(bytes.decode_char(2), Ok('\x10'));
    }
}
