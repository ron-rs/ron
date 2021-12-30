use ron::de::from_str;
use ron::error::{Error, ErrorCode, Position};

#[test]
fn test_hex() {
    assert_eq!(from_str("0x507"), Ok(0x507));
    assert_eq!(from_str("0x1A5"), Ok(0x1A5));
    assert_eq!(from_str("0x53C537"), Ok(0x53C537));

    assert_eq!(
        from_str::<u8>("0x"),
        Err(Error {
            code: ErrorCode::ExpectedInteger,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0x_1"),
        Err(Error {
            code: ErrorCode::UnderscoreAtBeginning,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0xFFF"),
        Err(Error {
            code: ErrorCode::IntegerOutOfBounds,
            position: Position { line: 1, col: 6 },
        })
    );
}

#[test]
fn test_bin() {
    assert_eq!(from_str("0b101"), Ok(0b101));
    assert_eq!(from_str("0b001"), Ok(0b001));
    assert_eq!(from_str("0b100100"), Ok(0b100100));

    assert_eq!(
        from_str::<u8>("0b"),
        Err(Error {
            code: ErrorCode::ExpectedInteger,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0b_1"),
        Err(Error {
            code: ErrorCode::UnderscoreAtBeginning,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0b111111111"),
        Err(Error {
            code: ErrorCode::IntegerOutOfBounds,
            position: Position { line: 1, col: 12 },
        })
    );
}

#[test]
fn test_oct() {
    assert_eq!(from_str("0o1461"), Ok(0o1461));
    assert_eq!(from_str("0o051"), Ok(0o051));
    assert_eq!(from_str("0o150700"), Ok(0o150700));

    assert_eq!(
        from_str::<u8>("0o"),
        Err(Error {
            code: ErrorCode::ExpectedInteger,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0o_1"),
        Err(Error {
            code: ErrorCode::UnderscoreAtBeginning,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0o77777"),
        Err(Error {
            code: ErrorCode::IntegerOutOfBounds,
            position: Position { line: 1, col: 8 },
        })
    );
}

#[test]
fn test_dec() {
    assert_eq!(from_str("1461"), Ok(1461));
    assert_eq!(from_str("51"), Ok(51));
    assert_eq!(from_str("150700"), Ok(150700));

    assert_eq!(
        from_str::<i8>("-_1"),
        Err(Error {
            code: ErrorCode::UnderscoreAtBeginning,
            position: Position { line: 1, col: 2 },
        })
    );
    assert_eq!(
        from_str::<u8>("256"),
        Err(Error {
            code: ErrorCode::IntegerOutOfBounds,
            position: Position { line: 1, col: 4 },
        })
    );
}
