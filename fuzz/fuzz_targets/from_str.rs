#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![warn(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::unreachable)]
#![deny(unsafe_code)]
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    if data.len() < 50_000 {
        if let Ok(value) = ron::from_str::<ron::Value>(data) {
            let _ = ron::to_string(&value);
        }
    }
});
