use ron::{
    error::{Position, SpannedError},
    Error,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct BytesStruct {
    small: Vec<u8>,
    #[serde(with = "serde_bytes")]
    large: Vec<u8>,
}

#[test]
fn v_9_deprecated_base64_bytes_support() {
    #![allow(deprecated)]

    // Requires padding of the base64 data
    assert_eq!(
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 3, 4]
        }),
        ron::from_str("BytesStruct( small:[1, 2], large:\"AQIDBA==\" )"),
    );

    // Requires no padding of the base64 data
    assert_eq!(
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 3, 4, 5, 6]
        }),
        ron::from_str("BytesStruct( small:[1, 2], large:\"AQIDBAUG\" )"),
    );

    // Invalid base64
    assert_eq!(
        Err(SpannedError {
            code: Error::Base64Error(base64::DecodeError::InvalidByte(0, b'_')),
            position: Position { line: 1, col: 40 }
        }),
        ron::from_str::<BytesStruct>("BytesStruct( small:[1, 2], large:\"_+!!\" )"),
    );

    // Invalid last base64 symbol
    assert_eq!(
        Err(SpannedError {
            code: Error::Base64Error(base64::DecodeError::InvalidLastSymbol(1, b'x')),
            position: Position { line: 1, col: 40 }
        }),
        ron::from_str::<BytesStruct>("BytesStruct( small:[1, 2], large:\"/x==\" )"),
    );

    // Missing padding
    assert_eq!(
        Err(SpannedError {
            code: Error::Base64Error(base64::DecodeError::InvalidPadding),
            position: Position { line: 1, col: 42 }
        }),
        ron::from_str::<BytesStruct>("BytesStruct( small:[1, 2], large:\"AQIDBA\" )"),
    );

    // Too much padding
    assert_eq!(
        Err(SpannedError {
            code: Error::Base64Error(base64::DecodeError::InvalidLength),
            position: Position { line: 1, col: 45 }
        }),
        ron::from_str::<BytesStruct>("BytesStruct( small:[1, 2], large:\"AQIDBA===\" )"),
    );
}

#[test]
fn rusty_byte_string() {
    assert_eq!(
        Ok(BytesStruct {
            small: vec![1, 2],
            large: vec![1, 2, 0, 4]
        }),
        ron::from_str("BytesStruct( small:[1, 2], large: b\"\\x01\\u{2}\\0\\x04\" )"),
    );

    assert_eq!(
        ron::from_str::<String>("\"Hello \\x01 \\u{2}!\"").unwrap(),
        "Hello \x01 \u{2}!",
    );
    assert_eq!(
        &*ron::from_str::<bytes::Bytes>("b\"Hello \\x01 \\u{2}!\"").unwrap(),
        b"Hello \x01 \x02!",
    );
}
