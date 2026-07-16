#![no_main]
//! Fixpoint round-trip target.
//!
//! `data` is parsed to a `Value`. NOTE that this first `Value` is not
//! necessarily canonical: a suffixed literal like `1.5f64` parses to `F64`, but
//! the default serializer emits no suffix, so the *canonical* form is reached
//! only after one serialize+parse. We therefore anchor the laws on `canon`:
//!   * the serializer's own output must always parse (self-consistency);
//!   * on the canonical value, serialize is idempotent and serialize->parse is
//!     a stable fixpoint.
//! A failure is a genuine serializer/parser disagreement.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    if data.len() >= 50_000 {
        return;
    }
    let Ok(v) = ron::from_str::<ron::Value>(data) else {
        return;
    };
    let Ok(s1) = ron::to_string(&v) else { return };
    // (self-consistency) our own output must parse.
    let canon: ron::Value = match ron::from_str(&s1) {
        Ok(c) => c,
        Err(e) => panic!("serializer emitted RON the parser rejects: {}\n---\n{}\n---", e, s1),
    };
    let s2 = ron::to_string(&canon).expect("canonical value must re-serialize");
    let canon2: ron::Value = match ron::from_str(&s2) {
        Ok(c) => c,
        Err(e) => panic!("canonical serialization unparseable: {}\n---\n{}\n---", e, s2),
    };
    assert_eq!(s1, s2, "serialization is not idempotent under one round-trip");
    assert_eq!(canon, canon2, "canonical value is not a ser->parse fixpoint");
});
