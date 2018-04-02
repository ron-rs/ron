extern crate ron;

use ron::de::Error as RonErr;
use ron::de::ParseError;
use ron::de::Position;

#[test]
fn test_simple() {
    assert_eq!(ron::de::from_str("/*
 * We got a hexadecimal number here!
 *
 */0x507"
    ), Ok(0x507));
}

#[test]
fn test_nested() {
    assert_eq!(ron::de::from_str("/*
        /* quite * some * nesting * going * on * /* here /* (yeah, maybe a bit too much) */ */ */
    */
    // The actual value comes.. /*
    // very soon, these are just checks that */
    // multi-line comments don't trigger in line comments /*
\"THE VALUE\" /* This is the value /* :) */ */
    "
    ), Ok("THE VALUE".to_owned()));
}

#[test]
fn test_unclosed() {
    assert_eq!(ron::de::from_str::<String>("/*
        /* quite * some * nesting * going * on * /* here /* (yeah, maybe a bit too much) */ */ */
    */
    // The actual value comes.. /*
    // very soon, these are just checks that */
    // multi-line comments don't trigger in line comments /*
/* Unfortunately, this comment won't get closed :(
\"THE VALUE (which is invalid)\"
"
    ), Err(RonErr::Parser(ParseError::UnclosedBlockComment, Position { col: 1, line: 9 })));
}
