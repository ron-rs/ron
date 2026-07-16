//! Grammar-conformance differential for ron.
//!
//! ron ships a human-readable grammar in [`docs/grammar.md`], but nothing checks
//! that the parser actually agrees with it — there is no grammar-conformance test.
//! This crate transcribes that EBNF **verbatim** into a PEG acceptor ([`ron.pest`])
//! and diffs it, input by input, against the real parser. A disagreement is either
//! a parser bug or a grammar-doc bug.
//!
//! The oracle for "does ron accept this as one syntactically-valid value?" is
//! `Deserializer::from_str -> IgnoredAny::deserialize -> end()`: pure syntactic
//! validation with the whole input consumed and no data model imposed. (`Value`
//! is unusable as the oracle — it has no struct/enum/range/byte variants and would
//! reject or mangle them.)
//!
//! Running the differential over a wide battery — hardened by mutation-fuzzing
//! valid seeds, which is what surfaced the range / char-apostrophe / exponent
//! findings too subtle to hit by hand — turned up **seven** places where
//! `docs/grammar.md` disagrees with the parser; in every case the parser is right
//! and the doc is wrong. They are pinned below in [`KNOWN_DOC_BUGS`] and reported
//! upstream in ron-rs/ron#614. Everywhere else the transcription and the parser
//! agree (see the `grammar_conformance` test).
//!
//! [`docs/grammar.md`]: https://github.com/ron-rs/ron/blob/master/docs/grammar.md
//! [`ron.pest`]: ./ron.pest

use pest::Parser;
use pest_derive::Parser;
use serde::de::{Deserialize, IgnoredAny};

#[derive(Parser)]
#[grammar = "ron.pest"]
struct RonRef;

/// Reference side: does the grammar transcribed from `docs/grammar.md` accept the
/// whole input as one RON value?
pub fn ref_accepts(s: &str) -> bool {
    RonRef::parse(Rule::RON, s).is_ok()
}

/// Parser side (the oracle): does ron accept the input as exactly one
/// syntactically-valid value, consuming all of it?
pub fn ron_accepts(s: &str) -> bool {
    match ron::Deserializer::from_str(s) {
        Ok(mut d) => IgnoredAny::deserialize(&mut d).is_ok() && d.end().is_ok(),
        Err(_) => false,
    }
}

/// Inputs where the transcribed grammar and the parser MUST agree. A broad spread
/// across every production family: if the grammar drifts from the parser here, the
/// `grammar_conformance` test fails. `(input, note)`.
pub const CONFORMANT: &[(&str, &str)] = &[
    // integers / floats / suffixes / bases
    ("5", "int"),
    ("-3", "neg int"),
    ("+5", "explicit plus"),
    ("05", "leading zero"),
    ("3.14", "float"),
    ("1.", "trailing-dot float"),
    (".5", "leading-dot float"),
    ("1u8", "int suffix"),
    ("1.5f32", "float suffix"),
    ("0x1F", "hex"),
    ("0b1010", "binary"),
    ("0o17", "octal"),
    ("1_000", "digit-group underscores"),
    ("1e10", "exponent"),
    ("1E-3", "signed exponent"),
    ("inf", "inf"),
    ("-inf", "neg inf"),
    ("NaN", "NaN"),
    // bool / option / strings / chars / bytes / idents
    ("true", "bool"),
    ("false", "bool"),
    ("None", "option none"),
    ("Some(5)", "option some"),
    ("\"hi\"", "string"),
    ("\"a\\tb\"", "string with valid escape"),
    ("\"\\u{1F600}\"", "string unicode escape (braced form)"),
    ("'a'", "char"),
    ("'\\''", "escaped-apostrophe char"),
    ("[1, 2, 3]", "list"),
    ("{\"a\": 1}", "map"),
    ("(1, 2)", "tuple"),
    ("(1, 2,)", "trailing-comma tuple"),
    ("Foo(1, 2)", "tuple struct"),
    ("Foo(a: 1)", "named struct"),
    ("(a: 1)", "anonymous struct"),
    ("()", "unit"),
    ("Unit", "unit ident"),
    ("information", "ident with inf prefix"),
    ("NaNa", "ident with NaN prefix"),
    ("r\"raw\"", "raw string, 0 hashes"),
    ("r#\"a\"b\"#", "raw string, 1 hash with inner quote"),
    ("b\"bytes\"", "byte string"),
    ("b'x'", "byte literal"),
    ("br\"raw\"", "raw byte string"),
    ("r#name", "raw ident"),
    // ranges (adjacent operator — the whitespace variants are finding D)
    ("1..2", "range excl"),
    ("1..=2", "range incl"),
    ("..2", "range to excl"),
    ("..=2", "range to incl"),
    ("1..", "range from"),
    ("..", "range full"),
    // extensions / comments / whitespace
    ("#![enable(implicit_some)] 5", "extension header"),
    ("#![enable(unwrap_newtypes)]\n(a: 1)", "extension + struct"),
    ("/* c */ 5", "block comment"),
    ("// c\n5", "line comment"),
    ("/* /* nested */ */ 5", "nested block comment"),
    // malformed — both must REJECT
    ("5 5", "two values"),
    ("5,", "trailing comma at top level"),
    ("0x", "empty hex"),
    ("1e", "empty exponent"),
    ("1__2", "double underscore between digits"),
    ("\"unterminated", "unterminated string"),
    ("[1, 2", "unterminated list"),
    ("", "empty input"),
    ("   ", "whitespace only"),
];

