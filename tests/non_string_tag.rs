macro_rules! test_non_tag {
    ($test_name:ident => $deserialize_method:ident($($deserialize_param:expr),*)) => {
        #[test]
        fn $test_name() {
            use serde::{Deserialize, Deserializer, de::Visitor};

            struct TagVisitor;

            impl<'de> Visitor<'de> for TagVisitor {
                type Value = Tag;

                // GRCOV_EXCL_START
                fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                    fmt.write_str("an error")
                }
                // GRCOV_EXCL_STOP
            }

            struct Tag;

            impl<'de> Deserialize<'de> for Tag {
                fn deserialize<D: Deserializer<'de>>(deserializer: D)
                    -> Result<Self, D::Error>
                {
                    deserializer.$deserialize_method($($deserialize_param,)* TagVisitor)
                }
            }

            #[derive(Debug)] // GRCOV_EXCL_LINE
            struct InternallyTagged;

            impl<'de> Deserialize<'de> for InternallyTagged {
                fn deserialize<D: Deserializer<'de>>(deserializer: D)
                    -> Result<Self, D::Error>
                {
                    deserializer.deserialize_any(serde::__private::de::TaggedContentVisitor::<
                        Tag,
                    >::new(
                        "tag",
                        "an internally tagged value",
                    )).map(|_| unreachable!())
                }
            }

            assert_eq!(
                ron::from_str::<InternallyTagged>("(tag: err)").unwrap_err().code,
                ron::Error::ExpectedString
            )
        }
    };
}

test_non_tag! { test_bool => deserialize_bool() }
test_non_tag! { test_i8 => deserialize_i8() }
test_non_tag! { test_i16 => deserialize_i16() }
test_non_tag! { test_i32 => deserialize_i32() }
test_non_tag! { test_i64 => deserialize_i64() }
#[cfg(feature = "integer128")]
test_non_tag! { test_i128 => deserialize_i128() }
test_non_tag! { test_u8 => deserialize_u8() }
test_non_tag! { test_u16 => deserialize_u16() }
test_non_tag! { test_u32 => deserialize_u32() }
test_non_tag! { test_u64 => deserialize_u64() }
#[cfg(feature = "integer128")]
test_non_tag! { test_u128 => deserialize_u128() }
test_non_tag! { test_f32 => deserialize_f32() }
test_non_tag! { test_f64 => deserialize_f64() }
test_non_tag! { test_char => deserialize_char() }
test_non_tag! { test_bytes => deserialize_bytes() }
test_non_tag! { test_byte_buf => deserialize_byte_buf() }
test_non_tag! { test_option => deserialize_option() }
test_non_tag! { test_unit => deserialize_unit() }
test_non_tag! { test_unit_struct => deserialize_unit_struct("") }
test_non_tag! { test_newtype_struct => deserialize_newtype_struct("") }
test_non_tag! { test_seq => deserialize_seq() }
test_non_tag! { test_tuple => deserialize_tuple(0) }
test_non_tag! { test_tuple_struct => deserialize_tuple_struct("", 0) }
test_non_tag! { test_map => deserialize_map() }
test_non_tag! { test_struct => deserialize_struct("", &[]) }
test_non_tag! { test_enum => deserialize_enum("", &[]) }

macro_rules! test_tag {
    ($test_name:ident => $deserialize_method:ident($($deserialize_param:expr),*)) => {
        #[test]
        fn $test_name() {
            use serde::{Deserialize, Deserializer, de::Visitor};

            struct TagVisitor;

            impl<'de> Visitor<'de> for TagVisitor {
                type Value = Tag;

                // GRCOV_EXCL_START
                fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                    fmt.write_str("a tag")
                }
                // GRCOV_EXCL_STOP

                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                    assert_eq!(v, "tag");
                    Ok(Tag)
                }
            }

            struct Tag;

            impl<'de> Deserialize<'de> for Tag {
                fn deserialize<D: Deserializer<'de>>(deserializer: D)
                    -> Result<Self, D::Error>
                {
                    deserializer.$deserialize_method($($deserialize_param,)* TagVisitor)
                }
            }

            #[derive(Debug)] // GRCOV_EXCL_LINE
            struct InternallyTagged;

            impl<'de> Deserialize<'de> for InternallyTagged {
                fn deserialize<D: Deserializer<'de>>(deserializer: D)
                    -> Result<Self, D::Error>
                {
                    deserializer.deserialize_any(serde::__private::de::TaggedContentVisitor::<
                        Tag,
                    >::new(
                        "tag",
                        "an internally tagged value",
                    )).map(|_| InternallyTagged)
                }
            }

            assert!(
                ron::from_str::<InternallyTagged>("(tag: \"tag\")").is_ok(),
            )
        }
    };
}

test_tag! { test_str => deserialize_string() }
test_tag! { test_string => deserialize_string() }
test_tag! { test_identifier => deserialize_identifier() }

test_tag! { test_any => deserialize_any() }
test_tag! { test_ignored_any => deserialize_ignored_any() }
