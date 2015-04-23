extern crate rustc_serialize;

mod encode;
mod decode;

pub use self::encode::{Encoder, EncoderError, Expect};

/// Shortcut function to encode a `T` into a RON string
pub fn encode<'a, T: rustc_serialize::Encodable>(object: &T, indent: Option<&'a str>)
              -> Result<String, EncoderError> {
    let mut s = String::new();
    {
        let mut encoder = match indent {
            Some(sin) => Encoder::new_pretty(&mut s, sin),
            None => Encoder::new(&mut s),
        };
        object.encode(&mut encoder)
    }.map(|_| s)
}
