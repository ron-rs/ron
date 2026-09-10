#![no_main]
//! Fixpoint round-trip target.
//!
//! Parse -> serialize -> parse must agree on the canonical value, and the
//! serializer must never emit RON its own parser rejects.
//!
//! Values that contain a float are SKIPPED: the f32 Value round-trip drift is a
//! known, separately-tracked issue (ron-rs/ron#613), and leaving it in would
//! halt the fuzzer on the very first float. Skipping it lets this target spend
//! its whole budget hunting for *new*, non-float serializer/parser
//! disagreements (integers, strings, bytes, chars, maps, seqs, options, units,
//! and nesting thereof).

use libfuzzer_sys::fuzz_target;
use ron::value::Number;
use ron::Value;

fn contains_float(v: &Value) -> bool {
    match v {
        Value::Number(n) => matches!(n, Number::F32(_) | Number::F64(_)),
        Value::Option(Some(b)) => contains_float(b),
        Value::Seq(xs) => xs.iter().any(contains_float),
        Value::Map(m) => m
            .iter()
            .any(|(k, val)| contains_float(k) || contains_float(val)),
        _ => false,
    }
}

fuzz_target!(|data: &str| {
    if data.len() >= 50_000 {
        return;
    }
    let Ok(v) = ron::from_str::<Value>(data) else {
        return;
    };
    if contains_float(&v) {
        return; // known drift: ron-rs/ron#613
    }
    let Ok(s1) = ron::to_string(&v) else { return };
    let canon: Value = match ron::from_str(&s1) {
        Ok(c) => c,
        Err(e) => panic!(
            "serializer emitted RON the parser rejects: {}\n---\n{}\n---",
            e, s1
        ),
    };
    // canon can still gain a float only via number narrowing of a huge integer;
    // guard again so the drift can't sneak back in on the second hop.
    if contains_float(&canon) {
        return;
    }
    let s2 = ron::to_string(&canon).expect("canonical value must re-serialize");
    let canon2: Value = match ron::from_str(&s2) {
        Ok(c) => c,
        Err(e) => panic!(
            "canonical serialization unparseable: {}\n---\n{}\n---",
            e, s2
        ),
    };
    assert_eq!(
        s1, s2,
        "serialization is not idempotent under one round-trip"
    );
    assert_eq!(
        canon, canon2,
        "canonical value is not a ser->parse fixpoint"
    );
});
