use std::collections::{BTreeMap, HashMap};

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
    struct Unit;

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Option<Option<Unit>>,
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
                    hi: Some(Some(Unit))
                }
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &AdjacentlyTagged::B {
                ho: 24,
                a: A {
                    hi: Some(Some(Unit))
                }
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
            a: A {
                hi: Some(Some(Unit))
            }
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
    #[serde(untagged)]
    enum Untagged {
        Unit,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Option<Option<Untagged>>,
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
                        hi: Some(Some(Untagged::Unit))
                    }
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
                    a: A { hi: Some(Some(Untagged::Unit)) }
                }
            },
            PrettyConfig::default().extensions(Extensions::IMPLICIT_SOME)
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: FlattenedStruct { ho: 24, a: B { a: A { hi: Some(Some(Unit)) } } } != FlattenedStruct { ho: 24, a: B { a: A { hi: None } } }"))))
    );
}

#[test]
fn implicit_some_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Option<Option<[u8; 0]>>,
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
                    a: A { hi: Some(Some([])) }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("an empty array"),
                found: String::from("a unit value")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: B {
                    a: A { hi: Some(Some([])) }
                }
            },
            PrettyConfig::default().extensions(Extensions::IMPLICIT_SOME)
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: C { ho: 24, a: B { a: A { hi: Some(Some([])) } } } != C { ho: 24, a: B { a: A { hi: None } } }"))))
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
    struct OneTuple(i32, #[serde(skip)] ());

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        B { ho: i32, a: Option<OneTuple> },
    }

    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: Some(OneTuple(42, ()))
            },
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &Untagged::B {
                ho: 24,
                a: Some(OneTuple(42, ()))
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
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>("(tag: B, content: (ho: 24, a: (hi: OneTuple(42))))"),
        Ok(AdjacentlyTagged::B {
            ho: 24,
            a: A {
                hi: OneEnum::OneTuple(42, ())
            }
        }),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>("(content: (ho: 24, a: (hi: OneTuple(42))), tag: B)"),
        Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("tuple variant"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 1, col: 50 }
        })
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
fn raw_value_inside_internally_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Box<ron::value::RawValue>,
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
                    hi: ron::value::RawValue::from_boxed_ron(String::from("42").into_boxed_str())
                        .unwrap(),
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any valid RON-value-string"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 7, col: 2 }
        }))
    );
}

#[test]
fn raw_value_inside_adjacently_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Box<ron::value::RawValue>,
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
                    hi: ron::value::RawValue::from_boxed_ron(String::from("42").into_boxed_str())
                        .unwrap(),
                }
            },
            PrettyConfig::default()
        ),
        // adds an extra space
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: B { ho: 24, a: A { hi: RawValue(42) } } != B { ho: 24, a: A { hi: RawValue( 42) } }"))))
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>("(tag: B, content: (ho: 24, a: (hi:42)))"),
        Ok(AdjacentlyTagged::B {
            ho: 24,
            a: A {
                hi: ron::value::RawValue::from_boxed_ron(String::from("42").into_boxed_str())
                    .unwrap(),
            }
        }),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>("(content: (ho: 24, a: (hi:42)), tag: B)"),
        Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any valid RON-value-string"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 1, col: 39 }
        })
    );
}

#[test]
fn raw_value_inside_untagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Box<ron::value::RawValue>,
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
                    hi: ron::value::RawValue::from_boxed_ron(String::from("42").into_boxed_str())
                        .unwrap(),
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
fn raw_value_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Box<ron::value::RawValue>,
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
                        hi: ron::value::RawValue::from_boxed_ron(
                            String::from("42").into_boxed_str()
                        )
                        .unwrap(),
                    }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any valid RON-value-string"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
}

#[test]
fn raw_value_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Box<ron::value::RawValue>,
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
                        hi: ron::value::RawValue::from_boxed_ron(
                            String::from("42").into_boxed_str()
                        )
                        .unwrap(),
                    }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("any valid RON-value-string"),
                found: String::from("the unsigned integer `42`")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
}

