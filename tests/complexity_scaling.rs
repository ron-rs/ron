//! Complexity / scaling regression tests for the parser.
//!
//! ron has ~450 correctness tests but, until now, **zero** tests that guard the
//! *asymptotic* cost of parsing. That blind spot is not hypothetical: PR #534
//! ("Speed up escaped byte buf parsing") turned `escaped_byte_buf` from O(n) into
//! O(n^2) and the regression lived from v0.9.0 through v0.12.2 — four releases,
//! two years — with every one of those correctness tests staying green. A second,
//! independent O(n^2) sat in `next_bytes_is_float` from v0.12.2. Both are the same
//! shape: an unbounded `find(..)` that scans to the end of its input, called once
//! per element inside a loop over the same input.
//!
//! The tests below encode a machine-checkable invariant: *parsing time must grow
//! at most linearly in input size*. They are deliberately assertion-on-**ratio**,
//! not on wall-clock milliseconds, so they do not depend on the speed of the
//! machine running them:
//!
//!   * Double the input, measure the time. Linear code multiplies time by ~2;
//!     quadratic code multiplies it by ~4.
//!   * We double several times and take the **median** doubling ratio, so a single
//!     noisy sample cannot flip the verdict.
//!   * Each measurement is the **minimum** of a few runs: scheduler / cache noise
//!     can only ever *add* time, so the minimum is the cleanest estimate of the
//!     real cost.
//!
//! A linear form lands around 2.0, a quadratic form around 4.0, and the threshold
//! sits at 3.0 — a wide margin on both sides. These are `#[ignore]`d so they never
//! run (or slow down) the normal test job; a dedicated CI job runs them in
//! `--release`. See the "Scaling" job in `.github/workflows/ci.yaml`.
//!
//! Run locally with:
//!
//! ```text
//! cargo test --release --test complexity_scaling -- --ignored --nocapture
//! ```

use std::time::{Duration, Instant};

/// Each size is measured this many times; we keep the fastest run. Noise only
/// adds time, so the minimum is the least-contaminated estimate of real cost.
const REPEATS: u32 = 4;

/// Verdict threshold on the median doubling ratio. Linear parsing sits near 2.0,
/// quadratic near 4.0; 3.0 is the wide gap between them.
const RATIO_LIMIT: f64 = 3.0;

/// Time a single closure `REPEATS` times, returning the fastest observed run.
fn min_time(mut run: impl FnMut()) -> Duration {
    let mut best = Duration::MAX;
    for _ in 0..REPEATS {
        let start = Instant::now();
        run();
        let elapsed = start.elapsed();
        if elapsed < best {
            best = elapsed;
        }
    }
    best
}

/// Measure how parse time scales as the input doubles.
///
/// `build(n)` produces a RON document whose relevant element count is `n`;
/// `parse(&str)` parses it (its result is dropped — we only care about the cost).
/// For each adjacent pair of sizes we record `t(2n) / t(n)`; the returned verdict
/// is the median of those ratios (robust to one slow sample). The per-size timings
/// are returned too so a failing test can print the full curve.
fn median_doubling_ratio(
    sizes: &[usize],
    build: impl Fn(usize) -> String,
    parse: impl Fn(&str),
) -> (f64, Vec<(usize, Duration)>) {
    let timings: Vec<(usize, Duration)> = sizes
        .iter()
        .map(|&n| {
            let input = build(n);
            let time = min_time(|| parse(&input));
            (n, time)
        })
        .collect();

    let mut ratios: Vec<f64> = timings
        .windows(2)
        .map(|w| w[1].1.as_secs_f64() / w[0].1.as_secs_f64())
        .collect();
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = ratios[ratios.len() / 2];

    (median, timings)
}

/// Assert that a parse form grows at most linearly with input size.
fn assert_linear(
    name: &str,
    sizes: &[usize],
    build: impl Fn(usize) -> String,
    parse: impl Fn(&str),
) {
    let (median, timings) = median_doubling_ratio(sizes, build, parse);
    println!("{name}: median doubling ratio = {median:.2}  (limit {RATIO_LIMIT})");
    for (n, time) in &timings {
        println!("    n = {n:>9}  ->  {time:?}");
    }
    assert!(
        median < RATIO_LIMIT,
        "{name}: median doubling ratio {median:.2} >= {RATIO_LIMIT} -> super-linear (likely O(n^2)).\n\
         Each doubling of the input multiplied parse time by ~{median:.1}x instead of ~2x.\n\
         Timings: {timings:?}",
    );
}

