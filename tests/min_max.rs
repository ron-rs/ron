use ron::{de::*, ser::*};

#[test]
fn test_i32_min() {
    assert_eq!(
        std::i32::MIN,
        from_str(&to_string(&std::i32::MIN).unwrap()).unwrap()
    );
}

#[test]
fn test_i32_max() {
    assert_eq!(
        std::i32::MAX,
        from_str(&to_string(&std::i32::MAX).unwrap()).unwrap()
    );
}

#[test]
fn test_i64_min() {
    assert_eq!(
        std::i64::MIN,
        from_str(&to_string(&std::i64::MIN).unwrap()).unwrap()
    );
}

#[test]
fn test_i64_max() {
    assert_eq!(
        std::i64::MAX,
        from_str(&to_string(&std::i64::MAX).unwrap()).unwrap()
    );
}
