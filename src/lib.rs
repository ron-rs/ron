extern crate rustc_serialize;

use std::fmt;


#[derive(Clone, Copy, Debug)]
pub enum EncoderError {
    Format(fmt::Error),
    BadHashmapKey,
}

impl From<fmt::Error> for EncoderError {
    fn from(err: fmt::Error) -> EncoderError {
        EncoderError::Format(err)
    }
}

enum EncodingFormat {
    Compact,
    Pretty {
        curr_indent: u32,
        indent: u32
    }
}

enum Expect {
    Element,
}

/// A structure for implementing serialization to RON.
pub struct Encoder<'a> {
    writer: &'a mut (fmt::Write+'a),
    format : EncodingFormat,
    expect: Expect,
}

 /// Creates a new encoder whose output will be written in compact
/// JSON to the specified writer
pub fn new<'a>(writer: &'a mut fmt::Write) -> Encoder<'a> {
    Encoder {
        writer: writer,
        format: EncodingFormat::Pretty {
            curr_indent: 0,
            indent: 2,
        },
        expect: Expect::Element,
    }
}


impl<'a> rustc_serialize::Encoder for Encoder<'a> {
    type Error = EncoderError;

    fn emit_nil(&mut self) -> Result<(), EncoderError> {
        //if self.is_emitting_map_key { return Err(EncoderError::BadHashmapKey); }
        try!(write!(self.writer, "()"));
        Ok(())
    }
}
