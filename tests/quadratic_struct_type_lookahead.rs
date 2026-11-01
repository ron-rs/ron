//! Regression test: the struct type lookahead must stay linear on malformed strings.
//!
//! `Parser::check_struct_type` scans ahead to tell a newtype from a tuple. It
//! used to backtrack on any string error except a missing closing quote, so a
//! string like `"\"\"\"...\q"` was re-parsed from every escaped `"` up to the
//! invalid escape at its end — quadratic.
//!
//! The assertion is on *scaling*, not on absolute time, so it stays meaningful on
//! slow/noisy CI machines: doubling the input should roughly double the work
//! (ratio ~2). Before the fix the ratio is ~4. The threshold sits far from both.

use std::time::Instant;

use ron::error::{Error, SpannedError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Untagged {
    A(#[allow(dead_code)] ron::Value),
}

fn malformed_string_in_variant(prefix: &str, n: usize) -> String {
    let mut s = format!("A({}\"", prefix);
    for _ in 0..n {
        s.push_str("\\\"");
    }
    s.push_str("\\q\")");
    s
}

fn parse_millis(prefix: &str, n: usize) -> f64 {
    let src = malformed_string_in_variant(prefix, n);
    let start = Instant::now();
    let _err = ron::from_str::<Untagged>(&src).unwrap_err();
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    elapsed
}

fn assert_linear(prefix: &str) {
    const N: usize = 4_000;

    // Warm up so allocator/caches don't skew the first sample.
    parse_millis(prefix, N / 10);

    let base = parse_millis(prefix, N);
    let double = parse_millis(prefix, N * 2);

    // Floor the denominator so timer noise on a fast linear parse (a few
    // milliseconds at most) can't inflate the ratio. The quadratic parse takes
    // seconds, so the floor doesn't hide it.
    let ratio = double / base.max(10.0);
    assert!(
        ratio < 3.0,
        "a malformed {}string with {} escapes took {:.1}ms but {} escapes took {:.1}ms \
         (ratio {:.2}); expected ~2 (linear), ~4 means the quadratic rescan is back",
        prefix,
        N,
        base,
        N * 2,
        double,
        ratio
    );
}

#[test]
fn malformed_string_lookahead_scales_linearly() {
    assert_linear("");
}

#[test]
fn malformed_byte_string_lookahead_scales_linearly() {
    assert_linear("b");
}

#[test]
fn malformed_string_lookahead_reports_escape_error() {
    for prefix in ["", "b"] {
        assert!(matches!(
            ron::from_str::<Untagged>(&malformed_string_in_variant(prefix, 2)),
            Err(SpannedError {
                code: Error::InvalidEscape("Unknown escape character"),
                ..
            })
        ));
    }

    // raw identifiers and identifiers starting with `r` or `b` still backtrack
    assert!(ron::from_str::<Untagged>("A(r#rust, br, bar)").is_ok());
    assert!(ron::from_str::<Untagged>("A(\"a\\\"b\", b\"c\", r#\"d\"#)").is_ok());
}
