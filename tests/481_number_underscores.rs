#[test]
fn de_integer_underscores() {
    assert_eq!(ron::from_str("0b10_10___101_"), Ok(0b10_10___101__u8));
    assert_eq!(
        ron::from_str::<u8>("_0b1"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("0b2"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidIntegerDigit {
                digit: '2',
                base: 2
            },
            position: ron::error::Position { line: 1, col: 3 },
        })
    );

    assert_eq!(ron::from_str("0o71_32___145_"), Ok(0o71_32___145_));
    assert_eq!(
        ron::from_str::<u8>("_0o5"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("0oA"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidIntegerDigit {
                digit: 'A',
                base: 8
            },
            position: ron::error::Position { line: 1, col: 3 },
        })
    );

    assert_eq!(ron::from_str("0xa1_fe___372_"), Ok(0xa1_fe___372_));
    assert_eq!(
        ron::from_str::<u8>("_0xF"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("0xZ"),
        Err(ron::error::SpannedError {
            code: ron::Error::ExpectedInteger,
            position: ron::error::Position { line: 1, col: 3 },
        })
    );

    assert_eq!(ron::from_str("0_6_163_810___17"), Ok(0_6_163_810___17));
    assert_eq!(
        ron::from_str::<u8>("_123"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<u8>("12a"),
        Err(ron::error::SpannedError {
            code: ron::Error::InvalidIntegerDigit {
                digit: 'a',
                base: 10
            },
            position: ron::error::Position { line: 1, col: 3 },
        })
    );
}

#[test]
fn de_float_underscores() {
    assert_eq!(ron::from_str("2_18__6_"), Ok(2_18__6__f32));
    assert_eq!(
        ron::from_str::<f32>("_286"),
        Err(ron::error::SpannedError {
            code: ron::Error::UnderscoreAtBeginning,
            position: ron::error::Position { line: 1, col: 1 },
        })
    );
    assert_eq!(
        ron::from_str::<f32>("2a86"),
        Err(ron::error::SpannedError {
            code: ron::Error::TrailingCharacters,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );

    assert_eq!(ron::from_str("2_18__6_."), Ok(2_18__6__f32));
    assert_eq!(
        ron::from_str::<f32>("2_18__6_._"),
        Err(ron::error::SpannedError {
            code: ron::Error::FloatUnderscore,
            position: ron::error::Position { line: 1, col: 10 },
        })
    );
    assert_eq!(
        ron::from_str::<f32>("2_18__6_.3__7_"),
        Ok(2_18__6_.3__7__f32)
    );

    assert_eq!(ron::from_str::<f32>(".3__7_"), Ok(0.3__7__f32));
    assert_eq!(
        ron::from_str::<f32>("._3__7_"),
        Err(ron::error::SpannedError {
            code: ron::Error::FloatUnderscore,
            position: ron::error::Position { line: 1, col: 2 },
        })
    );

    assert_eq!(
        ron::from_str::<f64>("2_18__6_.3__7_e____7_3__"),
        Ok(2_18__6_.3__7_e____7_3___f64)
    );
    assert_eq!(
        ron::from_str::<f64>("2_18__6_.3__7_e+____"),
        Err(ron::error::SpannedError {
            code: ron::Error::ExpectedFloat,
            position: ron::error::Position { line: 1, col: 21 },
        })
    );
}