#[test]
fn unit_like_zero_length_inside_internally_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: [i32; 0],
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
                a: A { hi: [] }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("an empty array"),
                found: String::from("a unit value")
            },
            position: Position { line: 7, col: 2 }
        }))
    );
}

#[test]
fn unit_like_zero_length_inside_adjacently_tagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct TupleStruct();

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: TupleStruct,
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
                a: A { hi: TupleStruct() }
            },
            PrettyConfig::default()
        ),
        Ok(()),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>("(tag: B, content: (ho: 24, a: (hi: ())))"),
        Ok(AdjacentlyTagged::B {
            ho: 24,
            a: A { hi: TupleStruct() }
        }),
    );
    assert_eq!(
        ron::from_str::<AdjacentlyTagged>("(content: (ho: 24, a: (hi: ())), tag: B)"),
        Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("tuple struct TupleStruct"),
                found: String::from("a unit value")
            },
            position: Position { line: 1, col: 40 }
        })
    );
}

#[test]
fn unit_like_zero_length_inside_untagged() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Struct {}

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Struct,
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
                a: A { hi: Struct {} }
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
fn unit_like_zero_length_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum Enum {
        Tuple(),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Enum,
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
                    a: A { hi: Enum::Tuple() }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("tuple variant"),
                found: String::from("a unit value")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
}

#[test]
fn unit_like_zero_length_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum Enum {
        Struct {},
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct A {
        hi: Enum,
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
                        hi: Enum::Struct {}
                    }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("struct variant"),
                found: String::from("a unit value")
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
fn duplicate_key_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: [(String::from("ho"), 42)].into_iter().collect(),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::DuplicateStructField {
                field: "ho",
                outer: None
            },
            position: Position { line: 3, col: 9 }
        }))
    );
}

#[test]
fn duplicate_key_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct C {
        ho: i32,
        hi: bool,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct B {
        #[serde(flatten)]
        inner: C,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            inner: B,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                inner: B {
                    inner: C { ho: 42, hi: false }
                }
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::DuplicateStructField {
                field: "ho",
                outer: Some(String::from("A"))
            },
            position: Position { line: 3, col: 9 }
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
fn more_than_one_flatten_map_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Deep {
        #[serde(flatten)]
        other: HashMap<String, bool>,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Inner {
        #[serde(flatten)]
        deep: Deep,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        hi: Inner,
        #[serde(flatten)]
        other: HashMap<String, bool>,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                hi: Inner {
                    deep: Deep {
                        other: HashMap::new(),
                    },
                },
                other: [(String::from("42"), true)].into_iter().collect(),
            },
            PrettyConfig::default()
        ),
        // both maps collect all the unknown keys
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: FlattenedStruct { ho: 24, hi: Inner { deep: Deep { other: {} } }, other: {\"42\": true} } != FlattenedStruct { ho: 24, hi: Inner { deep: Deep { other: {\"42\": true} } }, other: {\"42\": true} }"))))
    );
}

#[test]
fn more_than_one_flatten_map_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Deep {
        #[serde(flatten)]
        other: HashMap<String, bool>,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Inner {
        #[serde(flatten)]
        deep: Deep,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            hi: Inner,
            #[serde(flatten)]
            other: HashMap<String, bool>,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                hi: Inner {
                    deep: Deep {
                        other: [(String::from("24"), false)].into_iter().collect(),
                    },
                },
                other: HashMap::new(),
            },
            PrettyConfig::default()
        ),
        // both maps collect all the unknown keys
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: A { ho: 24, hi: Inner { deep: Deep { other: {\"24\": false} } }, other: {} } != A { ho: 24, hi: Inner { deep: Deep { other: {\"24\": false} } }, other: {\"24\": false} }"))))
    );
}

