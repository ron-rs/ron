use std::ops::Neg;
use std::str::{FromStr, from_utf8_unchecked};

use de::{Error, Result};

const DIGITS: &[u8] = b"0123456789";
const FLOAT_CHARS: &[u8] = b"0123456789.+-eE";
const WHITE_SPACE: &[u8] = b"\n\t\r ";

#[derive(Clone, Copy)]
pub struct Bytes<'a> {
    bytes: &'a [u8],
    column: usize,
    line: usize,
}

impl<'a> Bytes<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Bytes {
            bytes,
            column: 1,
            line: 1,
        }
    }

    pub fn advance(&mut self, bytes: usize) -> Result<()> {
        for _ in 0..bytes {
            self.advance_single()?;
        }

        Ok(())
    }

    pub fn advance_single(&mut self) -> Result<()> {
        if self.peek().ok_or(Error::Eof)? == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.bytes = &self.bytes[1..];

        Ok(())
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

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn char(&mut self) -> Result<char> {
        if !self.consume("'") {
            return Err(Error::ExpectedChar);
        }

        let c = self.eat_byte()?;

        let c = if c == b'\\' {
            let c = self.eat_byte()?;

            if c != b'\\' && c != b'\'' {
                return Err(Error::InvalidEscape);
            }

            c
        } else {
            c
        };

        if !self.consume("'") {
            return Err(Error::ExpectedChar);
        }

        Ok(c as char)
    }

    pub fn comma(&mut self) -> bool {
        if self.consume(",") {
            self.skip_ws();

            true
        } else {
            false
        }
    }

    pub fn consume(&mut self, s: &str) -> bool {
        if s.bytes().enumerate().all(|(i, b)| self.bytes.get(i).map(|t| *t == b).unwrap_or(false)) {
            let _ = self.advance(s.len());

            true
        } else {
            false
        }
    }

    pub fn eat_byte(&mut self) -> Result<u8> {
        if let Some(peek) = self.peek() {
            let _ = self.advance_single();

            Ok(peek)
        } else {
            Err(Error::Eof)
        }
    }

    pub fn float<T>(&mut self) -> Result<T>
        where T: FromStr
    {
        let num_bytes = self.next_bytes_contained_in(FLOAT_CHARS);

        let res = FromStr::from_str(unsafe { from_utf8_unchecked(&self.bytes[0..num_bytes]) })
            .map_err(|_| Error::ExpectedFloat);

        let _ = self.advance(num_bytes);

        res
    }

    pub fn next_bytes_contained_in(&self, allowed: &[u8]) -> usize {
        (0..)
            .flat_map(|i| self.bytes.get(i))
            .filter(|b| allowed.contains(b))
            .fold(1, |acc, _| acc + 1)
    }

    pub fn skip_ws(&mut self) {
        while self.peek().map(|c| WHITE_SPACE.contains(&c)).unwrap_or(false) {
            let _ = self.advance_single();
        }
    }

    pub fn option(&mut self) -> Result<Option<>>

    pub fn peek(&self) -> Option<u8> {
        self.bytes.get(0).map(|b| *b)
    }

    pub fn signed_integer<T>(&mut self) -> Result<T> where T: FromStr + Neg<Output = T> {
        match self.peek() {
            Some(b'+') => {
                let _ = self.advance_single();

                self.unsigned_integer()
            }
            Some(b'-') => {
                let _ = self.advance_single();

                self.unsigned_integer::<T>().map(Neg::neg)
            }
            Some(_) => self.unsigned_integer(),
            None => Err(Error::Eof),
        }
    }

    pub fn unsigned_integer<T>(&mut self) -> Result<T> where T: FromStr {
        let num_bytes = self.next_bytes_contained_in(DIGITS);

        if num_bytes == 0 {
            return Err(Error::Eof);
        }

        let res = FromStr::from_str(unsafe { from_utf8_unchecked(&self.bytes[0..num_bytes]) })
            .map_err(|_| Error::ExpectedInteger);

        let _ = self.advance(num_bytes);

        res
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub col: usize,
    pub line: usize,
}
