use crate::{
    de::{Position, Span},
    error::SpannedResult,
};
use alloc::string::String;

impl Position {
    /// Given a Position and a string, return the 0-indexed grapheme index into the
    /// string at that position, or [None] if the Position is out of bounds of the string.
    #[must_use]
    pub fn grapheme_index(&self, s: &str) -> Option<usize> {
        use unicode_segmentation::UnicodeSegmentation;
        let mut line_no = 1;
        let mut col_no = 1;

        if (self.line, self.col) == (1, 1) {
            return Some(0);
        }

        let mut i = 0;

        // Slightly non-intuitive arithmetic: a zero-length string at line 1, col 1 -> 0

        if (line_no, col_no) == (self.line, self.col) {
            return Some(i);
        }

        for ch in s.graphemes(true) {
            if (line_no, col_no) == (self.line, self.col) {
                return Some(i);
            }

            // "\n" and "\r\n" each come through the iterator as a single grapheme
            if matches!(ch, "\n" | "\r\n") {
                line_no += 1;
                col_no = 1;
            } else {
                col_no += 1;
            }

            i += 1;
        }

        // ...and a string of length 7 at line 1, col 8 -> 7
        if (line_no, col_no) == (self.line, self.col) {
            return Some(i);
        }

        None
    }
}

impl Span {
    /// Given a `Span` and a string, form the resulting string selected exclusively (as in `[start..end`]) by the `Span`
    /// or [`None`] if the span is out of bounds of the string at either end.
    #[must_use]
    pub fn substring_exclusive(&self, s: &str) -> Option<String> {
        use alloc::vec::Vec;
        use unicode_segmentation::UnicodeSegmentation;

        if let (Some(start), Some(end)) = (self.start.grapheme_index(s), self.end.grapheme_index(s))
        {
            Some(s.graphemes(true).collect::<Vec<&str>>()[start..end].concat())
        } else {
            None
        }
    }

    /// Given a `Span` and a string, form the resulting string selected inclusively (as in `[start..=end]`) by the `Span`
    /// or [`None`] if the span is out of bounds of the string at either end.
    #[must_use]
    pub fn substring_inclusive(&self, s: &str) -> Option<String> {
        use alloc::vec::Vec;
        use unicode_segmentation::UnicodeSegmentation;

        if let (Some(start), Some(end)) = (self.start.grapheme_index(s), self.end.grapheme_index(s))
        {
            Some(s.graphemes(true).collect::<Vec<&str>>()[start..=end].concat())
        } else {
            None
        }
    }
}

/// Given a string `ron`, a [`SpannedResult`], and a substring, verify that trying to parse `ron` results in an error
/// equal to the [`SpannedResult`] with a Span that exclusively (as in `[start..end]`) selects that substring.
/// Note that there are two versions of this helper, inclusive and exclusive. This is because while the parser cursor
/// arithmetic that computes span positions always produces exclusive spans (as in `[start..end]`),
/// when doing validation against a target substring, the inclusive check including the final grapheme that triggered
/// the error is often a more intuitive target to check against.
/// Meanwhile, if the parser threw an EOF, for example, there is no final grapheme to check, and so
/// only the exclusive check would produce a meaningful result.
#[allow(clippy::unwrap_used)]
#[allow(clippy::missing_panics_doc)]
pub fn check_error_span_exclusive<T: serde::de::DeserializeOwned + PartialEq + core::fmt::Debug>(
    ron: &str,
    check: SpannedResult<T>,
    substr: &str,
) {
    let res_str = crate::de::from_str::<T>(ron);
    assert_eq!(res_str, check);

    let res_bytes = crate::de::from_bytes::<T>(ron.as_bytes());
    assert_eq!(res_bytes, check);

    #[cfg(feature = "std")]
    {
        let res_reader = crate::de::from_reader::<&[u8], T>(ron.as_bytes());
        assert_eq!(res_reader, check);
    }

    assert_eq!(
        check.unwrap_err().span.substring_exclusive(ron).unwrap(),
        substr
    );
}

/// Given a string `ron`, a [`SpannedResult`], and a substring, verify that trying to parse `ron` results in an error
/// equal to the [`SpannedResult`] with a Span that inclusively (as in `[start..=end`]) selects that substring.
/// See [`check_error_span_exclusive`] for the rationale behind both versions of this helper.
#[allow(clippy::unwrap_used)]
#[allow(clippy::missing_panics_doc)]
pub fn check_error_span_inclusive<T: serde::de::DeserializeOwned + PartialEq + core::fmt::Debug>(
    ron: &str,
    check: SpannedResult<T>,
    substr: &str,
) {
    let res_str = crate::de::from_str::<T>(ron);
    assert_eq!(res_str, check);

    let res_bytes = crate::de::from_bytes::<T>(ron.as_bytes());
    assert_eq!(res_bytes, check);

    #[cfg(feature = "std")]
    {
        let res_reader = crate::de::from_reader::<&[u8], T>(ron.as_bytes());
        assert_eq!(res_reader, check);
    }

    assert_eq!(
        check.unwrap_err().span.substring_inclusive(ron).unwrap(),
        substr
    );
}

