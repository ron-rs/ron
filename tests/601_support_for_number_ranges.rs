use ron;
use serde::{Deserialize, Serialize};

// NOTE:
// std::ops::RangeToInclusive and RangeFull do not have stable serde support out of the box.
// We use `serde(remote = "...")` to define how these std types should be serialized/deserialized.

#[derive(PartialEq, Deserialize, Serialize, Debug)]
#[serde(remote = "std::ops::RangeToInclusive")]
struct RangeToInclusive<T> {
    end: T,
}

#[derive(PartialEq, Deserialize, Serialize, Debug)]
#[serde(remote = "std::ops::RangeFull")]
struct RangeFull;

#[derive(PartialEq, Deserialize, Serialize, Debug)]
struct RangeTest {
    a: std::ops::Range<i32>,
    b: std::ops::RangeInclusive<i32>,
    c: std::ops::Range<f32>,
    d: std::ops::RangeInclusive<f32>,
}

#[test]
fn test_ranges() {
    let ranges = RangeTest {
        a: 0..5,
        b: 1..=3,
        c: 0.6..4.3,
        d: 0.3..=5.7,
    };

    let ser = ron::to_string(&ranges).unwrap();
    assert_eq!(
        ser,
        "(a:(start:0,end:5),b:(start:1,end:3),c:(start:0.6,end:4.3),d:(start:0.3,end:5.7))"
    );

    assert_eq!(
        ron::ser::to_string_pretty(
            &ranges,
            ron::ser::PrettyConfig::new()
                .compact_ranges(true)
                .new_line("")
                .indentor("")
                .separator("")
                .compact_structs(true)
        )
        .unwrap(),
        "(a:0..5,b:1..=3,c:0.6..4.3,d:0.3..=5.7)"
    );

    let de: RangeTest = ron::from_str(&ser).unwrap();
    assert_eq!(de, ranges);
}

#[test]
fn test_range_integer_bases() {
    assert_eq!(
        ron::from_str::<std::ops::Range<u8>>("0b0000..0b0101").unwrap(),
        0..5
    );
    assert_eq!(
        ron::from_str::<std::ops::Range<u8>>("0o0..0o5").unwrap(),
        0..5
    );
    assert_eq!(
        ron::from_str::<std::ops::Range<u8>>("0x0..0x5").unwrap(),
        0..5
    );

    assert_eq!(
        ron::from_str::<std::ops::Range<u8>>("b'\\x00'..b'\\x05'").unwrap(),
        0..5
    );
    assert_eq!(
        ron::from_str::<std::ops::RangeInclusive<u8>>("b'A'..=b'Z'").unwrap(),
        b'A'..=b'Z'
    );

    assert_eq!(
        ron::from_str::<std::ops::RangeTo<u8>>("..0b0101").unwrap(),
        ..5
    );
    assert_eq!(
        ron::from_str::<std::ops::RangeFrom<u8>>("0b0000..").unwrap(),
        0..
    );
}

#[derive(PartialEq, Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum MaybeRange {
    Range(std::ops::Range<i32>),
    RangeFrom(std::ops::RangeFrom<i32>),
    #[serde(with = "RangeFull")]
    RangeFull(std::ops::RangeFull),
    Value(i32),
}

#[test]
fn test_range_untagged() {
    assert_eq!(
        ron::from_str::<MaybeRange>("0..5").unwrap(),
        MaybeRange::Range(0..5)
    );
    assert_eq!(
        ron::from_str::<MaybeRange>("0..").unwrap(),
        MaybeRange::RangeFrom(0..)
    );
    assert_eq!(
        ron::from_str::<MaybeRange>("42").unwrap(),
        MaybeRange::Value(42)
    );
}

