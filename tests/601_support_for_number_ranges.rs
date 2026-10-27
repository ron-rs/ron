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

#[derive(PartialEq, Deserialize, Serialize, Debug)]
enum RangeVariant {
    Strings(std::ops::Range<String>),
    Numbers(std::ops::Range<i32>),
}

#[test]
fn test_non_numeric_range_falls_back_to_struct_form() {
    // A range whose bounds are not numbers is written as a struct even when
    // `compact_ranges` is enabled. That struct must be written as it would be
    // without `compact_ranges`, including the struct name and newtype variant
    // unwrapping, so that it can be read back.
    let compact = ron::ser::PrettyConfig::new()
        .new_line("")
        .indentor("")
        .separator("")
        .compact_structs(true)
        .compact_ranges(true);

    let str_range = String::from("a")..String::from("z");

    let ser = ron::ser::to_string_pretty(&str_range, compact.clone().struct_names(true)).unwrap();
    assert_eq!(ser, r#"Range(start:"a",end:"z")"#);

    let ser = ron::ser::to_string_pretty(
        &str_range,
        compact
            .clone()
            .extensions(ron::extensions::Extensions::EXPLICIT_STRUCT_NAMES),
    )
    .unwrap();
    assert_eq!(
        ser,
        r#"#![enable(explicit_struct_names)]Range(start:"a",end:"z")"#
    );
    let de: std::ops::Range<String> = ron::from_str(&ser).unwrap();
    assert_eq!(de, str_range);

    let unwrap_variant_newtypes = compact
        .clone()
        .extensions(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES);

    let variant = RangeVariant::Strings(str_range);
    let ser = ron::ser::to_string_pretty(&variant, unwrap_variant_newtypes.clone()).unwrap();
    assert_eq!(
        ser,
        r#"#![enable(unwrap_variant_newtypes)]Strings(start:"a",end:"z")"#
    );
    let de: RangeVariant = ron::from_str(&ser).unwrap();
    assert_eq!(de, variant);

    // Numeric bounds still use the compact syntax inside the variant
    let variant = RangeVariant::Numbers(0..5);
    let ser = ron::ser::to_string_pretty(&variant, unwrap_variant_newtypes).unwrap();
    assert_eq!(ser, "#![enable(unwrap_variant_newtypes)]Numbers(0..5)");
    let de: RangeVariant = ron::from_str(&ser).unwrap();
    assert_eq!(de, variant);
}

#[derive(PartialEq, Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum MaybeFloatRange {
    Range(std::ops::Range<f64>),
    RangeTo(std::ops::RangeTo<f64>),
    Value(f64),
}

#[test]
fn test_untyped_range_inf_nan_end_bound() {
    // `inf` and `NaN` start with a letter, so they were not recognised as
    // the end bound of an untyped range, and e.g. `0.0..inf` was read as
    // `0.0..` followed by trailing characters. The same happened to a byte
    // literal end bound after a bare `..`, e.g. `..b'z'`.
    for (range, fields) in [
        ("0.0..inf", "(start: 0.0, end: inf)"),
        ("-inf..inf", "(start: -inf, end: inf)"),
        ("1..NaN", "(start: 1, end: NaN)"),
        ("0.0..inff64", "(start: 0.0, end: inff64)"),
        ("0.0..NaNf32", "(start: 0.0, end: NaNf32)"),
        ("inf..=inf", "(start: inf, end: inf)"),
        ("..inf", "(end: inf)"),
        ("..NaN", "(end: NaN)"),
        ("..=inff64", "(end: inff64)"),
        ("..b'z'", "(end: b'z')"),
    ] {
        assert_eq!(
            ron::from_str::<ron::Value>(range).unwrap(),
            ron::from_str::<ron::Value>(fields).unwrap(),
            "{}",
            range
        );
    }

    let compact = ron::ser::PrettyConfig::new().compact_ranges(true);

    let range = MaybeFloatRange::Range(0.0..f64::INFINITY);
    let ser = ron::ser::to_string_pretty(&range, compact.clone()).unwrap();
    assert_eq!(ser, "0.0..inf");
    assert_eq!(ron::from_str::<MaybeFloatRange>(&ser).unwrap(), range);

    let range = MaybeFloatRange::RangeTo(..f64::INFINITY);
    let ser = ron::ser::to_string_pretty(&range, compact).unwrap();
    assert_eq!(ser, "..inf");
    assert_eq!(ron::from_str::<MaybeFloatRange>(&ser).unwrap(), range);
}
