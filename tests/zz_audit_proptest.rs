//! Property-based audit of the RON parser + serializer.
//!
//! Design note on why we test *fixpoints*, not strict `Value -> string -> Value`
//! identity: RON's default serializer emits no integer/float type suffixes, so
//! `Number::U16(5)` prints as `"5"` and parses back as the smallest-fitting
//! `Number::U8(5)`; likewise `F64` narrows to `F32` when it fits. The identity
//! therefore does NOT hold at the `Value` level. What *must* hold for a sound
//! format is:
//!   1. no input (arbitrary bytes/strings) may ever *panic* the parser;
//!   2. the serializer must never emit text the parser then rejects
//!      (self-consistency);
//!   3. once a value is in canonical (parsed) form, ser->parse is a stable
//!      fixpoint across every pretty/compact/extension configuration.
//!
//! These are exactly the invariants that catch memory-safety-adjacent holes
//! (crashes, unbounded recursion, serializer/parser disagreement) without the
//! false positives that a naive round-trip test would drown in.

use proptest::prelude::*;
use ron::extensions::Extensions;
use ron::ser::PrettyConfig;
use ron::value::Number;
use ron::{Options, Value};

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

#[cfg_attr(not(feature = "integer128"), allow(unused_mut))]
fn arb_number() -> impl Strategy<Value = Number> {
    let mut opts = vec![
        any::<i8>().prop_map(Number::I8).boxed(),
        any::<i16>().prop_map(Number::I16).boxed(),
        any::<i32>().prop_map(Number::I32).boxed(),
        any::<i64>().prop_map(Number::I64).boxed(),
        any::<u8>().prop_map(Number::U8).boxed(),
        any::<u16>().prop_map(Number::U16).boxed(),
        any::<u32>().prop_map(Number::U32).boxed(),
        any::<u64>().prop_map(Number::U64).boxed(),
        any::<f32>().prop_map(|f| Number::F32(f.into())).boxed(),
        any::<f64>().prop_map(|f| Number::F64(f.into())).boxed(),
    ];
    #[cfg(feature = "integer128")]
    {
        opts.push(any::<i128>().prop_map(Number::I128).boxed());
        opts.push(any::<u128>().prop_map(Number::U128).boxed());
    }
    proptest::strategy::Union::new(opts)
}

fn arb_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Unit),
        any::<bool>().prop_map(Value::Bool),
        any::<char>().prop_map(Value::Char),
        ".*".prop_map(Value::String),
        proptest::collection::vec(any::<u8>(), 0..16).prop_map(Value::Bytes),
        arb_number().prop_map(Value::Number),
    ];
    // depth 5, up to 64 total nodes, up to 10 items per collection.
    // Well under the default recursion limit of 128 so nesting itself is legal.
    leaf.prop_recursive(5, 64, 10, |inner| {
        prop_oneof![
            proptest::option::of(inner.clone())
                .prop_map(|o| Value::Option(o.map(Box::new))),
            proptest::collection::vec(inner.clone(), 0..8).prop_map(Value::Seq),
            proptest::collection::vec((inner.clone(), inner.clone()), 0..6)
                .prop_map(|kvs| kvs.into_iter().collect::<Value>()),
        ]
    })
}

/// A grab-bag of RON tokens, concatenated, to drive the parser into deep and
/// unusual states far more often than `any::<String>()` would.
fn arb_ron_like() -> impl Strategy<Value = String> {
    let token = prop_oneof![
        Just("("), Just(")"), Just("["), Just("]"), Just("{"), Just("}"),
        Just(":"), Just(","), Just("\""), Just("'"), Just("\\"), Just("//"),
        Just("/*"), Just("*/"), Just("Some"), Just("None"), Just("true"),
        Just("false"), Just("inf"), Just("-inf"), Just("NaN"), Just("0x1f"),
        Just("0b10"), Just("0o7"), Just("1_000"), Just("1.5e-3"), Just("42u8"),
        Just("#![enable(implicit_some)]"), Just("#![enable(unwrap_newtypes)]"),
        Just("#![enable(unwrap_variant_newtypes)]"), Just("r#raw"), Just("b\"\""),
        Just("r\"x\""), Just("\\u{1F600}"), Just("-"), Just("+"), Just("."),
        Just(" "), Just("\n"), Just("\t"), Just("ident"), Just("北"),
    ];
    proptest::collection::vec(token, 0..60).prop_map(|v| v.concat())
}

// ---------------------------------------------------------------------------
// Serialization configurations under test
// ---------------------------------------------------------------------------

