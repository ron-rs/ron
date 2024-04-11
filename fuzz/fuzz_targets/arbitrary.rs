#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![warn(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![allow(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::unreachable)]
#![allow(unsafe_code)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "bench/lib.rs"]
mod typed_data;

fuzz_target!(|data: &[u8]| {
    if data.len() < 50_000 {
        typed_data::roundtrip_arbitrary_typed_ron_or_panic(data);
    }
});