#[derive(PartialEq, Deserialize, Serialize, Debug)]
struct UnclosedRangeTest {
    a: std::ops::RangeFrom<i32>,
    b: std::ops::RangeTo<i32>,
    #[serde(with = "RangeToInclusive")]
    c: std::ops::RangeToInclusive<i32>,
    d: std::ops::RangeFrom<f32>,
    e: std::ops::RangeTo<f32>,
    #[serde(with = "RangeToInclusive")]
    f: std::ops::RangeToInclusive<f32>,
    #[serde(with = "RangeFull")]
    g: std::ops::RangeFull,
}

#[test]
fn test_unclosed_ranges() {
    let ranges = UnclosedRangeTest {
        a: 2..,
        b: ..3,
        c: ..=3,
        d: 1.5..,
        e: ..2.3,
        f: ..=2.3,
        g: ..,
    };

    let ser = ron::to_string(&ranges).unwrap();
    assert_eq!(
        ser,
        "(a:(start:2),b:(end:3),c:(end:3),d:(start:1.5),e:(end:2.3),f:(end:2.3),g:())"
    );

    assert_eq!(
        ron::ser::to_string_pretty(
            &ranges,
            ron::ser::PrettyConfig::new()
                .compact_ranges(true)
                .new_line("")
                .indentor("")
                .separator("")
                .compact_structs(true)
        )
        .unwrap(),
        "(a:2..,b:..3,c:..=3,d:1.5..,e:..2.3,f:..=2.3,g:..)"
    );

    let de: UnclosedRangeTest = ron::from_str(&ser).unwrap();
    assert_eq!(de, ranges);
}

