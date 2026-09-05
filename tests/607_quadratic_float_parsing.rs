//! Regression test: parsing number-heavy documents must stay linear.
//!
//! `Parser::next_bytes_is_float` used to call `find("..")` on the whole remaining
//! input while clamping the result to the current float-char run, which made every
//! number scan to EOF on documents containing no `..` at all — quadratic overall.
//!
//! The assertion is on *scaling*, not on absolute time, so it stays meaningful on
//! slow/noisy CI machines: doubling the input should roughly double the work
//! (ratio ~2). Before the fix the ratio is ~4. The threshold sits far from both.

use std::time::Instant;

fn seq_of_floats(n: usize) -> String {
    let mut s = String::from("[");
    for i in 0..n {
        if i > 0 {
            s.push(',');
        }
        s.push_str("1234.5678");
    }
    s.push(']');
    s
}

fn parse_millis(n: usize) -> f64 {
    let src = seq_of_floats(n);
    let start = Instant::now();
    // No `black_box` (it needs Rust 1.66, MSRV is 1.64): the unwrapped
    // cross-crate `from_str` allocates, so the parse isn't optimized away.
    let _value: ron::Value = ron::from_str(&src).unwrap();
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    elapsed
}

#[test]
fn parsing_many_floats_scales_linearly() {
    const N: usize = 10_000;

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
        "parsing {} floats took {:.1}ms but {} floats took {:.1}ms (ratio {:.2}); \
         expected ~2 (linear), ~4 means the quadratic scan is back",
        N,
        base,
        N * 2,
        double,
        ratio
    );
}
