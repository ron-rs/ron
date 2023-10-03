use std::collections::HashMap;

use ron::{error::Position, error::SpannedError, extensions::Extensions, ser::PrettyConfig, Error};
use serde::{Deserialize, Serialize};

#[test]
fn struct_names_inside_internally_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A { hi: 42 }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A { hi: 42 }
            },
            PrettyConfig::default().struct_names(true)
        ),
        Err(Err(SpannedError {
            code: Error::MissingStructField {
                field: "hi",
                outer: None
            },
            position: Position { line: 7, col: 2 }
        })),
    );
}

#[test]
fn struct_names_inside_adjacently_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum AdjacentlyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A { hi: 42 }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A { hi: 42 }
            },
            PrettyConfig::default().struct_names(true)
        ),
        Ok(()),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>(
            "AdjacentlyTagged(tag: B, content: B(ho: 24, a: A(hi: 42)))"
        ),
        Ok(AdjacentlyTagged::B {
            ho: 24,
            a: A { hi: 42 }
        }),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>(
            "AdjacentlyTagged(content: B(ho: 24, a: A(hi: 42)), tag: B)"
        ),
        Err(SpannedError {
            code: Error::MissingStructField {
                field: "ho",
                outer: Some(String::from("AdjacentlyTagged"))
            },
            position: Position { line: 1, col: 58 }
        }),
    );
}

#[test]
fn struct_names_inside_untagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A { hi: 42 }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A { hi: 42 }
            },
            PrettyConfig::default().struct_names(true)
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 6, col: 2 }
        })),
    );
}

#[test]
fn struct_names_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: B,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B { a: A { hi: 42 } }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B { a: A { hi: 42 } }
            },
            PrettyConfig::default().struct_names(true)
        ),
        Err(Err(SpannedError {
            code: Error::MissingStructField {
                field: "hi",
                outer: None
            },
            position: Position { line: 6, col: 1 }
        })),
    );
}

#[test]
fn struct_names_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        C {
            ho: i32,
            #[serde(flatten)]
            a: B,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B { a: A { hi: 42 } }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B { a: A { hi: 42 } }
            },
            PrettyConfig::default().struct_names(true)
        ),
        Err(Err(SpannedError {
            code: Error::MissingStructField {
                field: "hi",
                outer: Some(String::from("C"))
            },
            position: Position { line: 6, col: 1 }
        })),
    );
}

#[test]
fn implicit_some_inside_internally_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Option<Option<()>>,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A { hi: Some(Some(())) }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A { hi: Some(Some(())) }
            },
            PrettyConfig::default().extensions(Extensions::IMPLICIT_SOME)
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: B { ho: 24, a: A { hi: Some(Some(())) } } != B { ho: 24, a: A { hi: None } }"))))
    );
}

#[test]
fn implicit_some_inside_adjacently_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Option<Option<()>>,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum AdjacentlyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A { hi: Some(Some(())) }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A { hi: Some(Some(())) }
            },
            PrettyConfig::default().extensions(Extensions::IMPLICIT_SOME)
        ),
        Ok(()),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>(
            "#![enable(implicit_some)] (tag: B, content: (ho: 24, a: A(hi: ())))"
        ),
        Ok(AdjacentlyTagged::B {
            ho: 24,
            a: A { hi: Some(Some(())) }
        }),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>(
            "#![enable(implicit_some)] (content: (ho: 24, a: A(hi: ())), tag: B)"
        ),
        Ok(AdjacentlyTagged::B {
            ho: 24,
            a: A { hi: None } // THIS IS WRONG
        }),
    );
}

#[test]
fn implicit_some_inside_untagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Option<Option<()>>,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A { hi: Some(Some(())) }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A { hi: Some(Some(())) }
            },
            PrettyConfig::default().extensions(Extensions::IMPLICIT_SOME)
        ),
        Err(Ok(Error::Message(String::from(
            "ROUNDTRIP error: B { ho: 24, a: A { hi: Some(Some(())) } } != B { ho: 24, a: A { hi: None } }"
        )))),
    );
}

