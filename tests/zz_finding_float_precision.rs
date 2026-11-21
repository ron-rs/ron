//! Fidelity finding surfaced by the `roundtrip` fuzzer (minimised input
//! `924444480f64..922.`): an `f32`-typed `Value` does not survive a
//! serialize -> parse round-trip, and the numeric VALUE drifts.
//!
//! Chain (all default config, no extensions):
//!   1. `924444480` is exactly representable in f32, so `Value` infers F32.
//!      (ron's "fits in f32" check is otherwise correct: values that would lose
//!      precision, e.g. 16777217, are kept as F64 — so this is NOT a blanket
//!      "Value truncates floats" bug.)
//!   2. Rust's f32 formatter prints that value's SHORTEST round-tripping
//!      decimal, which is "924444500.0" (a different decimal that maps to the
//!      same f32). ron serializes exactly that.
//!   3. Re-parsing "924444500.0": it does NOT round-trip through f32, so ron
//!      falls back to F64 — yielding F64(924444500.0), a value drifted by 20
//!      from the original 924444480.
//!
//! Correctness/fidelity, not a soundness hole — but the value actually changes,
//! which makes it the sharpest of the three findings.

use ron::ser::PrettyConfig;
use ron::value::{Number, Value};

#[test]
fn f32_value_drifts_across_roundtrip() {
    // Parsed from a plain literal, inferred as F32 (924444480 is f32-exact).
    let v: Value = ron::from_str("924444480.0").unwrap();
    assert_eq!(v, Value::Number(Number::F32(924444480.0f32.into())));

    // Serialized via f32's shortest decimal — already a different number as text.
    let s = ron::to_string(&v).unwrap();
    assert_eq!(s, "924444500.0");

    // Re-parsed: no longer fits f32 -> F64, value drifted 924444480 -> 924444500.
    let back: Value = ron::from_str(&s).unwrap();
    assert_ne!(v, back, "value must have changed across the round-trip");
    assert_eq!(back, Value::Number(Number::F64(924444500.0.into())));
}

#[test]
fn number_suffixes_fixes_the_drift() {
    // The disambiguator already exists: `number_suffixes` writes `...f32`, so
    // the parser recovers the exact type/value. It is off by default and only
    // honoured by `to_string_pretty` (compact `to_string` ignores it) — so the
    // "fix" for a lossless f32 Value round-trip is to opt in here.
    let v = Value::Number(Number::F32(924444480.0f32.into()));

    let s = ron::ser::to_string_pretty(&v, PrettyConfig::default().number_suffixes(true)).unwrap();
    assert!(s.contains("f32"), "expected a type suffix, got {s:?}");

    let back: Value = ron::from_str(&s).unwrap();
    assert_eq!(
        v, back,
        "with number_suffixes the f32 Value round-trips exactly"
    );
}