#[test]
fn flatten_struct_beside_map_inside_flatten_struct() {
    // The non-flattened struct must contain some flattened fields
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Flattened {
        hi: i32,
        #[serde(flatten)]
        flat: (),
    }

    // The non-flattened struct must be behind a level of flatten
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Inner {
        flat: Flattened,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
        #[serde(flatten)]
        inner: Inner,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: HashMap::new(),
                inner: Inner {
                    flat: Flattened { hi: 42, flat: () },
                },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a map")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
}

#[test]
fn flatten_struct_beside_map_inside_flatten_struct_variant() {
    // The non-flattened struct must contain some flattened fields
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Flattened {
        hi: i32,
        #[serde(flatten)]
        flat: (),
    }

    // The non-flattened struct must be behind a level of flatten
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Inner {
        flat: Flattened,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<String, i32>,
            #[serde(flatten)]
            inner: Inner,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: HashMap::new(),
                inner: Inner {
                    flat: Flattened { hi: 42, flat: () },
                },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a map")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
}

#[test]
fn untagged_flatten_struct_variant_beside_map_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Flattened {
        Struct {
            hi: i32,
            #[serde(flatten)]
            flat: (),
        },
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
        #[serde(flatten)]
        flat: Flattened,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Struct { hi: 42, flat: () },
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: FlattenedStruct { ho: 24, other: {}, flat: Struct { hi: 42, flat: () } } != FlattenedStruct { ho: 24, other: {\"hi\": 42}, flat: Struct { hi: 42, flat: () } }"))))
    );
}

#[test]
fn untagged_flatten_struct_variant_beside_map_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Flattened {
        Struct {
            hi: i32,
            #[serde(flatten)]
            flat: (),
        },
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<String, i32>,
            #[serde(flatten)]
            flat: Flattened,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Struct { hi: 42, flat: () },
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: A { ho: 24, other: {}, flat: Struct { hi: 42, flat: () } } != A { ho: 24, other: {\"hi\": 42}, flat: Struct { hi: 42, flat: () } }"))))
    );
}

#[test]
fn externally_tagged_newtype_variant_beside_map_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum Flattened {
        Newtype(()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<String, i32>,
            #[serde(flatten)]
            flat: Flattened,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Newtype(()),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a unit value")
            },
            position: Position { line: 4, col: 1 }
        }))
    );
}

#[test]
fn externally_tagged_struct_variant_beside_map_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum Flattened {
        Struct { hi: i32 },
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
        #[serde(flatten)]
        flat: Flattened,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Struct { hi: 42 },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a map")
            },
            position: Position { line: 6, col: 1 }
        }))
    );
}

#[test]
fn externally_tagged_tuple_variant_beside_map_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum Flattened {
        Tuple(i32, bool),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<String, i32>,
            #[serde(flatten)]
            flat: Flattened,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Tuple(42, true),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("a sequence")
            },
            position: Position { line: 7, col: 1 }
        }))
    );
}

#[test]
fn internally_tagged_unit_variant_beside_map_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum Flattened {
        Unit,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
        #[serde(flatten)]
        flat: Flattened,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Unit,
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Unit\"")
            },
            position: Position { line: 4, col: 1 }
        }))
    );
}

#[test]
fn internally_tagged_newtype_variant_beside_map_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum Flattened {
        Newtype(()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<String, i32>,
            #[serde(flatten)]
            flat: Flattened,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Newtype(()),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Newtype\"")
            },
            position: Position { line: 4, col: 1 }
        }))
    );
}

#[test]
fn internally_tagged_struct_variant_beside_map_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum Flattened {
        Struct { hi: i32 },
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
        #[serde(flatten)]
        flat: Flattened,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Struct { hi: 42 },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Struct\"")
            },
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn adjacently_tagged_unit_variant_beside_map_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum Flattened {
        Unit,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
        #[serde(flatten)]
        flat: Flattened,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Unit,
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Unit\"")
            },
            position: Position { line: 4, col: 1 }
        }))
    );
}