#[cfg(test)]
mod tests {
    use crate::de::{Position, Span};

    fn span(start: Position, end: Position) -> Span {
        Span { start, end }
    }

    fn pos(line: usize, col: usize) -> Position {
        Position { line, col }
    }

    #[test]
    fn ascii_basics() {
        let text = "hello\nworld";

        // first char / first col
        assert_eq!(pos(1, 1).grapheme_index(text), Some(0));

        // last char on first line ('o')
        assert_eq!(pos(1, 5).grapheme_index(text), Some(4));

        // start of second line ('w')
        assert_eq!(pos(2, 1).grapheme_index(text), Some(6));

        // span across the `\n`
        assert_eq!(
            span(pos(1, 4), pos(2, 2))
                .substring_exclusive(text)
                .unwrap(),
            "lo\nw"
        );
    }

    #[test]
    fn multibyte_greek() {
        let text = "αβγ\ndeux\n三四五\r\nend";

        // Beta
        assert_eq!(pos(1, 2).grapheme_index(text), Some(1));

        // 三
        assert_eq!(pos(3, 1).grapheme_index(text), Some(9));

        // e
        assert_eq!(pos(4, 1).grapheme_index(text), Some(13));

        // span from α to start of “deux”
        assert_eq!(
            span(pos(1, 1), pos(2, 1))
                .substring_exclusive(text)
                .unwrap(),
            "αβγ\n"
        );
    }

    #[test]
    fn combining_mark_cluster() {
        // é  ==  [0x65, 0xCC, 0x81] in UTF-8
        let text = "e\u{0301}x\n";

        // grapheme #1 (“é”)
        assert_eq!(pos(1, 1).grapheme_index(text), Some(0));

        // grapheme #2 (“x”)
        assert_eq!(pos(1, 2).grapheme_index(text), Some(1));

        // column 4 is past EOL
        assert_eq!(pos(1, 4).grapheme_index(text), None);

        // full span
        assert_eq!(
            span(pos(1, 1), pos(1, 2))
                .substring_exclusive(text)
                .unwrap(),
            "e\u{0301}"
        );
    }

    #[test]
    fn zwj_emoji_cluster() {
        let text = "👩‍👩‍👧‍👧 and 👨‍👩‍👦";

        // The family emoji is the first grapheme on the line.
        assert_eq!(pos(1, 1).grapheme_index(text), Some(0));

        assert_eq!(pos(1, 2).grapheme_index(text), Some(1));

        // Span selecting only the first emoji
        assert_eq!(
            span(pos(1, 1), pos(1, 2))
                .substring_exclusive(text)
                .unwrap(),
            "👩‍👩‍👧‍👧"
        );

        // Span selecting only the second emoji
        assert_eq!(
            span(pos(1, 7), pos(1, 8))
                .substring_exclusive(text)
                .unwrap(),
            "👨‍👩‍👦"
        );
    }

    #[test]
    fn mixed_newlines() {
        let text = "one\r\ntwo\nthree\r\n";

        // start of “two” (line numbers are 1-based)
        assert_eq!(pos(2, 1).grapheme_index(text), Some(4));

        // “three”
        assert_eq!(pos(3, 1).grapheme_index(text), Some(8));

        // span “two\n”
        assert_eq!(
            span(pos(2, 1), pos(3, 1))
                .substring_exclusive(text)
                .unwrap(),
            "two\n"
        );

        // span “two\nthree”
        assert_eq!(
            span(pos(2, 1), pos(3, 6))
                .substring_exclusive(text)
                .unwrap(),
            "two\nthree"
        );
    }

    #[test]
    fn oob_and_error_paths() {
        let text = "short";

        // line past EOF
        assert_eq!(pos(2, 1).grapheme_index(text), None);

        // column past EOL
        assert_eq!(pos(1, 10).grapheme_index(text), None);

        // span with either endpoint oob → None
        assert_eq!(span(pos(1, 1), pos(2, 1)).substring_exclusive(text), None);
    }

    #[test]
    fn whole_text_span() {
        let text = "αβγ\nδεζ";
        let all = span(pos(1, 1), pos(2, 4));
        assert_eq!(&all.substring_exclusive(text).unwrap(), text);
    }

    #[test]
    fn span_substring_helper() {
        assert_eq!(
            Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 2, col: 1 },
            }
            .substring_exclusive(
                "In the first place, there are two sorts of bets, or toh.11 There is the
single axial bet in the center between the principals (toh ketengah), and
there is the cloud of peripheral ones around the ring between members
of the audience (toh kesasi). ",
            )
            .unwrap(),
            "In the first place, there are two sorts of bets, or toh.11 There is the\n"
        );

        assert_eq!(
            Span {
                start: Position { line: 2, col: 1 },
                end: Position { line: 3, col: 1 },
            }
            .substring_exclusive(
                "In the first place, there are two sorts of bets, or toh.11 There is the
single axial bet in the center between the principals (toh ketengah), and
there is the cloud of peripheral ones around the ring between members
of the audience (toh kesasi). ",
            )
            .unwrap(),
            "single axial bet in the center between the principals (toh ketengah), and\n"
        );
    }
}
