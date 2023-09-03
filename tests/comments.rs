use ron::de::{from_str, Error, Position, SpannedError as RonErr};

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
            position: Position { line: 1, col: 8 }
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
            position: Position { line: 9, col: 1 }
        })
    );
}

#[test]
fn test_unexpected_byte() {
    assert_eq!(
        from_str::<u8>("42 /q"),
        Err(RonErr {
            code: Error::UnexpectedChar('q'),
            position: Position { line: 1, col: 6 },
        })
    );
}