#[test]
fn adjacently_tagged_newtype_variant_beside_map_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum Flattened {
        Newtype(()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<String, i32>,
            #[serde(flatten)]
            flat: Flattened,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Newtype(()),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Newtype\"")
            },
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn adjacently_tagged_struct_variant_beside_map_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum Flattened {
        Struct { hi: i32 },
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
        #[serde(flatten)]
        flat: Flattened,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Struct { hi: 42 },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Struct\"")
            },
            position: Position { line: 7, col: 1 }
        }))
    );
}

#[test]
fn adjacently_tagged_tuple_variant_beside_map_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag", content = "content")]
    enum Flattened {
        Tuple(i32, bool),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<String, i32>,
            #[serde(flatten)]
            flat: Flattened,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened::Tuple(42, true),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Tuple\"")
            },
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn tagged_struct_beside_map_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    struct Flattened {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        other: HashMap<String, i32>,
        #[serde(flatten)]
        flat: Flattened,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened { hi: 42 },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Flattened\"")
            },
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn tagged_struct_beside_map_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    struct Flattened {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        A {
            ho: i32,
            #[serde(flatten)]
            other: HashMap<String, i32>,
            #[serde(flatten)]
            flat: Flattened,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::A {
                ho: 24,
                other: HashMap::new(),
                flat: Flattened { hi: 42 },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("i32"),
                found: String::from("the string \"Flattened\"")
            },
            position: Position { line: 5, col: 1 }
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

#[test]
fn unit_inside_untagged_newtype_variant_inside_internally_tagged_newtype_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Newtype(()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        Newtype(Untagged),
    }

    assert_eq!(
        check_roundtrip(
            &InternallyTagged::Newtype(Untagged::Newtype(())),
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 3, col: 2 }
        }))
    );
}

#[test]
fn unit_inside_untagged_newtype_variant_inside_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Newtype(()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: Untagged,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: Untagged::Newtype(()),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 3, col: 1 }
        }))
    );
}

#[test]
fn unit_struct_inside_untagged_newtype_variant_inside_internally_tagged_newtype_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Unit;

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Newtype(Unit),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        Newtype(Untagged),
    }

    assert_eq!(
        check_roundtrip(
            &InternallyTagged::Newtype(Untagged::Newtype(Unit)),
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 3, col: 2 }
        }))
    );
}

#[test]
fn untagged_unit_variant_inside_internally_tagged_newtype_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Unit,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum FlattenedStructVariant {
        C {
            ho: i32,
            #[serde(flatten)]
            a: Untagged,
        },
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStructVariant::C {
                ho: 24,
                a: Untagged::Unit,
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 3, col: 1 }
        }))
    );
}

#[test]
fn untagged_unit_variant_inside_flatten_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum Enum {
        Unit,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Unit,
        Newtype(Enum),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        Newtype(Untagged),
    }

    assert_eq!(
        check_roundtrip(
            &InternallyTagged::Newtype(Untagged::Newtype(Enum::Unit)),
            PrettyConfig::default()
        ),
        Ok(())
    );
    assert_eq!(
        check_roundtrip(
            &InternallyTagged::Newtype(Untagged::Unit),
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 3, col: 2 }
        }))
    );
}

