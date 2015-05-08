use std::fmt;
use rustc_serialize;


#[derive(Clone, Copy, Debug)]
pub enum Error {
    Format(fmt::Error),
    Expectation(Expect),
}

impl From<fmt::Error> for Error {
    fn from(err: fmt::Error) -> Error {
        Error::Format(err)
    }
}

enum Format<'a> {
    Compact,
    Pretty {
        indent: &'a str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Expect {
    Constant,
    Element,
}

struct State {
    expect: Expect,
    indent: u32,
    start_line: bool,
}

/// A structure for implementing serialization to RON.
pub struct Encoder<'a> {
    writer: &'a mut (fmt::Write+'a),
    format : Format<'a>,
    state: State,
}

impl<'a> Encoder<'a> {
    /// Creates a new encoder whose output will be written in compact
    /// RON to the specified writer
    pub fn new(writer: &'a mut fmt::Write) -> Encoder<'a> {
        Encoder {
            writer: writer,
            format: Format::Compact,
            state: State {
                expect: Expect::Element,
                indent: 0,
                start_line: false,
            },
        }
    }

    /// Creates a new encoder whose output will be written in pretty
    /// RON to the specified writer
    pub fn new_pretty(writer: &'a mut fmt::Write, indent: &'a str) -> Encoder<'a> {
        Encoder {
            writer: writer,
            format: Format::Pretty {
                indent: indent,
            },
            state: State {
                expect: Expect::Element,
                indent: 0,
                start_line: false,
            },
        }
    }

    fn emit_constant<T: fmt::Display>(&mut self, v: T) -> Result<(), Error> {
        match self.state.expect {
            Expect::Element | Expect::Constant => {
                try!(write!(self.writer, "{}", v));
                Ok(())
            },
            //_ => Err(Error::Expectation(self.state.expect)),
        }
    }

    fn emit_escape<T: fmt::Display>(&mut self, v: T, escape: char) -> Result<(), Error> {
        match self.state.expect {
            Expect::Constant | Expect::Element => {
                try!(write!(self.writer, "{}{}{}", escape, v, escape));
                Ok(())
            },
            //_ => Err(Error::Expectation(self.state.expect)),
        }
    }

    fn new_line(&mut self) -> Result<(), fmt::Error> {
        if let Format::Pretty { indent } = self.format {
            try!(write!(self.writer, "\n"));
            for _ in 0 .. self.state.indent {
                try!(write!(self.writer, "{}", indent));
            }
        }
        Ok(())
    }

    fn get_space(&self) -> &'static str {
        match self.format {
            Format::Pretty{..} => " ",
            Format::Compact => "",
        }
    }
}

impl<'a> rustc_serialize::Encoder for Encoder<'a> {
    type Error = Error;

    fn emit_nil(&mut self) -> Result<(), Error> {
        match self.state.expect {
            Expect::Element => {
                try!(write!(self.writer, "()"));
                Ok(())
            },
            _ => Err(Error::Expectation(self.state.expect)),
        }
    }

    fn emit_usize(&mut self, v: usize)  -> Result<(), Error> { self.emit_constant(v) }
    fn emit_u64(&mut self, v: u64)      -> Result<(), Error> { self.emit_constant(v) }
    fn emit_u32(&mut self, v: u32)      -> Result<(), Error> { self.emit_constant(v) }
    fn emit_u16(&mut self, v: u16)      -> Result<(), Error> { self.emit_constant(v) }
    fn emit_u8(&mut self, v: u8)        -> Result<(), Error> { self.emit_constant(v) }

    fn emit_isize(&mut self, v: isize)  -> Result<(), Error> { self.emit_constant(v) }
    fn emit_i64(&mut self, v: i64)      -> Result<(), Error> { self.emit_constant(v) }
    fn emit_i32(&mut self, v: i32)      -> Result<(), Error> { self.emit_constant(v) }
    fn emit_i16(&mut self, v: i16)      -> Result<(), Error> { self.emit_constant(v) }
    fn emit_i8(&mut self, v: i8)        -> Result<(), Error> { self.emit_constant(v) }

    fn emit_bool(&mut self, v: bool)    -> Result<(), Error> { self.emit_constant(v) }
    fn emit_f64(&mut self, v: f64)      -> Result<(), Error> { self.emit_constant(v) }
    fn emit_f32(&mut self, v: f32)      -> Result<(), Error> { self.emit_constant(v) }

    fn emit_char(&mut self, v: char)    -> Result<(), Error> { self.emit_escape(v, '\'') }
    fn emit_str(&mut self, v: &str)     -> Result<(), Error> { self.emit_escape(v, '\"') }