#[test]
fn implicit_some_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Option<Option<()>>,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: B,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B {
                    a: A { hi: Some(Some(())) }
                }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B {
                    a: A { hi: Some(Some(())) }
                }
            },
            PrettyConfig::default().extensions(Extensions::IMPLICIT_SOME)
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: FlattenedStruct { ho: 24, a: B { a: A { hi: Some(Some(())) } } } != FlattenedStruct { ho: 24, a: B { a: A { hi: None } } }"))))
    );
}

#[test]
fn implicit_some_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Option<Option<()>>,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        C {
            ho: i32,
            #[serde(flatten)]
            a: B,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B {
                    a: A { hi: Some(Some(())) }
                }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B {
                    a: A { hi: Some(Some(())) }
                }
            },
            PrettyConfig::default().extensions(Extensions::IMPLICIT_SOME)
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: C { ho: 24, a: B { a: A { hi: Some(Some(())) } } } != C { ho: 24, a: B { a: A { hi: None } } }"))))
    );
}

#[test]
fn newtype_inside_internally_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A(i32);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        B { ho: i32, a: A },
    }

    // NOTE:
    // 1. ron is correctly collected into Content, newtype is a one-seq here
    // 2. newtype asks ContentDeserializer for newtype
    // 3. ContentDeserializer forwards any value to visit_newtype_struct
    //    https://github.com/serde-rs/serde/blob/8c4aad3a59515f7b779f764d5e16d6bae297ab7f/serde/src/private/de.rs#L1347-L1359

    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B { ho: 24, a: A(42) },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a sequence")
            },
            position: Position { line: 5, col: 2 }
        }))
    );
}

#[test]
fn newtype_inside_adjacently_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A(i32);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum AdjacentlyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B { ho: 24, a: A(42) },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>("(tag: B, content: (ho: 24, a: (42)))"),
        Ok(AdjacentlyTagged::B { ho: 24, a: A(42) }),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>("(content: (ho: 24, a: (42)), tag: B)"),
        Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a sequence")
            },
            position: Position { line: 1, col: 36 }
        })
    );
}

#[test]
fn newtype_inside_untagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A(i32);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(&Untagged::B { ho: 24, a: A(42) }, PrettyConfig::default()),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 4, col: 2 }
        }))
    );
}

#[test]
fn newtype_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A(i32);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: B,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B { a: A(42) }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a sequence")
            },
            position: Position { line: 4, col: 1 }
        }))
    );
}

#[test]
fn newtype_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A(i32);

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        C {
            ho: i32,
            #[serde(flatten)]
            a: B,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B { a: A(42) }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a sequence")
            },
            position: Position { line: 4, col: 1 }
        }))
    );
}

#[test]
fn one_tuple_inside_unwrapped_newtype_variant_inside_internally_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum A {
        Newtype((i32,)),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A::Newtype((42,))
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A::Newtype((42,))
            },
            PrettyConfig::default().extensions(Extensions::UNWRAP_VARIANT_NEWTYPES)
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("a tuple of size 1"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 6, col: 2 }
        }))
    );
}

#[test]
fn one_tuple_inside_unwrapped_newtype_variant_inside_adjacently_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum A {
        Newtype([i32; 1]),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum AdjacentlyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A::Newtype([42])
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A::Newtype([42])
            },
            PrettyConfig::default().extensions(Extensions::UNWRAP_VARIANT_NEWTYPES)
        ),
        Ok(())
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>(
            "#![enable(unwrap_variant_newtypes)] (tag: B, content: (ho: 24, a: Newtype(42)))"
        ),
        Ok(AdjacentlyTagged::B {
            ho: 24,
            a: A::Newtype([42])
        }),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>(
            "#![enable(unwrap_variant_newtypes)] (content: (ho: 24, a: Newtype(42)), tag: B)"
        ),
        Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("an array of length 1"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 1, col: 79 }
        })
    );
}

