//! Grammar-conformance differential: the grammar transcribed verbatim from
//! `docs/grammar.md` (`src/ron.pest`) vs the real parser.
//!
//! Run in the dedicated CI job (see the "Conformance" job in ci.yaml):
//!
//! ```text
//! cargo test --manifest-path conformance/Cargo.toml -- --nocapture
//! ```

use ron_conformance::{
    ref_accepts, ron_accepts, CONFORMANT, KNOWN_DOC_BUGS, SEMANTIC_NOT_GRAMMAR,
};

fn yn(b: bool) -> &'static str {
    if b {
        "accept"
    } else {
        "reject"
    }
}

/// The grammar and the parser agree everywhere in the conformant battery.
#[test]
fn conformant_battery_agrees() {
    let mut mismatches = Vec::new();
    for (input, note) in CONFORMANT {
        let r = ref_accepts(input);
        let o = ron_accepts(input);
        if r != o {
            mismatches.push(format!(
                "  `{}` ({}): grammar {} vs parser {}",
                input.escape_debug(),
                note,
                yn(r),
                yn(o)
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "grammar/parser disagree on inputs that should conform \
         (if intentional, move them to KNOWN_DOC_BUGS):\n{}",
        mismatches.join("\n"),
    );
}

/// Each of the seven ron-rs/ron#614 findings is real and pinned: the transcription
/// is faithful to the doc, the parser behaves as claimed, and the two disagree.
#[test]
fn known_doc_bugs_are_real() {
    for bug in KNOWN_DOC_BUGS {
        let r = ref_accepts(bug.input);
        let o = ron_accepts(bug.input);

        assert_eq!(
            r, bug.doc_accepts,
            "finding {}: transcription of docs/grammar.md should {} `{}` — {}",
            bug.finding,
            yn(bug.doc_accepts),
            bug.input.escape_debug(),
            bug.summary,
        );
        assert_eq!(
            o, bug.parser_accepts,
            "finding {}: the parser should {} `{}` — {}",
            bug.finding,
            yn(bug.parser_accepts),
            bug.input.escape_debug(),
            bug.summary,
        );
        assert_ne!(
            bug.doc_accepts, bug.parser_accepts,
            "finding {}: doc and parser must actually disagree — {}",
            bug.finding, bug.summary,
        );
        println!(
            "finding {}: `{}`  doc={}  parser={}  — {}",
            bug.finding,
            bug.input.escape_debug(),
            yn(bug.doc_accepts),
            yn(bug.parser_accepts),
            bug.summary,
        );
    }
}

/// Syntactically-valid inputs that ron rejects for semantic reasons are NOT
/// grammar-doc bugs: the grammar accepts them, the parser rejects them, and
/// `docs/grammar.md` should not change.
#[test]
fn semantic_rejections_are_not_grammar_bugs() {
    for (input, note) in SEMANTIC_NOT_GRAMMAR {
        assert!(
            ref_accepts(input),
            "`{}` ({}) should be syntactically accepted by the grammar",
            input,
            note
        );
        assert!(
            !ron_accepts(input),
            "`{}` ({}) should be rejected by the parser (semantic layer)",
            input,
            note
        );
    }
}
