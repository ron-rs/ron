#![allow(clippy::identity_op)]

use std::{
    char::from_u32 as char_from_u32,
    str::{from_utf8, FromStr},
};

use base64::engine::general_purpose::{GeneralPurpose, STANDARD};

use crate::{
    error::{Error, Position, Result, SpannedError, SpannedResult},
    extensions::Extensions,
    value::Number,
};

pub const BASE64_ENGINE: GeneralPurpose = STANDARD;

// We have the following char categories.
const INT_CHAR: u8 = 1 << 0; // [0-9A-Fa-f_]
const FLOAT_CHAR: u8 = 1 << 1; // [0-9\.Ee+-_]
const IDENT_FIRST_CHAR: u8 = 1 << 2; // [A-Za-z_]
const IDENT_OTHER_CHAR: u8 = 1 << 3; // [A-Za-z_0-9]
const IDENT_RAW_CHAR: u8 = 1 << 4; // [A-Za-z_0-9\.+-]
const WHITESPACE_CHAR: u8 = 1 << 5; // [\n\t\r ]

// We encode each char as belonging to some number of these categories.
const DIGIT: u8 = INT_CHAR | FLOAT_CHAR | IDENT_OTHER_CHAR | IDENT_RAW_CHAR; // [0-9]
const ABCDF: u8 = INT_CHAR | IDENT_FIRST_CHAR | IDENT_OTHER_CHAR | IDENT_RAW_CHAR; // [ABCDFabcdf]
const UNDER: u8 = INT_CHAR | FLOAT_CHAR | IDENT_FIRST_CHAR | IDENT_OTHER_CHAR | IDENT_RAW_CHAR; // [_]
const E____: u8 = INT_CHAR | FLOAT_CHAR | IDENT_FIRST_CHAR | IDENT_OTHER_CHAR | IDENT_RAW_CHAR; // [Ee]
const G2Z__: u8 = IDENT_FIRST_CHAR | IDENT_OTHER_CHAR | IDENT_RAW_CHAR; // [G-Zg-z]
const PUNCT: u8 = FLOAT_CHAR | IDENT_RAW_CHAR; // [\.+-]
const WS___: u8 = WHITESPACE_CHAR; // [\t\n\r ]
const _____: u8 = 0; // everything else