#[test]
fn one_tuple_inside_unwrapped_newtype_variant_inside_untagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum A {
        Newtype((i32,)),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A::Newtype((42,))
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A::Newtype((42,))
            },
            PrettyConfig::default().extensions(Extensions::UNWRAP_VARIANT_NEWTYPES)
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 5, col: 2 }
        }))
    );
}

#[test]
fn one_tuple_inside_unwrapped_newtype_variant_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum A {
        Newtype([i32; 1]),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: B,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B {
                    a: A::Newtype([42])
                }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B {
                    a: A::Newtype([42])
                }
            },
            PrettyConfig::default().extensions(Extensions::UNWRAP_VARIANT_NEWTYPES)
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("an array of length 1"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn one_tuple_inside_unwrapped_newtype_variant_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum A {
        Newtype((i32,)),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        C {
            ho: i32,
            #[serde(flatten)]
            a: B,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B {
                    a: A::Newtype((42,))
                }
            },
            PrettyConfig::default()
        ),
        Ok(()),
    );
    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B {
                    a: A::Newtype((42,))
                }
            },
            PrettyConfig::default().extensions(Extensions::UNWRAP_VARIANT_NEWTYPES)
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("a tuple of size 1"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn one_tuple_variant_inside_internally_tagged() {
    // A tuple variant with just one element that is not a newtype variant
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum OneEnum {
        OneTuple(i32, #[serde(skip)] ()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: OneEnum,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A {
                    hi: OneEnum::OneTuple(42, ())
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("tuple variant"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 7, col: 2 }
        }))
    );
}

#[test]
fn one_tuple_variant_inside_adjacently_tagged() {
    // A tuple variant with just one element that is not a newtype variant
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum OneEnum {
        OneTuple(i32, #[serde(skip)] ()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: OneEnum,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum AdjacentlyTagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A {
                    hi: OneEnum::OneTuple(42, ())
                }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
}

#[test]
fn one_tuple_variant_inside_untagged() {
    // A tuple variant with just one element that is not a newtype variant
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum OneEnum {
        OneTuple(i32, #[serde(skip)] ()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: OneEnum,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        B { ho: i32, a: A },
    }

    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A {
                    hi: OneEnum::OneTuple(42, ())
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 6, col: 2 }
        }))
    );
}

#[test]
fn one_tuple_variant_inside_flatten_struct() {
    // A tuple variant with just one element that is not a newtype variant
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum OneEnum {
        OneTuple(i32, #[serde(skip)] ()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: OneEnum,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: B,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B {
                    a: A {
                        hi: OneEnum::OneTuple(42, ())
                    }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("tuple variant"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
}

#[test]
fn one_tuple_variant_inside_flatten_struct_variant() {
    // A tuple variant with just one element that is not a newtype variant
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum OneEnum {
        OneTuple(i32, #[serde(skip)] ()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: OneEnum,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        C {
            ho: i32,
            #[serde(flatten)]
            a: B,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B {
                    a: A {
                        hi: OneEnum::OneTuple(42, ())
                    }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("tuple variant"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
}

#[test]
fn i128_inside_internally_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: i128,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        B { ho: i32, a: A },
    }

    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A { hi: i128::MAX }
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("i128 is not supported"))))
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        check_roundtrip(
            &InternallyTagged::B {
                ho: 24,
                a: A { hi: i128::MAX }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any value"),
                found: format!("integer `{}` as u128", i128::MAX)
            },
            position: Position { line: 5, col: 52 }
        }))
    );
}

