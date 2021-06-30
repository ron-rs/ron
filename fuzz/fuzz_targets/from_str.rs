#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let _ = ron::from_str::<ron::Value>(data);
});