#[test]
fn unit_inside_internally_tagged_newtype_variant_inside_multi_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct AnotherFlattenedStruct {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        Newtype(()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: InternallyTagged,
        #[serde(flatten)]
        b: AnotherFlattenedStruct,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: InternallyTagged::Newtype(()),
                b: AnotherFlattenedStruct { hi: 42 },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("unit"),
                found: String::from("a map")
            },
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn untagged_unit_variant_inside_internally_tagged_newtype_variant_inside_multi_flatten_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Unit,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct AnotherFlattenedStruct {
        hi: i32,
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        Newtype(Untagged),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct FlattenedStruct {
        ho: i32,
        #[serde(flatten)]
        a: InternallyTagged,
        #[serde(flatten)]
        b: AnotherFlattenedStruct,
    }

    assert_eq!(
        check_roundtrip(
            &FlattenedStruct {
                ho: 24,
                a: InternallyTagged::Newtype(Untagged::Unit),
                b: AnotherFlattenedStruct { hi: 42 },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum Untagged"
            )),
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn flattened_externally_tagged_newtype_variant_beside_flattened_intenally_tagged_enum() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum ExternallyTagged {
        Newtype(()),
        Other(()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        Newtype(ExternallyTagged),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Flattened {
        #[serde(flatten)]
        a: InternallyTagged,
        #[serde(flatten)]
        b: ExternallyTagged,
    }

    assert_eq!(
        check_roundtrip(
            &Flattened {
                a: InternallyTagged::Newtype(ExternallyTagged::Other(())),
                b: ExternallyTagged::Newtype(()),
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("map with a single key"),
                found: String::from("a map"),
            },
            position: Position { line: 5, col: 1 }
        }))
    );
}

#[test]
fn flattened_externally_tagged_struct_variant_beside_flattened_intenally_tagged_enum() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum ExternallyTagged {
        Struct { a: i32 },
        Other(()),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(tag = "tag")]
    enum InternallyTagged {
        Newtype(ExternallyTagged),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Flattened {
        #[serde(flatten)]
        a: InternallyTagged,
        #[serde(flatten)]
        b: ExternallyTagged,
    }

    assert_eq!(
        check_roundtrip(
            &Flattened {
                a: InternallyTagged::Newtype(ExternallyTagged::Other(())),
                b: ExternallyTagged::Struct { a: 42 },
            },
            PrettyConfig::default()
        ),
        Err(Err(SpannedError {
            code: Error::InvalidValueForType {
                expected: String::from("map with a single key"),
                found: String::from("a map"),
            },
            position: Position { line: 7, col: 1 }
        }))
    );
}

#[test]
fn flattened_map_inside_option_beside_flattened_struct_variant() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Struct { a: i32 },
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Flattened {
        #[serde(flatten)]
        a: Untagged,
        #[serde(flatten)]
        b: Option<BTreeMap<String, i32>>,
    }

    assert_eq!(
        check_roundtrip(
            &Flattened {
                a: Untagged::Struct {
                    a: 42,
                },
                b: Some([(String::from("b"), 24)].into_iter().collect()),
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: Flattened { a: Struct { a: 42 }, b: Some({\"b\": 24}) } != Flattened { a: Struct { a: 42 }, b: Some({\"a\": 42, \"b\": 24}) }"))))
    );
}

#[test]
fn flattened_untagged_struct_beside_flattened_untagged_struct() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Untagged {
        Struct { a: i32 },
        Other { b: i32 },
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Flattened {
        #[serde(flatten)]
        a: Untagged,
        #[serde(flatten)]
        b: Untagged,
    }

    assert_eq!(
        check_roundtrip(
            &Flattened {
                a: Untagged::Struct {
                    a: 42,
                },
                b: Untagged::Other {
                    b: 24,
                },
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: Flattened { a: Struct { a: 42 }, b: Other { b: 24 } } != Flattened { a: Struct { a: 42 }, b: Struct { a: 42 } }"))))
    );
}

#[test]
fn flattened_field_inside_flattened_struct_alongside_map() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Units {
        a: i32,
        #[serde(flatten)]
        b: (),
    }

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Flattened {
        #[serde(flatten)]
        a: Units,
        #[serde(flatten)]
        b: BTreeMap<String, i32>,
    }

    assert_eq!(
        check_roundtrip(
            &Flattened {
                a: Units {
                    a: 42,
                    b: (),
                },
                b: [(String::from("c"), 24)].into_iter().collect(),
            },
            PrettyConfig::default()
        ),
        Err(Ok(Error::Message(String::from("ROUNDTRIP error: Flattened { a: Units { a: 42, b: () }, b: {\"c\": 24} } != Flattened { a: Units { a: 42, b: () }, b: {\"a\": 42, \"c\": 24} }"))))
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
