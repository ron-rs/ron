extern crate rustc_serialize;

mod encode;
mod decode;

pub use self::encode::{Encoder, EncoderError, Expect};

/// Shortcut function to encode a `T` into a RON string
pub fn encode<T: rustc_serialize::Encodable>(object: &T, pretty: bool) -> Result<String, EncoderError> {
    let mut s = String::new();
    if pretty {
        let mut encoder = Encoder::new_pretty(&mut s);
        try!(object.encode(&mut encoder));
    }else {
        let mut encoder = Encoder::new(&mut s);
        try!(object.encode(&mut encoder));
    }
    Ok(s)
}
