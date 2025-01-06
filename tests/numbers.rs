use ron::{
    de::from_str,
    error::{Error, Position, SpannedError},
    ser::{to_string_pretty, PrettyConfig},
};

#[test]
fn test_hex() {
    assert_eq!(from_str("0x507"), Ok(0x507));
    assert_eq!(from_str("0x1A5"), Ok(0x1A5));
    assert_eq!(from_str("0x53C537"), Ok(0x53C537));

    assert_eq!(
        from_str::<u8>("0x"),
        Err(SpannedError {
            code: Error::ExpectedInteger,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0x_1"),
        Err(SpannedError {
            code: Error::UnderscoreAtBeginning,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0xFFF"),
        Err(SpannedError {
            code: Error::IntegerOutOfBounds,
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
        Err(SpannedError {
            code: Error::ExpectedInteger,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0b_1"),
        Err(SpannedError {
            code: Error::UnderscoreAtBeginning,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0b111111111"),
        Err(SpannedError {
            code: Error::IntegerOutOfBounds,
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
        Err(SpannedError {
            code: Error::ExpectedInteger,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0o_1"),
        Err(SpannedError {
            code: Error::UnderscoreAtBeginning,
            position: Position { line: 1, col: 3 },
        })
    );
    assert_eq!(
        from_str::<u8>("0o77777"),
        Err(SpannedError {
            code: Error::IntegerOutOfBounds,
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
        Err(SpannedError {
            code: Error::UnderscoreAtBeginning,
            position: Position { line: 1, col: 2 },
        })
    );
    assert_eq!(
        from_str::<u8>("256"),
        Err(SpannedError {
            code: Error::IntegerOutOfBounds,
            position: Position { line: 1, col: 4 },
        })
    );
}

#[test]
fn test_hex_serialization() {
    let config = PrettyConfig::new()
        .hex_as_raw(true);
    
    // Test valid hex without quotes
    let val = "0x1234";
    let serialized = to_string_pretty(&val, config.clone()).unwrap();
    assert_eq!(serialized, "0x1234");
    
    // Test uppercase hex
    let val = "0X1A5B";
    let serialized = to_string_pretty(&val, config.clone()).unwrap();
    assert_eq!(serialized, "0X1A5B");
    
    // Test that normal strings are still escaped
    let val = "normal string";
    let serialized = to_string_pretty(&val, config.clone()).unwrap();
    assert_eq!(serialized, "\"normal string\"");
    
    // Test that invalid hex is treated as a normal string
    let val = "0xGGG";
    let serialized = to_string_pretty(&val, config.clone()).unwrap();
    assert_eq!(serialized, "\"0xGGG\"");
    
    // Test with hex_as_raw disabled
    let config = PrettyConfig::new().hex_as_raw(false);
    let val = "0x1234";
    let serialized = to_string_pretty(&val, config).unwrap();
    assert_eq!(serialized, "\"0x1234\"");
}
