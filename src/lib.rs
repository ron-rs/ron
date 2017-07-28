//extern crate rustc_serialize;
//
//pub mod encode;
//pub mod decode;
//
//pub use self::encode::Encoder;
//pub use self::decode::Decoder;
//
///// Shortcut function to encode a `T` into a RON string
//pub fn encode<'a, T: rustc_serialize::Encodable>(object: &T, indent: Option<&'a str>)
//              -> Result<String, encode::Error> {
//    let mut s = String::new();
//    {
//        let mut encoder = match indent {
//            Some(sin) => Encoder::new_pretty(&mut s, sin),
//            None => Encoder::new(&mut s),
//        };
//        object.encode(&mut encoder)
//    }.map(|_| s)
//}
//
///// Shortcut function to decode a RON `&str` into an object
//pub fn decode<T: ::rustc_serialize::Decodable>(s: &str) -> Result<T, decode::Error> {
//    let mut decoder = Decoder::new(s.chars());
//    rustc_serialize::Decodable::decode(&mut decoder)
//}

extern crate serde;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;

pub mod de;
pub mod ser;