fn configs() -> Vec<(&'static str, Options)> {
    let all_ext = Extensions::all();
    vec![
        ("default", Options::default()),
        (
            "implicit_some",
            Options::default().with_default_extension(Extensions::IMPLICIT_SOME),
        ),
        (
            "unwrap_newtypes",
            Options::default().with_default_extension(Extensions::UNWRAP_NEWTYPES),
        ),
        ("all_extensions", Options::default().with_default_extension(all_ext)),
    ]
}

fn pretty_configs() -> Vec<PrettyConfig> {
    vec![
        PrettyConfig::default(),
        PrettyConfig::default().struct_names(true),
        PrettyConfig::default()
            .compact_arrays(true)
            .compact_maps(true)
            .compact_structs(true),
        PrettyConfig::default().indentor("\t".to_string()).depth_limit(64),
        PrettyConfig::default().enumerate_arrays(true),
    ]
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4000))]

    /// (1) The parser must NEVER panic on arbitrary UTF-8 input, whatever the
    /// target type. Only `Ok`/`Err` are acceptable outcomes.
    #[test]
    fn no_panic_on_arbitrary_string(s in ".{0,400}") {
        use std::collections::HashMap;
        let _ = ron::from_str::<Value>(&s);
        let _ = ron::from_str::<HashMap<String, Value>>(&s);
        let _ = ron::from_str::<Vec<i64>>(&s);
        let _ = ron::from_str::<String>(&s);
        let _ = ron::from_str::<f64>(&s);
        let _ = ron::from_str::<(i32, String, bool)>(&s);
        let _ = ron::from_str::<Box<ron::value::RawValue>>(&s);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(6000))]

    /// (1b) Same, but with RON-shaped token soup that reaches deep parser code.
    #[test]
    fn no_panic_on_ron_like(s in arb_ron_like()) {
        let _ = ron::from_str::<Value>(&s);
        // Also stress the extension-parsing front-end explicitly.
        let _ = ron::from_str::<Value>(&format!("#![enable(implicit_some)] {s}"));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// (2)+(3) For every generated Value and every serializer config: if the
    /// value serializes at all, the parser MUST accept that output (2,
    /// self-consistency), and starting from the *canonical* (parsed) form,
    /// ser->parse must be a stable fixpoint and serialization idempotent (3).
    ///
    /// NB: we intentionally do NOT compare `to_string(v)` against
    /// `to_string(canon)` — `v` fresh from the generator need not be canonical
    /// (e.g. under IMPLICIT_SOME a map key `Some(())` and a key `()` both print
    /// as `()`, so the map is not injective and collapses on parse). Idempotence
    /// is only a meaningful law once we are on a canonical value.
    #[test]
    fn compact_fixpoint(v in arb_value()) {
        for (name, opts) in configs() {
            let Ok(s1) = opts.to_string(&v) else { continue };
            // (2) our own output must always parse — a serializer that emits
            // text its own parser rejects is a genuine hole.
            let canon: Value = opts.from_str(&s1)
                .map_err(|e| TestCaseError::fail(
                    format!("[{name}] serializer emitted unparseable RON: {e}\n---\n{s1}\n---")))?;
            // (3) fixpoint + idempotence, anchored on the canonical value.
            let s2 = opts.to_string(&canon)
                .map_err(|e| TestCaseError::fail(format!("[{name}] canon failed to serialize: {e}")))?;
            let canon2: Value = opts.from_str(&s2)
                .map_err(|e| TestCaseError::fail(
                    format!("[{name}] canon output unparseable: {e}\n---\n{s2}\n---")))?;
            let s3 = opts.to_string(&canon2)
                .map_err(|e| TestCaseError::fail(format!("[{name}] canon2 failed to serialize: {e}")))?;
            prop_assert_eq!(&canon, &canon2, "[{}] ser->parse not a fixpoint", name);
            prop_assert_eq!(s2, s3, "[{}] canonical serialization not idempotent", name);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1500))]

    /// (3b) Pretty printing must agree with compact printing: both must parse
    /// back to the same canonical Value, for every PrettyConfig.
    #[test]
    fn pretty_agrees_with_compact(v in arb_value()) {
        let Ok(compact) = ron::to_string(&v) else { return Ok(()) };
        let Ok(canon): Result<Value, _> = ron::from_str(&compact) else { return Ok(()) };
        for pc in pretty_configs() {
            let Ok(pretty) = ron::ser::to_string_pretty(&v, pc.clone()) else { continue };
            let reparsed: Value = ron::from_str(&pretty)
                .map_err(|e| TestCaseError::fail(
                    format!("pretty output rejected by parser: {e}\n---\n{pretty}\n---")))?;
            prop_assert_eq!(&canon, &reparsed, "pretty vs compact disagree");
        }
    }
}