#[test]
fn test_string_range() {
    assert!(ron::from_str::<std::ops::Range<&str>>("\"x\"..\"h\"").is_err());
    assert!(ron::from_str::<std::ops::Range<i32>>("\"x\"..\"h\"").is_err());

    let str_range = "a".."z";
    let ser = ron::to_string(&str_range).unwrap();
    assert_eq!(ser, r#"(start:"a",end:"z")"#);

    let pretty_compact = ron::ser::to_string_pretty(
        &str_range,
        ron::ser::PrettyConfig::new()
            .compact_ranges(true)
            .new_line("")
            .indentor("")
            .separator("")
            .compact_structs(true),
    )
    .unwrap();

    assert_eq!(pretty_compact, r#"(start:"a",end:"z")"#);
}

#[test]
fn test_inf_nan_ranges() {
    let r = ron::from_str::<std::ops::RangeFrom<f32>>("inff32..").unwrap();
    assert!(r.start.is_infinite() && r.start.is_sign_positive());

    let r = ron::from_str::<std::ops::RangeFrom<f32>>("NaNf32..").unwrap();
    assert!(r.start.is_nan());

    let r = ron::from_str::<std::ops::RangeFrom<f64>>("inff64..").unwrap();
    assert!(r.start.is_infinite() && r.start.is_sign_positive());

    let r = ron::from_str::<std::ops::RangeFrom<f64>>("NaNf64..").unwrap();
    assert!(r.start.is_nan());

    let r = ron::from_str::<std::ops::Range<f32>>("(start:inf,end:NaN)").unwrap();
    assert!(r.start.is_infinite() && r.start.is_sign_positive());
    assert!(r.end.is_nan());

    let r = ron::from_str::<std::ops::RangeInclusive<f32>>("(start:NaN,end:inf)").unwrap();
    assert!(r.start().is_nan());
    assert!(r.end().is_infinite());
}

// Untagged enum where RangeFull comes after RangeTo and Value
#[derive(PartialEq, Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum MaybeRangeOrValue {
    Range(std::ops::Range<i32>),
    RangeFrom(std::ops::RangeFrom<i32>),
    RangeTo(std::ops::RangeTo<i32>),
    Value(i32),
    #[serde(with = "RangeFull")]
    RangeFull(std::ops::RangeFull),
}

#[test]
fn test_range_full_whitespace_lookahead() {
    // In deserialize_any context, `..` deserializes as a unit value
    assert_eq!(ron::from_str::<ron::Value>("..").unwrap(), ron::Value::Unit);

    // `.. 5` is invalid: `..` consumes the range-full token, but `5` is trailing garbage
    assert!(ron::from_str::<ron::Value>(".. 5").is_err());

    // Untagged enum: `.. 5` must not silently match as RangeFull/unit variant,
    // because `5` is trailing garbage after `..`
    assert!(ron::from_str::<MaybeRange>(".. 5").is_err());

    assert_eq!(
        ron::from_str::<MaybeRangeOrValue>("..5").unwrap(),
        MaybeRangeOrValue::RangeTo(..5)
    );

    // Untagged enum where RangeFull comes after Value: `.. 5` is invalid RON
    // regardless of variant order — `..` is not a valid prefix for integers
    assert!(ron::from_str::<MaybeRangeOrValue>(".. 5").is_err());

    assert_eq!(
        ron::from_str::<MaybeRangeOrValue>("..").unwrap(),
        MaybeRangeOrValue::RangeFull(std::ops::RangeFull)
    );

    // Untagged enum: plain `..` correctly matches the RangeFull variant
    assert_eq!(
        ron::from_str::<MaybeRange>("..").unwrap(),
        MaybeRange::RangeFull(std::ops::RangeFull)
    );
}

#[test]
fn test_range_inf_nan_start_bound_roundtrips() {
    // Regression: the compact-range serializer emits `inf`/`NaN` start bounds
    // (e.g. `inf..2.0`, `NaN..=2.0`), but the `Range`/`RangeInclusive`
    // deserializers only entered the compact-range path when the first byte
    // was recognised by `is_number_start` (digit / `+` / `-` / `.` / `b`).
    // Since `inf` and `NaN` start with a letter, those inputs failed to
    // round-trip with `ExpectedNamedStructLike`, even though `RangeFrom`
    // (e.g. `inf..`) round-tripped fine.
    let compact = ron::ser::PrettyConfig::new().compact_ranges(true);

    // Range<f64> with +inf start
    let r = f64::INFINITY..2.0;
    let s = ron::ser::to_string_pretty(&r, compact.clone()).unwrap();
    assert_eq!(s, "inf..2.0");
    let de: std::ops::Range<f64> = ron::from_str(&s).unwrap();
    assert!(de.start.is_infinite() && de.start.is_sign_positive());
    assert_eq!(de.end, 2.0);

    // Range<f64> with NaN start
    let r = f64::NAN..2.0;
    let s = ron::ser::to_string_pretty(&r, compact.clone()).unwrap();
    assert_eq!(s, "NaN..2.0");
    let de: std::ops::Range<f64> = ron::from_str(&s).unwrap();
    assert!(de.start.is_nan());
    assert_eq!(de.end, 2.0);

    // RangeInclusive<f64> with +inf start
    let r = f64::INFINITY..=2.0;
    let s = ron::ser::to_string_pretty(&r, compact.clone()).unwrap();
    assert_eq!(s, "inf..=2.0");
    let de: std::ops::RangeInclusive<f64> = ron::from_str(&s).unwrap();
    assert!(de.start().is_infinite() && de.start().is_sign_positive());
    assert_eq!(*de.end(), 2.0);

    // RangeInclusive<f64> with NaN start
    let r = f64::NAN..=2.0;
    let s = ron::ser::to_string_pretty(&r, compact.clone()).unwrap();
    assert_eq!(s, "NaN..=2.0");
    let de: std::ops::RangeInclusive<f64> = ron::from_str(&s).unwrap();
    assert!(de.start().is_nan());
    assert_eq!(*de.end(), 2.0);

    // Direct parse of the serializer output (independent of the serializer).
    let de: std::ops::Range<f64> = ron::from_str("inf..2.0").unwrap();
    assert!(de.start.is_infinite());
    let de: std::ops::Range<f64> = ron::from_str("NaN..2.0").unwrap();
    assert!(de.start.is_nan());
}
