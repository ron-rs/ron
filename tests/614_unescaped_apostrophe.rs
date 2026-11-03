use ron::{
    error::{Position, Span, SpannedError},
    Error, Value,
};

#[test]
fn char_apostrophe_must_be_escaped() {
    assert_eq!(ron::from_str::<char>(r"'\''"), Ok('\''));
    assert_eq!(ron::to_string(&'\'').unwrap(), r"'\''");

    // `''` is not a char, so `'''` must not be read as the char `'`
    for ron in ["''", "'''", "''''"] {
        assert_eq!(
            ron::from_str::<char>(ron),
            Err(SpannedError {
                code: Error::ExpectedChar,
                span: Span {
                    start: Position { line: 1, col: 1 },
                    end: Position { line: 1, col: 3 },
                },
            }),
            "{ron}",
        );
        assert_eq!(
            ron::from_str::<Value>(ron),
            Err(SpannedError {
                code: Error::ExpectedChar,
                span: Span {
                    start: Position { line: 1, col: 1 },
                    end: Position { line: 1, col: 3 },
                },
            }),
            "{ron}",
        );
    }

    assert_eq!(
        ron::from_str::<Vec<char>>("['\\'', 'a']"),
        Ok(vec!['\'', 'a'])
    );
    assert_eq!(
        ron::from_str::<Vec<char>>("[''', 'a']"),
        Err(SpannedError {
            code: Error::ExpectedChar,
            span: Span {
                start: Position { line: 1, col: 2 },
                end: Position { line: 1, col: 4 },
            },
        }),
    );
}

#[test]
fn byte_literal_apostrophe_must_be_escaped() {
    assert_eq!(ron::from_str::<u8>(r"b'\''"), Ok(b'\''));
    assert_eq!(
        ron::from_str::<Value>(r"b'\''"),
        Ok(Value::Number(ron::value::Number::U8(b'\'')))
    );

    for ron in ["b''", "b'''", "b''''"] {
        assert_eq!(
            ron::from_str::<u8>(ron),
            Err(SpannedError {
                code: Error::ExpectedByteLiteral,
                span: Span {
                    start: Position { line: 1, col: 1 },
                    end: Position { line: 1, col: 4 },
                },
            }),
            "{ron}",
        );
        assert_eq!(
            ron::from_str::<Value>(ron),
            Err(SpannedError {
                code: Error::ExpectedByteLiteral,
                span: Span {
                    start: Position { line: 1, col: 1 },
                    end: Position { line: 1, col: 4 },
                },
            }),
            "{ron}",
        );
    }
}