    fn emit_enum<F>(&mut self, _name: &str, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        if self.state.expect == Expect::Element {
            f(self)
        } else {
            Err(Error::Expectation(self.state.expect))
        }
    }

    fn emit_enum_variant<F>(&mut self, name: &str, _id: usize, cnt: usize, f: F)
                         -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_struct(name, cnt, f)
    }

    fn emit_enum_variant_arg<F>(&mut self, idx: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_tuple_arg(idx, f)
    }

    fn emit_enum_struct_variant<F>(&mut self, name: &str, _id: usize, _cnt: usize, f: F)
                                -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        try!(write!(self.writer, "{}{{", name));
        self.state.indent += 1;
        try!(f(self));
        self.state.indent -= 1;
        try!(self.new_line());
        try!(write!(self.writer, "}}"));
        Ok(())
    }

    fn emit_enum_struct_variant_field<F>(&mut self, name: &str, idx: usize, f: F)
                                      -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_struct_field(name, idx, f)
    }

    fn emit_struct<F>(&mut self, name: &str, len: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        if self.state.expect != Expect::Element {
            return Err(Error::Expectation(self.state.expect))
        }
        if len == 0 {
            try!(write!(self.writer, "{}", name));
        } else {
            try!(write!(self.writer, "{}(", name));
            if len > 1 {
                self.state.start_line = true;
                self.state.indent += 1;
                try!(f(self));
                self.state.indent -= 1;
                try!(self.new_line());
            } else {
                self.state.start_line = false;
                try!(f(self));
            }
            try!(write!(self.writer, ")"));
        }
        Ok(())
    }

    fn emit_struct_field<F>(&mut self, name: &str, _idx: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        try!(self.new_line());
        let space = self.get_space();
        try!(write!(self.writer, "{}:{}", name, space));
        try!(f(self));
        try!(write!(self.writer, ","));
        Ok(())
    }

    fn emit_tuple<F>(&mut self, len: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_struct("", len, f)
    }

    fn emit_tuple_arg<F>(&mut self, _idx: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        if self.state.start_line {
            try!(self.new_line());
            try!(f(self));
            try!(write!(self.writer, ","));
            Ok(())
        } else {
            f(self)
        }
    }

    fn emit_tuple_struct<F>(&mut self, name: &str, len: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_struct(name, len, f)
    }

    fn emit_tuple_struct_arg<F>(&mut self, idx: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_tuple_arg(idx, f)
    }

    fn emit_option<F>(&mut self, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_enum("", f)
    }

    fn emit_option_none(&mut self) -> Result<(), Error> {
        try!(write!(self.writer, "None"));
        Ok(())
    }

    fn emit_option_some<F>(&mut self, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_struct("Some", 1, f)
    }

    fn emit_seq<F>(&mut self, len: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        if self.state.expect != Expect::Element {
            return Err(Error::Expectation(self.state.expect))
        }
        if len == 0 {
            try!(write!(self.writer, "[]"));
        } else {
            self.state.start_line = true;
            try!(write!(self.writer, "["));
            self.state.indent += 1;
            try!(f(self));
            self.state.indent -= 1;
            try!(self.new_line());
            try!(write!(self.writer, "]"));
        }
        Ok(())
    }

    fn emit_seq_elt<F>(&mut self, idx: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        self.emit_tuple_arg(idx, f)
    }

    fn emit_map<F>(&mut self, len: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        if self.state.expect != Expect::Element {
            return Err(Error::Expectation(self.state.expect))
        }
        if len == 0 {
            try!(write!(self.writer, "{{}}"));
        } else {
            try!(write!(self.writer, "{{"));
            self.state.indent += 1;
            try!(f(self));
            self.state.indent -= 1;
            try!(self.new_line());
            try!(write!(self.writer, "}}"));
        }
        Ok(())
    }

    fn emit_map_elt_key<F>(&mut self, _idx: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        try!(self.new_line());
        let expect = self.state.expect;
        self.state.expect = Expect::Constant;
        try!(f(self));
        try!(write!(self.writer, ","));
        self.state.expect = expect;
        Ok(())
    }

    fn emit_map_elt_val<F>(&mut self, _idx: usize, f: F) -> Result<(), Error> where
        F: FnOnce(&mut Encoder<'a>) -> Result<(), Error>,
    {
        let space = self.get_space();
        try!(write!(self.writer, ":{}", space));
        let expect = self.state.expect;
        self.state.expect = Expect::Element;
        try!(f(self));
        self.state.expect = expect;
        Ok(())
    }
}