// Table of encodings, for fast predicates. (Non-ASCII and special chars are
// shown with '·' in the comment.)
#[rustfmt::skip]
const ENCODINGS: [u8; 256] = [
/*                     0      1      2      3      4      5      6      7      8      9    */
/*   0+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, WS___,
/*  10+: ·········· */ WS___, _____, _____, WS___, _____, _____, _____, _____, _____, _____,
/*  20+: ·········· */ _____, _____, _____, _____, _____, _____, _____, _____, _____, _____,
/*  30+: ·· !"#$%&' */ _____, _____, WS___, _____, _____, _____, _____, _____, _____, _____,
/*  40+: ()*+,-./01 */ _____, _____, _____, PUNCT, _____, PUNCT, PUNCT, _____, DIGIT, DIGIT,
/*  50+: 23456789:; */ DIGIT, DIGIT, DIGIT, DIGIT, DIGIT, DIGIT, DIGIT, DIGIT, _____, _____,
/*  60+: <=>?@ABCDE */ _____, _____, _____, _____, _____, ABCDF, ABCDF, ABCDF, ABCDF, E____,
/*  70+: FGHIJKLMNO */ ABCDF, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__,
/*  80+: PQRSTUVWZY */ G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__,
/*  90+: Z[\]^_`abc */ G2Z__, _____, _____, _____, _____, UNDER, _____, ABCDF, ABCDF, ABCDF,
/* 100+: defghijklm */ ABCDF, E____, ABCDF, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__,
/* 110+: nopqrstuvw */ G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__, G2Z__,
/* 120+: xyz{|}~··· */ G2Z__, G2Z__, G2Z__, _____, _____, _____, _____, _____, _____, _____,
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

pub const fn is_ident_first_char(c: u8) -> bool {
    ENCODINGS[c as usize] & IDENT_FIRST_CHAR != 0
}

pub const fn is_ident_other_char(c: u8) -> bool {
    ENCODINGS[c as usize] & IDENT_OTHER_CHAR != 0
}

pub const fn is_ident_raw_char(c: u8) -> bool {
    ENCODINGS[c as usize] & IDENT_RAW_CHAR != 0
}

const fn is_whitespace_char(c: u8) -> bool {
    ENCODINGS[c as usize] & WHITESPACE_CHAR != 0
}

#[derive(Clone, Copy, Debug)]
pub struct Bytes<'a> {
    /// Bits set according to the [`Extensions`] enum.
    pub exts: Extensions,
    bytes: &'a [u8],
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

impl<'a> Bytes<'a> {
    pub fn new(bytes: &'a [u8]) -> SpannedResult<Self> {
        let mut b = Bytes {
            exts: Extensions::empty(),
            bytes,
            cursor: Position { line: 1, col: 1 },
        };

        b.skip_ws().map_err(|e| b.span_error(e))?;

        // Loop over all extensions attributes
        loop {
            let attribute = b.extensions().map_err(|e| b.span_error(e))?;

            if attribute.is_empty() {
                break;
            }

            b.exts |= attribute;
            b.skip_ws().map_err(|e| b.span_error(e))?;
        }

        Ok(b)
    }

    pub fn span_error(&self, code: Error) -> SpannedError {
        SpannedError {
            code,
            position: self.cursor,
        }
    }

    pub fn advance(&mut self, bytes: usize) -> Result<()> {
        for _ in 0..bytes {
            self.advance_single()?;
        }

        Ok(())
    }

    pub fn advance_single(&mut self) -> Result<()> {
        if self.peek_or_eof()? == b'\n' {
            self.cursor.line += 1;
            self.cursor.col = 1;
        } else {
            self.cursor.col += 1;
        }

        self.bytes = &self.bytes[1..];

        Ok(())
    }

    fn parse_integer<T: Num>(&mut self, sign: i8) -> Result<T> {
        let base = if self.peek() == Some(b'0') {
            match self.bytes.get(1).copied() {
                Some(b'x') => 16,
                Some(b'b') => 2,
                Some(b'o') => 8,
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

        let num_bytes = self.next_bytes_contained_in(is_int_char);

        if num_bytes == 0 {
            return Err(Error::ExpectedInteger);
        }

        if self.bytes[0] == b'_' {
            return Err(Error::UnderscoreAtBeginning);
        }

        fn calc_num<T: Num>(
            bytes: &mut Bytes,
            b: &[u8],
            base: u8,
            mut f: impl FnMut(&mut T, u8) -> bool,
        ) -> Result<T> {
            let mut num_acc = T::from_u8(0);

            for (i, &byte) in b.iter().enumerate() {
                if byte == b'_' {
                    continue;
                }

                if num_acc.checked_mul_ext(base) {
                    let _ = bytes.advance(b.len());
                    return Err(Error::IntegerOutOfBounds);
                }

                let digit = bytes.decode_hex(byte)?;

                if digit >= base {
                    let _ = bytes.advance(i);
                    return Err(Error::InvalidIntegerDigit {
                        digit: char::from(byte),
                        base,
                    });
                }

                if f(&mut num_acc, digit) {
                    let _ = bytes.advance(b.len());
                    return Err(Error::IntegerOutOfBounds);
                }
            }

            let _ = bytes.advance(b.len());

            Ok(num_acc)
        }

        if sign > 0 {
            calc_num(self, &self.bytes[0..num_bytes], base, T::checked_add_ext)
        } else {
            calc_num(self, &self.bytes[0..num_bytes], base, T::checked_sub_ext)
        }
    }

    pub fn any_integer(&mut self, desire: DesireInteger) -> Result<ParsedInteger> {
        let is_negative = match self.peek_or_eof()? {
            b'+' => {
                let _ = self.advance_single();
                false
            }
            b'-' => {
                let _ = self.advance_single();
                true
            }
            _ => false,
        };
        let sign = if is_negative { -1 } else { 1 };

        let num_bytes = self.next_bytes_contained_in(is_int_char);

        if let Some(b'i' | b'u') = self.bytes.get(num_bytes) {
            let int_bytes_backup = *self;
            let _ = self.advance(num_bytes);

            #[allow(clippy::never_loop)]
            loop {
                let res = if self.consume_ident("i8") {
                    *self = int_bytes_backup;
                    self.parse_integer::<i8>(sign).map(ParsedInteger::I8)
                } else if self.consume_ident("i16") {
                    *self = int_bytes_backup;
                    self.parse_integer::<i16>(sign).map(ParsedInteger::I16)
                } else if self.consume_ident("i32") {
                    *self = int_bytes_backup;
                    self.parse_integer::<i32>(sign).map(ParsedInteger::I32)
                } else if self.consume_ident("i64") {
                    *self = int_bytes_backup;
                    self.parse_integer::<i64>(sign).map(ParsedInteger::I64)
                } else if self.consume_ident("u8") {
                    *self = int_bytes_backup;
                    self.parse_integer::<u8>(sign).map(ParsedInteger::U8)
                } else if self.consume_ident("u16") {
                    *self = int_bytes_backup;
                    self.parse_integer::<u16>(sign).map(ParsedInteger::U16)
                } else if self.consume_ident("u32") {
                    *self = int_bytes_backup;
                    self.parse_integer::<u32>(sign).map(ParsedInteger::U32)
                } else if self.consume_ident("u64") {
                    *self = int_bytes_backup;
                    self.parse_integer::<u64>(sign).map(ParsedInteger::U64)
                } else {
                    #[cfg(feature = "integer128")]
                    if self.consume_ident("i128") {
                        *self = int_bytes_backup;
                        self.parse_integer::<i128>(sign).map(ParsedInteger::I128)
                    } else if self.consume_ident("u128") {
                        *self = int_bytes_backup;
                        self.parse_integer::<u128>(sign).map(ParsedInteger::U128)
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

                return res;
            }

            *self = int_bytes_backup;
        }

        match desire {
            DesireInteger::Any => (),
            DesireInteger::I8 => return self.parse_integer::<i8>(sign).map(ParsedInteger::I8),
            DesireInteger::I16 => return self.parse_integer::<i16>(sign).map(ParsedInteger::I16),
            DesireInteger::I32 => return self.parse_integer::<i32>(sign).map(ParsedInteger::I32),
            DesireInteger::I64 => return self.parse_integer::<i64>(sign).map(ParsedInteger::I64),
            #[cfg(feature = "integer128")]
            DesireInteger::I128 => {
                return self.parse_integer::<i128>(sign).map(ParsedInteger::I128)
            }
            DesireInteger::U8 => return self.parse_integer::<u8>(sign).map(ParsedInteger::U8),
            DesireInteger::U16 => return self.parse_integer::<u16>(sign).map(ParsedInteger::U16),
            DesireInteger::U32 => return self.parse_integer::<u32>(sign).map(ParsedInteger::U32),
            DesireInteger::U64 => return self.parse_integer::<u64>(sign).map(ParsedInteger::U64),
            #[cfg(feature = "integer128")]
            DesireInteger::U128 => {
                return self.parse_integer::<u128>(sign).map(ParsedInteger::U128)
            }
        }

        if is_negative {
            let signed = self.parse_integer::<LargeSInt>(-1)?;

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

        let unsigned = self.parse_integer::<LargeUInt>(1)?;

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

    pub fn any_number(&mut self) -> Result<Number> {
        if self.next_bytes_is_float() {
            return match self.any_float(DesireFloat::Any)? {
                ParsedFloat::F32(v) => Ok(Number::F32(v.into())),
                ParsedFloat::F64(v) => Ok(Number::F64(v.into())),
            };
        }

        let bytes_backup = *self;

        let (integer_err, integer_bytes) = match self.any_integer(DesireInteger::Any) {
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

        *self = bytes_backup;

        // Fall-back to parse an out-of-range integer as a float
        match self.any_float(DesireFloat::Any) {
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
        if self.consume("true") {
            Ok(true)
        } else if self.consume("false") {
            Ok(false)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn char(&mut self) -> Result<char> {
        if !self.consume("'") {
            return Err(Error::ExpectedChar);
        }

        let c = self.peek_or_eof()?;

        let c = if c == b'\\' {
            let _ = self.advance(1);

            self.parse_escape()?
        } else {
            // Check where the end of the char (') is and try to
            // interpret the rest as UTF-8

            let max = self.bytes.len().min(5);
            let pos: usize = self.bytes[..max]
                .iter()
                .position(|&x| x == b'\'')
                .ok_or(Error::ExpectedChar)?;
            let s = from_utf8(&self.bytes[0..pos]).map_err(Error::from)?;
            let mut chars = s.chars();

            let first = chars.next().ok_or(Error::ExpectedChar)?;
            if chars.next().is_some() {
                return Err(Error::ExpectedChar);
            }

            let _ = self.advance(pos);

            first
        };

        if !self.consume("'") {
            return Err(Error::ExpectedChar);
        }

        Ok(c)
    }

    pub fn comma(&mut self) -> Result<bool> {
        self.skip_ws()?;

        if self.consume(",") {
            self.skip_ws()?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Only returns true if the char after `ident` cannot belong
    /// to an identifier.
    pub fn check_ident(&mut self, ident: &str) -> bool {
        self.test_for(ident) && !self.check_ident_other_char(ident.len())
    }

    fn check_ident_other_char(&self, index: usize) -> bool {
        self.bytes
            .get(index)
            .map_or(false, |&b| is_ident_other_char(b))
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
            bytes: &mut Bytes,
            newtype: NewtypeMode,
            tuple: TupleMode,
        ) -> Result<StructType> {
            if matches!(newtype, NewtypeMode::NoParensMeanUnit) && !bytes.consume("(") {
                return Ok(StructType::Unit);
            }

            bytes.skip_ws()?;

            if bytes.identifier().is_ok() {
                bytes.skip_ws()?;

                match bytes.peek() {
                    // Definitely a struct with named fields
                    Some(b':') => return Ok(StructType::Named),
                    // Definitely a tuple struct with fields
                    Some(b',') => return Ok(StructType::Tuple),
                    // Either a newtype or a tuple struct
                    Some(b')') => return Ok(StructType::NewtypeOrTuple),
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
                bytes.skip_ws()?;
                let mut bytes_copy = *bytes;
                if bytes_copy.char().is_ok() {
                    *bytes = bytes_copy;
                }
                let mut bytes_copy = *bytes;
                if bytes_copy.string().is_ok() {
                    *bytes = bytes_copy;
                }

                let c = bytes.eat_byte()?;
                if c == b'(' || c == b'[' || c == b'{' {
                    braces += 1;
                } else if c == b')' || c == b']' || c == b'}' {
                    braces -= 1;
                } else if c == b',' && braces == 1 {
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
            Ok(maybe_ident) => std::str::from_utf8(maybe_ident)?,
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

    pub fn consume(&mut self, s: &str) -> bool {
        if self.test_for(s) {
            let _ = self.advance(s.len());

            true
        } else {
            false
        }
    }

    fn consume_all(&mut self, all: &[&str]) -> Result<bool> {
        all.iter()
            .map(|elem| {
                if self.consume(elem) {
                    self.skip_ws()?;

                    Ok(true)
                } else {
                    Ok(false)
                }
            })
            .try_fold(true, |acc, x| x.map(|x| acc && x))
    }

    pub fn eat_byte(&mut self) -> Result<u8> {
        let peek = self.peek_or_eof()?;
        let _ = self.advance_single();

        Ok(peek)
    }

    pub fn expect_byte(&mut self, byte: u8, error: Error) -> Result<()> {
        self.eat_byte()
            .and_then(|b| if b == byte { Ok(()) } else { Err(error) })
    }

    /// Returns the extensions bit mask.
    fn extensions(&mut self) -> Result<Extensions> {
        if self.peek() != Some(b'#') {
            return Ok(Extensions::empty());
        }

        if !self.consume_all(&["#", "!", "[", "enable", "("])? {
            return Err(Error::ExpectedAttribute);
        }

        self.skip_ws()?;
        let mut extensions = Extensions::empty();

        loop {
            let ident = self.identifier()?;
            let extension = Extensions::from_ident(ident).ok_or_else(|| {
                Error::NoSuchExtension(String::from_utf8_lossy(ident).into_owned())
            })?;

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

    pub fn any_float(&mut self, desire: DesireFloat) -> Result<ParsedFloat> {
        const F32_SUFFIX: &str = "_f32";
        const F64_SUFFIX: &str = "_f64";

        for (literal, value_f32, value_f64) in &[
            ("inf", f32::INFINITY, f64::INFINITY),
            ("+inf", f32::INFINITY, f64::INFINITY),
            ("-inf", f32::NEG_INFINITY, f64::NEG_INFINITY),
            ("NaN", f32::NAN, f64::NAN),
            ("+NaN", f32::NAN, f64::NAN),
            ("-NaN", -f32::NAN, -f64::NAN),
        ] {
            if self.consume_ident(literal) {
                return match desire {
                    // Prefer f32 over f64 for equivalent literal values
                    DesireFloat::Any => Ok(ParsedFloat::F32(*value_f32)),
                    DesireFloat::F32 => Ok(ParsedFloat::F32(*value_f32)),
                    DesireFloat::F64 => Ok(ParsedFloat::F64(*value_f64)),
                };
            }

            if self.bytes.starts_with(literal.as_bytes())
                && self.bytes[literal.len()..].starts_with(F32_SUFFIX.as_bytes())
                && !self.check_ident_other_char(literal.len() + F32_SUFFIX.len())
            {
                let _ = self.advance(literal.len() + F32_SUFFIX.len());
                return Ok(ParsedFloat::F32(*value_f32));
            }

            if self.bytes.starts_with(literal.as_bytes())
                && self.bytes[literal.len()..].starts_with(F64_SUFFIX.as_bytes())
                && !self.check_ident_other_char(literal.len() + F64_SUFFIX.len())
            {
                let _ = self.advance(literal.len() + F64_SUFFIX.len());
                return Ok(ParsedFloat::F64(*value_f64));
            }
        }

        let num_bytes = self.next_bytes_contained_in(is_float_char);

        if num_bytes == 0 {
            return Err(Error::ExpectedFloat);
        }

        if self.peek_or_eof()? == b'_' {
            return Err(Error::UnderscoreAtBeginning);
        }

        let mut f = String::with_capacity(num_bytes);
        let mut allow_underscore = false;

        for (i, b) in self.bytes[0..num_bytes].iter().enumerate() {
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

            f.push(char::from(*b));
        }

        let value = f64::from_str(&f).map_err(|_| Error::ExpectedFloat)?;

        let _ = self.advance(num_bytes);

        if self.consume_ident("f32") {
            return Ok(ParsedFloat::F32(value as f32));
        }

        if self.consume_ident("f64") {
            return Ok(ParsedFloat::F64(value));
        }

        match desire {
            // Prefer f32 over f64 for equivalent floating point values
            DesireFloat::Any => {
                if value.total_cmp(&f64::from(value as f32)).is_eq() {
                    Ok(ParsedFloat::F32(value as f32))
                } else {
                    Ok(ParsedFloat::F64(value))
                }
            }
            DesireFloat::F32 => Ok(ParsedFloat::F32(value as f32)),
            DesireFloat::F64 => Ok(ParsedFloat::F64(value)),
        }
    }

    pub fn identifier(&mut self) -> Result<&'a [u8]> {
        let next = self.peek_or_eof()?;
        if !is_ident_first_char(next) {
            if is_ident_raw_char(next) {
                let ident_bytes = &self.bytes[..self.next_bytes_contained_in(is_ident_raw_char)];

                if let Ok(ident) = std::str::from_utf8(ident_bytes) {
                    return Err(Error::SuggestRawIdentifier(String::from(ident)));
                }
            }

            return Err(Error::ExpectedIdentifier);
        }

        // If the next two bytes signify the start of a raw string literal,
        // return an error.
        let length = if next == b'r' {
            match self.bytes.get(1) {
                Some(b'"') => return Err(Error::ExpectedIdentifier),
                Some(b'#') => {
                    let after_next = self.bytes.get(2).copied().unwrap_or_default();
                    // Note: it's important to check this before advancing forward, so that
                    // the value-type deserializer can fall back to parsing it differently.
                    if !is_ident_raw_char(after_next) {
                        return Err(Error::ExpectedIdentifier);
                    }
                    // skip "r#"
                    let _ = self.advance(2);
                    self.next_bytes_contained_in(is_ident_raw_char)
                }
                _ => {
                    let std_ident_length = self.next_bytes_contained_in(is_ident_other_char);
                    let raw_ident_length = self.next_bytes_contained_in(is_ident_raw_char);

                    if raw_ident_length > std_ident_length {
                        if let Ok(ident) = std::str::from_utf8(&self.bytes[..raw_ident_length]) {
                            return Err(Error::SuggestRawIdentifier(String::from(ident)));
                        }
                    }

                    std_ident_length
                }
            }
        } else {
            let std_ident_length = self.next_bytes_contained_in(is_ident_other_char);
            let raw_ident_length = self.next_bytes_contained_in(is_ident_raw_char);

            if raw_ident_length > std_ident_length {
                if let Ok(ident) = std::str::from_utf8(&self.bytes[..raw_ident_length]) {
                    return Err(Error::SuggestRawIdentifier(String::from(ident)));
                }
            }

            std_ident_length
        };

        let ident = &self.bytes[..length];
        let _ = self.advance(length);

        Ok(ident)
    }

    pub fn next_bytes_contained_in(&self, allowed: fn(u8) -> bool) -> usize {
        self.bytes.iter().take_while(|&&b| allowed(b)).count()
    }

    pub fn next_bytes_is_float(&self) -> bool {
        if let Some(byte) = self.peek() {
            let skip = match byte {
                b'+' | b'-' => 1,
                _ => 0,
            };
            let flen = self
                .bytes
                .iter()
                .skip(skip)
                .take_while(|&&b| is_float_char(b))
                .count();
            let ilen = self
                .bytes
                .iter()
                .skip(skip)
                .take_while(|&&b| is_int_char(b))
                .count();
            flen > ilen
        } else {
            false
        }
    }

    pub fn skip_ws(&mut self) -> Result<()> {
        loop {
            while self.peek().map_or(false, is_whitespace_char) {
                let _ = self.advance_single();
            }

            if !self.skip_comment()? {
                return Ok(());
            }
        }
    }

    pub fn peek(&self) -> Option<u8> {
        self.bytes.first().copied()
    }

    pub fn peek_or_eof(&self) -> Result<u8> {
        self.bytes.first().copied().ok_or(Error::Eof)
    }

    pub fn string(&mut self) -> Result<ParsedStr<'a>> {
        if self.consume("\"") {
            self.escaped_string()
        } else if self.consume("r") {
            self.raw_string()
        } else {
            Err(Error::ExpectedString)
        }
    }

    fn escaped_string(&mut self) -> Result<ParsedStr<'a>> {
        use std::iter::repeat;

        let (i, end_or_escape) = self
            .bytes
            .iter()
            .enumerate()
            .find(|&(_, &b)| b == b'\\' || b == b'"')
            .ok_or(Error::ExpectedStringEnd)?;

        if *end_or_escape == b'"' {
            let s = from_utf8(&self.bytes[..i]).map_err(Error::from)?;

            // Advance by the number of bytes of the string
            // + 1 for the `"`.
            let _ = self.advance(i + 1);

            Ok(ParsedStr::Slice(s))
        } else {
            let mut i = i;
            let mut s: Vec<_> = self.bytes[..i].to_vec();

            loop {
                let _ = self.advance(i + 1);
                let character = self.parse_escape()?;
                match character.len_utf8() {
                    1 => s.push(character as u8),
                    len => {
                        let start = s.len();
                        s.extend(repeat(0).take(len));
                        character.encode_utf8(&mut s[start..]);
                    }
                }

                let (new_i, end_or_escape) = self
                    .bytes
                    .iter()
                    .enumerate()
                    .find(|&(_, &b)| b == b'\\' || b == b'"')
                    .ok_or(Error::ExpectedStringEnd)?;

                i = new_i;
                s.extend_from_slice(&self.bytes[..i]);

                if *end_or_escape == b'"' {
                    let _ = self.advance(i + 1);

                    let s = String::from_utf8(s).map_err(Error::from)?;
                    break Ok(ParsedStr::Allocated(s));
                }
            }
        }
    }

    fn raw_string(&mut self) -> Result<ParsedStr<'a>> {
        let num_hashes = self.bytes.iter().take_while(|&&b| b == b'#').count();
        let hashes = &self.bytes[..num_hashes];
        let _ = self.advance(num_hashes);

        if !self.consume("\"") {
            return Err(Error::ExpectedString);
        }

        let ending = [&[b'"'], hashes].concat();
        let i = self
            .bytes
            .windows(num_hashes + 1)
            .position(|window| window == ending.as_slice())
            .ok_or(Error::ExpectedStringEnd)?;

        let s = from_utf8(&self.bytes[..i]).map_err(Error::from)?;

        // Advance by the number of bytes of the string
        // + `num_hashes` + 1 for the `"`.
        let _ = self.advance(i + num_hashes + 1);

        Ok(ParsedStr::Slice(s))
    }

    fn test_for(&self, s: &str) -> bool {
        self.bytes.starts_with(s.as_bytes())
    }

    fn decode_ascii_escape(&mut self) -> Result<u8> {
        let mut n = 0;
        for _ in 0..2 {
            n <<= 4;
            let byte = self.eat_byte()?;
            let decoded = self.decode_hex(byte)?;
            n |= decoded;
        }

        Ok(n)
    }

    #[inline]
    fn decode_hex(&self, c: u8) -> Result<u8> {
        match c {
            c @ b'0'..=b'9' => Ok(c - b'0'),
            c @ b'a'..=b'f' => Ok(10 + c - b'a'),
            c @ b'A'..=b'F' => Ok(10 + c - b'A'),
            _ => Err(Error::InvalidEscape("Non-hex digit found")),
        }
    }

    fn parse_escape(&mut self) -> Result<char> {
        let c = match self.eat_byte()? {
            b'\'' => '\'',
            b'"' => '"',
            b'\\' => '\\',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'0' => '\0',
            b'x' => self.decode_ascii_escape()? as char,
            b'u' => {
                self.expect_byte(b'{', Error::InvalidEscape("Missing { in Unicode escape"))?;

                let mut bytes: u32 = 0;
                let mut num_digits = 0;

                while num_digits < 6 {
                    let byte = self.peek_or_eof()?;

                    if byte == b'}' {
                        break;
                    } else {
                        self.advance_single()?;
                    }

                    let byte = self.decode_hex(byte)?;
                    bytes <<= 4;
                    bytes |= u32::from(byte);

                    num_digits += 1;
                }

                if num_digits == 0 {
                    return Err(Error::InvalidEscape(
                        "Expected 1-6 digits, got 0 digits in Unicode escape",
                    ));
                }

                self.expect_byte(
                    b'}',
                    Error::InvalidEscape("No } at the end of Unicode escape"),
                )?;
                char_from_u32(bytes).ok_or(Error::InvalidEscape("Not a valid char"))?
            }
            _ => {
                return Err(Error::InvalidEscape("Unknown escape character"));
            }
        };

        Ok(c)
    }

    fn skip_comment(&mut self) -> Result<bool> {
        if self.consume("/") {
            match self.eat_byte()? {
                b'/' => {
                    let bytes = self.bytes.iter().take_while(|&&b| b != b'\n').count();

                    let _ = self.advance(bytes);
                }
                b'*' => {
                    let mut level = 1;

                    while level > 0 {
                        let bytes = self
                            .bytes
                            .iter()
                            .take_while(|&&b| b != b'/' && b != b'*')
                            .count();

                        if self.bytes.is_empty() {
                            return Err(Error::UnclosedBlockComment);
                        }

                        let _ = self.advance(bytes);

                        // check whether / or * and take action
                        if self.consume("/*") {
                            level += 1;
                        } else if self.consume("*/") {
                            level -= 1;
                        } else {
                            self.eat_byte().map_err(|_| Error::UnclosedBlockComment)?;
                        }
                    }
                }
                b => return Err(Error::UnexpectedByte(b as char)),
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

pub enum DesireInteger {
    Any,
    I8,
    I16,
    I32,
    I64,
    #[cfg(feature = "integer128")]
    I128,
    U8,
    U16,
    U32,
    U64,
    #[cfg(feature = "integer128")]
    U128,
}

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

impl ParsedInteger {
    #[must_use]
    #[cold]
    pub fn into_error_found_message(self) -> String {
        match self {
            ParsedInteger::I8(v) => format!("the specifically 8-bit signed integer `{}`", v),
            ParsedInteger::I16(v) => format!("the specifically 16-bit signed integer `{}`", v),
            ParsedInteger::I32(v) => format!("the specifically 32-bit signed integer `{}`", v),
            ParsedInteger::I64(v) => format!("the specifically 64-bit signed integer `{}`", v),
            #[cfg(feature = "integer128")]
            ParsedInteger::I128(v) => format!("the specifically 128-bit signed integer `{}`", v),
            ParsedInteger::U8(v) => format!("the specifically 8-bit unsigned integer `{}`", v),
            ParsedInteger::U16(v) => format!("the specifically 16-bit unsigned integer `{}`", v),
            ParsedInteger::U32(v) => format!("the specifically 32-bit unsigned integer `{}`", v),
            ParsedInteger::U64(v) => format!("the specifically 64-bit unsigned integer `{}`", v),
            #[cfg(feature = "integer128")]
            ParsedInteger::U128(v) => format!("the specifically 128-bit unsigned integer `{}`", v),
        }
    }
}

pub enum DesireFloat {
    Any,
    F32,
    F64,
}

pub enum ParsedFloat {
    F32(f32),
    F64(f64),
}

#[derive(Clone, Debug)]
pub enum ParsedStr<'a> {
    Allocated(String),
    Slice(&'a str),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_x10() {
        let mut bytes = Bytes::new(b"10").unwrap();
        assert_eq!(bytes.decode_ascii_escape(), Ok(0x10));
    }
}
