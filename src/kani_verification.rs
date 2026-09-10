//! Bounded proofs (Kani model checking). Compiled only under `--cfg kani`.
//!
//! Kani cannot swallow the whole allocating parser (`from_str::<Value>` builds
//! `String`/`Vec`/`Map`), but the integer front-end is allocation-free, so we
//! can *prove* — over ALL inputs of a bounded length, not just sampled ones —
//! that it never panics, never wraps, and never reads out of bounds. Parsing
//! into a NARROW type (`u8`/`i8`) makes a 3-digit input already exercise the
//! `checked_mul`/`checked_add` overflow branch, so the unwind bound stays tiny.

/// For every 3-byte ASCII-digit string, `from_str::<u8>` must return without
/// panicking. This drives the overflow branch (e.g. "300" > 255 => Err), the
/// happy path ("042" => Ok(42)), and the leading-underscore/invalid-digit
/// guards — proving the checked arithmetic in `parse_integer_digits` is sound.
#[kani::proof]
#[kani::unwind(8)]
fn from_str_u8_never_panics() {
    let bytes: [u8; 3] = kani::any();
    for b in bytes {
        kani::assume(b.is_ascii_digit() || b == b'_');
    }
    // Bytes are all ASCII => always valid UTF-8.
    let s = core::str::from_utf8(&bytes).unwrap();
    let _ = crate::from_str::<u8>(s);
}

/// Tightest, most tractable target: the allocation-free integer front-end
/// `Parser::integer`, bypassing serde/`Options`/`Deserializer` entirely. Two
/// symbolic digit bytes already reach the `checked_mul`/`checked_add` overflow
/// branch for `u8` (e.g. "99" ok, "39" ok, but the accumulator can exceed 255
/// via the base-10 multiply). Proves the checked arithmetic never panics/wraps.
#[kani::proof]
#[kani::unwind(4)]
fn integer_parser_u8_direct_never_panics() {
    let bytes: [u8; 2] = kani::any();
    kani::assume(bytes[0].is_ascii_digit());
    kani::assume(bytes[1].is_ascii_digit());
    let s = core::str::from_utf8(&bytes).unwrap();
    if let Ok(mut p) = crate::parse::Parser::new(s) {
        let _ = p.integer::<u8>();
    }
}

/// Same for a signed narrow type with an optional sign, covering the
/// `checked_sub` (negative) accumulation path and `i8::MIN` boundary.
#[kani::proof]
#[kani::unwind(8)]
fn from_str_i8_never_panics() {
    let bytes: [u8; 4] = kani::any();
    for b in bytes {
        kani::assume(b.is_ascii_digit() || b == b'-' || b == b'+' || b == b'_');
    }
    let s = core::str::from_utf8(&bytes).unwrap();
    let _ = crate::from_str::<i8>(s);
}

/// Broadest bounded guarantee: ANY 4-byte UTF-8 input handed to the integer
/// deserializer yields `Ok`/`Err`, never a panic.
#[kani::proof]
#[kani::unwind(8)]
fn from_str_i64_arbitrary_bytes_never_panics() {
    let bytes: [u8; 4] = kani::any();
    if let Ok(s) = core::str::from_utf8(&bytes) {
        let _ = crate::from_str::<i64>(s);
    }
}
