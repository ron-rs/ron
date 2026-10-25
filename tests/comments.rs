use ron::de::{from_str, Error, Position, Span, SpannedError as RonErr};

#[test]
fn test_simple() {
    assert_eq!(
        from_str(
            "/*
 * We got a hexadecimal number here!
 *
 */0x507"
        ),
        Ok(0x507)
    );
}

#[test]
fn test_nested() {
    assert_eq!(
        from_str(
            "/*
        /* quite * some * nesting * going * on * /* here /* (yeah, maybe a bit too much) */ */ */
    */
    // The actual value comes.. /*
    // very soon, these are just checks that */
    // multi-line comments don't trigger in line comments /*
\"THE VALUE\" /* This is the value /* :) */ */
    "
        ),
        Ok("THE VALUE".to_owned())
    );
}

#[test]
fn test_unclosed() {
    assert_eq!(
        from_str::<String>("\"hi\" /*"),
        Err(RonErr {
            code: Error::UnclosedBlockComment,
            span: Span {
                start: Position { line: 1, col: 6 },
                end: Position { line: 1, col: 8 }
            }
        })
    );
    assert_eq!(
        from_str::<String>(
            "/*
        /* quite * some * nesting * going * on * /* here /* (yeah, maybe a bit too much) */ */ */
    */
    // The actual value comes.. /*
    // very soon, these are just checks that */
    // multi-line comments don't trigger in line comments /*
/* Unfortunately, this comment won't get closed :(
\"THE VALUE (which is invalid)\"
"
        ),
        Err(RonErr {
            code: Error::UnclosedBlockComment,
            span: Span {
                start: Position { line: 7, col: 3 },
                end: Position { line: 9, col: 1 }
            }
        })
    );
}

#[test]
fn test_unexpected_byte() {
    assert_eq!(
        from_str::<u8>("42 /q"),
        Err(RonErr {
            code: Error::UnexpectedChar('q'),
            span: Span {
                start: Position { line: 1, col: 4 },
                end: Position { line: 1, col: 6 },
            }
        })
    );
}

#[test]
fn test_comment_after_literal_in_newtype_lookahead() {
    #[derive(Debug, PartialEq, serde::Deserialize)]
    enum Cmd {
        Echo(Msg),
    }

    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(untagged)]
    enum Msg {
        One(ron::Value),
        Many(Vec<ron::Value>),
    }

    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Cmd(Cmd),
    }

    // A comment directly after a literal must not be scanned as code
    for (ron, inner) in [
        ("\"hi\"", ron::Value::String(String::from("hi"))),
        ("'h'", ron::Value::Char('h')),
        ("b\"hi\"", ron::Value::Bytes(b"hi".to_vec())),
    ] {
        let expected = Cmd::Echo(Msg::One(inner));

        for comment in ["", "/*,x*/", "// note\n"] {
            let ron = format!("Echo({}{})", ron, comment);

            assert_eq!(
                from_str::<Cmd>(&format!("#![enable(unwrap_variant_newtypes)] {}", ron)).as_ref(),
                Ok(&expected),
                "{}",
                ron,
            );
            assert_eq!(
                from_str::<Untagged>(&ron)
                    .map(|Untagged::Cmd(cmd)| cmd)
                    .as_ref(),
                Ok(&expected),
                "{}",
                ron,
            );
        }
    }
}