#[test]
fn u128_inside_adjacently_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: u128,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum AdjacentlyTagged {
        B { ho: i32, a: A },
    }

    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A { hi: u128::MAX }
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("u128 is not supported"))))
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A { hi: u128::MAX }
            },
            PrettyConfig::default()
        ),
        Ok(()),
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>(&format!(
            "(tag: B, content: (ho: 24, a: (hi: {})))",
            u128::MAX
        ),),
        Ok(AdjacentlyTagged::B {
            ho: 24,
            a: A { hi: u128::MAX }
        }),
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>(&format!(
            "(content: (ho: 24, a: (hi: {})), tag: B)",
            u128::MAX
        ),),
        Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any value"),
                found: format!("integer `{}` as u128", u128::MAX)
            },
            position: Position { line: 1, col: 67 }
        }),
    );
}

#[test]
fn i128_inside_untagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: i128,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        B { ho: i32, a: A },
    }

    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A { hi: i128::MIN }
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("i128 is not supported"))))
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: A { hi: i128::MIN }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any value"),
                found: format!("integer `{}` as i128", i128::MIN)
            },
            position: Position { line: 4, col: 53 }
        }))
    );
}

#[test]
fn u128_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: u128,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: B,
    }

    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B {
                    a: A { hi: u128::MAX }
                }
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("u128 is not supported"))))
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: B {
                    a: A { hi: u128::MAX }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any value"),
                found: format!("integer `{}` as u128", u128::MAX)
            },
            position: Position { line: 4, col: 52 }
        }))
    );
}

#[test]
fn i128_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: i128,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        a: A,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        C {
            ho: i32,
            #[serde(flatten)]
            a: B,
        },
    }

    #[cfg(not(feature = "integer128"))]
    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B {
                    a: A { hi: i128::MIN }
                }
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("i128 is not supported"))))
    );
    #[cfg(feature = "integer128")]
    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B {
                    a: A { hi: i128::MIN }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any value"),
                found: format!("integer `{}` as i128", i128::MIN)
            },
            position: Position { line: 4, col: 53 }
        }))
    );
}

#[test]
fn non_string_key_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<i32, bool>,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: [(1, true), (0, false)].into_iter().collect(),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::ExpectedString,
            position: Position { line: 3, col: 5 }
        }))
    );
}

#[test]
fn non_string_key_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<char, u8>,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: [('h', 0), ('i', 1)].into_iter().collect(),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::ExpectedString,
            position: Position { line: 3, col: 5 }
        }))
    );
}

#[test]
fn zero_length_untagged_tuple_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        A(),
    }

    assert_eq!(
        check_roundtrip(&Untagged::A(), PrettyConfig::default()),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 1, col: 3 }
        }))
    );
}

#[test]
fn zero_length_untagged_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        A {},
    }

    assert_eq!(
        check_roundtrip(&Untagged::A {}, PrettyConfig::default()),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 1, col: 3 }
        }))
    );
}

#[test]
fn unwrapped_one_element_untagged_tuple_variant() {
    // A tuple variant with just one element that is not a newtype variant
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        OneTuple(i32, #[serde(skip)] ()),
    }

    assert_eq!(
        check_roundtrip(&Untagged::OneTuple(42, ()), PrettyConfig::default()),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &Untagged::OneTuple(42, ()),
            PrettyConfig::default().extensions(Extensions::UNWRAP_VARIANT_NEWTYPES)
        ),
        Ok(())
    );
}

fn check_roundtrip<T: PartialEq + std::fmt::Debug + Serialize + serde::de::DeserializeOwned>(
    val: &T,
    config: PrettyConfig,
) -> Result<(), Result<Error, SpannedError>> {
    let ron = ron::ser::to_string_pretty(val, config).map_err(|err| Ok(err))?;
    println!("{ron}");
    let de = ron::from_str(&ron).map_err(|err| Err(err))?;
    if val == &de {
        Ok(())
    } else {
        Err(Ok(Error::Message(format!(
            "ROUNDTRIP error: {:?} != {:?}",
            val, de
        ))))
    }
}
