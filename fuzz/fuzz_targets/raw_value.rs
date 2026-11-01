#![no_main]
//! Exercises the crate's only `unsafe` code — the `RawValue` transparent-newtype
//! transmutes and the `trim_boxed` in-place `str` drain — under AddressSanitizer.
//! `RawValue::from_ron` validates, `trim`/`trim_boxed` slice on comment/whitespace
//! boundaries; any pointer/length mistake in the unsafe blocks surfaces as an ASAN
//! report here.

use libfuzzer_sys::fuzz_target;
use ron::value::RawValue;

fuzz_target!(|data: &str| {
    if data.len() >= 50_000 {
        return;
    }
    // Borrowed validation + trim (borrowed unsafe cast + slice).
    if let Ok(rv) = RawValue::from_ron(data) {
        let t = rv.trim();
        // trimming is idempotent and stays valid RON.
        assert_eq!(t.trim().get_ron(), t.get_ron());
        let _ = RawValue::from_ron(t.get_ron()).expect("trimmed RawValue must still be valid");
    }
    // Boxed path: from_boxed_ron (Box<str>->Box<RawValue> transmute) + trim_boxed
    // (as_mut_vec drain of both ends).
    if let Ok(boxed) = RawValue::from_boxed_ron(String::from(data).into_boxed_str()) {
        let trimmed = boxed.clone().trim_boxed();
        assert_eq!(trimmed.get_ron(), boxed.trim().get_ron());
        // Box<RawValue> -> Box<str> transmute back.
        let _back: Box<str> = trimmed.into();
    }
});
