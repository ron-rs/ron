//! Regression test: parsing strings with many escapes must stay linear.
//!
//! `Parser::escaped_byte_buf` used to call `find('"')` once per escape. That call
//! scans to the closing quote, while the cursor only advances past one escape per
//! iteration, so a string with N escapes rescanned the tail N times — quadratic.
//!
//! The assertion is on *scaling*, not on absolute time, so it stays meaningful on
//! slow/noisy CI machines: doubling the input should roughly double the work
//! (ratio ~2). Before the fix the ratio is ~4. The threshold sits far from both.

use std::time::Instant;

fn string_of_escapes(n: usize) -> String {
    let mut s = String::from("[\"");
    for _ in 0..n {
        s.push_str("\\n");
    }
    s.push_str("\"]");
    s
}

fn parse_millis(n: usize) -> f64 {
    let src = string_of_escapes(n);
    let start = Instant::now();
    let value: ron::Value = ron::from_str(&src).unwrap();
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    std::hint::black_box(&value);
    elapsed
}

#[test]
fn parsing_many_escapes_scales_linearly() {
    const N: usize = 50_000;

    // Warm up so allocator/caches don't skew the first sample.
    parse_millis(N / 10);

    let base = parse_millis(N);
    let double = parse_millis(N * 2);

    // Guard against a uselessly small denominator on a very fast machine.
    if base < 1.0 {
        return;
    }

    let ratio = double / base;
    assert!(
        ratio < 3.0,
        "parsing a string with {} escapes took {:.1}ms but {} escapes took {:.1}ms \
         (ratio {:.2}); expected ~2 (linear), ~4 means the quadratic rescan is back",
        N,
        base,
        N * 2,
        double,
        ratio
    );
}

/// The escaped path is shared with byte strings, and it is the only path that
/// allocates — keep a plain correctness check next to the scaling one.
#[test]
fn escapes_still_parse_correctly() {
    assert_eq!(ron::from_str::<String>(r#""a\nb""#).unwrap(), "a\nb");
    assert_eq!(ron::from_str::<String>(r#""a\"b""#).unwrap(), "a\"b");
    assert_eq!(ron::from_str::<String>(r#""a\\b""#).unwrap(), "a\\b");
    assert_eq!(ron::from_str::<String>(r#""\u{41}\u{42}""#).unwrap(), "AB");
    assert_eq!(ron::from_str::<String>(r#""\n\n\n""#).unwrap(), "\n\n\n");
    // escape immediately before the closing quote, and an empty tail after it
    assert_eq!(ron::from_str::<String>(r#""ab\n""#).unwrap(), "ab\n");
    // no escapes at all still takes the borrowed fast path
    assert_eq!(ron::from_str::<String>(r#""abc""#).unwrap(), "abc");
}
