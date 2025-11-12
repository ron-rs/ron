#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![warn(clippy::pedantic)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![deny(clippy::panic)]
#![warn(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::unreachable)]
#![deny(unsafe_code)]
#![allow(clippy::missing_errors_doc)] // FIXME
#![warn(clippy::alloc_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![warn(clippy::std_instead_of_core)]
#![doc = include_str!("../README.md")]
#![doc(html_root_url = "https://docs.rs/ron/0.12.0")]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod de;
pub mod ser;

pub mod error;
pub mod value;

pub mod extensions;

pub mod options;
pub mod util;

pub use de::{from_str, Deserializer};
pub use error::{Error, Result};
pub use options::Options;
pub use ser::{to_string, Serializer};
pub use value::{Map, Number, Value};

mod parse;
