#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    if let Ok(value) = ron::from_str::<ron::Value>(data) {
        let _ = ron::to_string(&value);
    }
});
