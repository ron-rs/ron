#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "bench/lib.rs"]
mod typed_data;

fuzz_target!(|data: &[u8]| {
    typed_data::roundtrip_arbitrary_typed_ron_or_panic(data);
});