/// A place where `docs/grammar.md` disagrees with the parser. Self-checking: the
/// test asserts `ref_accepts(input) == doc_accepts` (the transcription is faithful
/// to the doc), `ron_accepts(input) == parser_accepts` (the parser behaves as
/// claimed), and `doc_accepts != parser_accepts` (this really is a divergence). A
/// wrong field therefore fails the test loudly rather than passing silently.
pub struct DocBug {
    /// finding id in ron-rs/ron#614
    pub finding: &'static str,
    pub input: &'static str,
    /// what a verbatim reading of `docs/grammar.md` does == `ref_accepts(input)`
    pub doc_accepts: bool,
    /// what the parser actually does (the correct behaviour)
    pub parser_accepts: bool,
    pub summary: &'static str,
}

/// The seven divergences reported in ron-rs/ron#614. In every row the parser is
/// right; the grammar doc is wrong. These are the ONLY inputs on which the
/// verbatim transcription and the parser disagree.
pub const KNOWN_DOC_BUGS: &[DocBug] = &[
    DocBug {
        finding: "A",
        input: "\"a\\c\"", // "a\c"
        doc_accepts: true,
        parser_accepts: false,
        summary: "string_std: `no_double_quotation_marks` lets a lone `\\` be literal; \
                  the parser requires every `\\` to start a valid escape.",
    },
    DocBug {
        finding: "B",
        input: "'\\n'", // '\n'
        doc_accepts: false,
        parser_accepts: true,
        summary: "char: grammar allows only `\\\\` / `\\'`, but the parser accepts the \
                  full string escape set (`\\n`, `\\x41`, `\\u{..}`).",
    },
    DocBug {
        finding: "C",
        input: "\"\\uFFFF\"", // "￿" — the braceless doc form
        doc_accepts: true,
        parser_accepts: false,
        summary: "escape_unicode: grammar is `\\u` + bare hex; the parser requires \
                  `\\u{ 1..=6 hex }`.",
    },
    DocBug {
        finding: "D",
        input: "1 ..2", // whitespace before the range operator
        doc_accepts: true,
        parser_accepts: false,
        summary: "range_*: grammar puts `ws` around `..`/`..=`; the parser consumes the \
                  operator adjacent, with no skip_ws.",
    },
    DocBug {
        finding: "E",
        input: "'''", // a bare apostrophe as the char content
        doc_accepts: false,
        parser_accepts: true,
        summary: "char: `no_apostrophe` says a `'` inside a char must be escaped, but the \
                  parser takes a bare `'` as-is (`'''` is the char `'`).",
    },
    DocBug {
        finding: "F",
        input: "1e_+0", // underscore before the exponent sign
        doc_accepts: false,
        parser_accepts: true,
        summary: "float_exp: grammar puts the sign strictly before any `_`; the parser \
                  allows `_` after `e`/`E`, i.e. before the sign.",
    },
    DocBug {
        finding: "G",
        input: " #![enable(implicit_some)]\n(a: 1)", // leading ws before extensions
        doc_accepts: false,
        parser_accepts: true,
        summary: "RON: nothing precedes `[extensions]` in the grammar, but the parser \
                  accepts leading whitespace/comments before `#![enable(..)]`.",
    },
];

/// Inputs that are syntactically valid (the grammar accepts them) but that ron
/// rejects for **semantic** reasons layered on top of the syntax. These are NOT
/// grammar-doc bugs and must not be "fixed" in `docs/grammar.md`; they are pinned
/// so a differential does not mistake them for conformance gaps. Asserted as
/// `ref_accepts == true && ron_accepts == false`.
pub const SEMANTIC_NOT_GRAMMAR: &[(&str, &str)] = &[
    ("911u8", "integer does not fit its suffix (911 > u8::MAX)"),
    ("-1u8", "negative value with an unsigned suffix"),
    ("Some()", "`Some` is reserved for Option and needs exactly one inner value"),
];
