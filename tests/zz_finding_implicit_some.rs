//! Minimal reproduction of the one non-memory-safety wart the property audit
//! surfaced automatically: under the `IMPLICIT_SOME` extension, serialization
//! of a `Value::Map` is NOT injective, and can emit a map with duplicate keys
//! that silently loses an entry on re-parse.
//!
//! Root cause is the documented `IMPLICIT_SOME` ambiguity class (a bare `()`
//! can mean either `Unit` or `Some(Unit)`); this test just pins the concrete
//! data-loss consequence. It is a correctness wart, NOT a soundness hole.

use ron::extensions::Extensions;
use ron::value::Value;
use ron::Options;

#[test]
fn implicit_some_map_key_collision_loses_entry() {
    let opts = Options::default().with_default_extension(Extensions::IMPLICIT_SOME);

    // Two DISTINCT keys: Some(Unit) and Unit.
    let v: Value = vec![
        (Value::Option(Some(Box::new(Value::Unit))), Value::Unit),
        (Value::Unit, Value::Unit),
    ]
    .into_iter()
    .collect();

    let s = opts.to_string(&v).unwrap();
    // Both keys serialize to `()`, producing a duplicate-key map.
    assert_eq!(s, "{():(),():()}");

    // Re-parsing deduplicates: two entries in, one entry out — silent loss.
    let back: Value = opts.from_str(&s).unwrap();
    // `let...else` is 1.65+, but ron's MSRV is 1.64 — use an explicit match.
    let m = match &back {
        Value::Map(m) => m,
        _ => panic!("expected a map"),
    };
    assert_eq!(
        m.len(),
        1,
        "entry silently lost on round-trip under IMPLICIT_SOME"
    );

    // Sanity: WITHOUT implicit_some the two keys stay distinct and it round-trips.
    let plain = ron::to_string(&v).unwrap();
    let plain_back: Value = ron::from_str(&plain).unwrap();
    let m2 = match &plain_back {
        Value::Map(m2) => m2,
        _ => panic!("expected a map"),
    };
    assert_eq!(m2.len(), 2, "default config must preserve both keys");
}
