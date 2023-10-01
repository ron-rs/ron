use ron::{error::Position, error::SpannedError, ser::PrettyConfig, Error};
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
        Ok(()),
    );
    assert_eq!(
        ron::from_str::<InternallyTagged>(
            "InternallyTagged(tag: B, content: B(ho: 24, a: A(hi: 42)))"
        ),
        Ok(InternallyTagged::B {
            ho: 24,
            a: A { hi: 42 }
        }),
    );
    assert_eq!(
        ron::from_str::<InternallyTagged>(
            "InternallyTagged(content: B(ho: 24, a: A(hi: 42)), tag: B)"
        ),
        Err(SpannedError {
            code: Error::MissingStructField {
                field: "ho",
                outer: Some(String::from("InternallyTagged"))
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
            code: Error::Message(String::from(
                "data did not match any variant of untagged enum InternallyTagged"
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
        }
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

fn check_roundtrip<T: PartialEq + std::fmt::Debug + Serialize + serde::de::DeserializeOwned>(
    val: &T,
    config: PrettyConfig,
) -> Result<(), Result<Error, SpannedError>> {
    let ron = ron::ser::to_string_pretty(val, config).map_err(|err| Ok(err))?;
    println!("{ron}");
    let de = ron::from_str(&ron).map_err(|err| Err(err))?;
    assert_eq!(val, &de);
    Ok(())
}