// --- Quadratic bugs this suite is designed to catch --------------------------

/// A string literal made of `n` escape sequences (`"\n\n\n..."`).
///
/// Regressed by #534 (v0.9.0..=v0.12.2): `escaped_byte_buf` called `find('"')` —
/// which scans all the way to the closing quote — once per escape, so a string
/// with `n` escapes rescans its tail `n` times. O(n^2) on *any* deserialize path.
#[test]
#[ignore = "timing-based; run in the dedicated release CI job"]
fn escaped_string_is_linear() {
    assert_linear(
        "escaped_string",
        &[25_000, 50_000, 100_000, 200_000],
        |n| {
            let mut s = String::with_capacity(n * 2 + 2);
            s.push('"');
            for _ in 0..n {
                s.push_str("\\n");
            }
            s.push('"');
            s
        },
        |input| {
            let _ = ron::from_str::<String>(input).unwrap();
        },
    );
}

/// A flat list of `n` floats, parsed through the self-describing `Value` path.
///
/// Regressed in v0.12.2 (#602 number ranges): `next_bytes_is_float` called
/// `find("..")` over the entire remaining input to detect a range separator,
/// even though the result is immediately clamped to the current number's length.
/// A document with no `..` therefore scans to EOF once per number. O(n^2).
///
/// This is not only about floats: on the self-describing `Value` path
/// `next_bytes_is_float` runs for *every* number to disambiguate int from float,
/// so the same blow-up hits integer sequences and map keys/values too.
#[test]
#[ignore = "timing-based; run in the dedicated release CI job"]
fn floats_via_value_is_linear() {
    assert_linear(
        "floats_via_value",
        &[8_000, 16_000, 32_000, 64_000],
        |n| {
            let mut s = String::with_capacity(n * 8 + 2);
            s.push('[');
            for i in 0..n {
                if i != 0 {
                    s.push(',');
                }
                s.push_str(&i.to_string());
                s.push_str(".5");
            }
            s.push(']');
            s
        },
        |input| {
            let _ = ron::from_str::<ron::Value>(input).unwrap();
        },
    );
}

// --- Linear controls ---------------------------------------------------------
// These forms are already O(n). They keep the suite honest: the harness must
// pass cleanly on well-behaved parsing, or a failure above means nothing.

/// A flat list of `n` integers, parsed into `Vec<i64>`.
#[test]
#[ignore = "timing-based; run in the dedicated release CI job"]
fn int_list_is_linear() {
    assert_linear(
        "int_list",
        &[50_000, 100_000, 200_000, 400_000],
        |n| {
            let mut s = String::with_capacity(n * 7 + 2);
            s.push('[');
            for i in 0..n {
                if i != 0 {
                    s.push(',');
                }
                s.push_str(&i.to_string());
            }
            s.push(']');
            s
        },
        |input| {
            let _ = ron::from_str::<Vec<i64>>(input).unwrap();
        },
    );
}

/// A list of `n` short strings with no escapes, parsed into `Vec<String>`.
///
/// Exercises the string path's fast branch (no escape → borrow the slice between
/// quotes), which is the counterpart to `escaped_string_is_linear`: it confirms
/// the harness does not flag well-behaved string parsing.
#[test]
#[ignore = "timing-based; run in the dedicated release CI job"]
fn plain_strings_is_linear() {
    assert_linear(
        "plain_strings",
        &[50_000, 100_000, 200_000, 400_000],
        |n| {
            let mut s = String::with_capacity(n * 5 + 2);
            s.push('[');
            for i in 0..n {
                if i != 0 {
                    s.push(',');
                }
                s.push_str("\"ab\"");
            }
            s.push(']');
            s
        },
        |input| {
            let _ = ron::from_str::<Vec<String>>(input).unwrap();
        },
    );
}
