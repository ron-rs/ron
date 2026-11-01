use ron::{
    de::from_bytes,
    error::{Error, Position, Span, SpannedError},
    from_str, to_string,
    value::RawValue,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct WithRawValue {
    a: bool,
    b: Box<RawValue>,
}

#[test]
fn test_raw_value_simple() {
    let raw: &RawValue = from_str("true").unwrap();
    assert_eq!(raw.get_ron(), "true");
    let ser = to_string(raw).unwrap();
    assert_eq!(ser, "true");
}

#[test]
fn test_raw_value_inner() {
    let raw: WithRawValue = from_str("(a: false, b: [1, /* lol */ 2, 3])").unwrap();
    assert_eq!(raw.b.get_ron(), "[1, /* lol */ 2, 3]");
    let ser = to_string(&raw).unwrap();
    assert_eq!(ser, "(a:false,b:[1, /* lol */ 2, 3])");
}

#[test]
fn test_raw_value_comment() {
    let raw: WithRawValue = from_str("(a: false, b: /* yeah */ 4)").unwrap();
    assert_eq!(raw.b.get_ron(), "/* yeah */ 4");

    let raw: WithRawValue = from_str("(a: false, b: 4 /* yes */)").unwrap();
    assert_eq!(raw.b.get_ron(), "4 /* yes */");

    let raw: WithRawValue = from_str("(a: false, b: (/* this */ 4 /* too */))").unwrap();
    assert_eq!(raw.b.get_ron(), "(/* this */ 4 /* too */)");
}

#[test]
fn test_raw_value_invalid() {
    let err = from_str::<&RawValue>("4.d").unwrap_err();
    assert_eq!(
        err,
        SpannedError {
            code: Error::TrailingCharacters,
            span: Span {
                start: Position { line: 1, col: 3 },
                end: Position { line: 1, col: 3 }
            }
        }
    );

    let err = from_bytes::<&RawValue>(b"\0").unwrap_err();
    assert_eq!(
        err,
        SpannedError {
            code: Error::UnexpectedChar('\0'),
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 }
            }
        }
    )
}

#[test]
fn test_raw_value_from_ron() {
    let raw = RawValue::from_ron("/* hi */ (None, 4.2) /* bye */").unwrap();
    assert_eq!(raw.get_ron(), "/* hi */ (None, 4.2) /* bye */");

    let err = RawValue::from_ron("4.d").unwrap_err();
    assert_eq!(
        err,
        SpannedError {
            code: Error::TrailingCharacters,
            span: Span {
                start: Position { line: 1, col: 3 },
                end: Position { line: 1, col: 3 }
            }
        }
    );

    let raw =
        RawValue::from_boxed_ron(String::from("/* hi */ (None, 4.2) /* bye */").into_boxed_str())
            .unwrap();
    assert_eq!(raw.get_ron(), "/* hi */ (None, 4.2) /* bye */");

    let err = RawValue::from_boxed_ron(String::from("(").into_boxed_str()).unwrap_err();
    assert_eq!(
        err,
        SpannedError {
            code: Error::Eof,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 2 },
            }
        }
    );
}

#[test]
fn test_raw_value_into_rust() {
    let raw = RawValue::from_ron("/* hi */ (a: false, b: None) /* bye */").unwrap();

    let with: WithRawValue = raw.into_rust().unwrap();
    assert_eq!(
        with,
        WithRawValue {
            a: false,
            b: from_str("None").unwrap(),
        }
    );

    let err = raw.into_rust::<i32>().unwrap_err();
    assert_eq!(
        err,
        SpannedError {
            code: Error::ExpectedInteger,
            span: Span {
                start: Position { line: 1, col: 9 },
                end: Position { line: 1, col: 10 },
            }
        }
    );
}

#[test]
fn test_raw_value_from_rust() {
    let raw = RawValue::from_rust(&42).unwrap();
    assert_eq!(raw.get_ron(), "42");

    let raw = RawValue::from_rust(&WithRawValue {
        a: true,
        b: from_str("4.2").unwrap(),
    })
    .unwrap();
    assert_eq!(raw.get_ron(), "(a:true,b:4.2)");
}

#[test]
fn test_raw_value_serde_json() {
    let raw = RawValue::from_ron("/* hi */ (None, 4.2) /* bye */").unwrap();

    let ser = serde_json::to_string(&WithRawValue {
        a: true,
        b: raw.to_owned(),
    })
    .unwrap();
    assert_eq!(ser, "{\"a\":true,\"b\":\"/* hi */ (None, 4.2) /* bye */\"}");

    let with: WithRawValue = serde_json::from_str(&ser).unwrap();
    assert_eq!(raw, &*with.b);

    let err =
        serde_json::from_str::<WithRawValue>("{\"a\":true,\"b\":\"/* hi */ (a:) /* bye */\"}")
            .unwrap_err();
    assert_eq!(
        err.to_string(),
        "invalid RON value at 1:13: Unexpected char ')' at line 1 column 39"
    );

    let err = serde_json::from_str::<WithRawValue>("{\"a\":true,\"b\":42}").unwrap_err();
    assert_eq!(
        err.to_string(),
        "invalid type: integer `42`, expected any valid RON-value-string at line 1 column 16"
    );

    let err = serde_json::from_str::<&RawValue>("\"/* hi */ (a:) /* bye */\"").unwrap_err();
    assert_eq!(
        err.to_string(),
        "invalid RON value at 1:13: Unexpected char ')' at line 1 column 25"
    );

    let err = serde_json::from_str::<&RawValue>("42").unwrap_err();
    assert_eq!(
        err.to_string(),
        "invalid type: integer `42`, expected any valid borrowed RON-value-string at line 1 column 2"
    );
}

#[test]
fn test_raw_value_serde_json_same_checks_as_from_ron() {
    // an unclosed line comment would swallow the rest of the line on serialisation
    let err = serde_json::from_str::<WithRawValue>("{\"a\":true,\"b\":\"42 //\"}").unwrap_err();
    assert!(err.to_string().contains("unclosed line comment"), "{}", err);
    let err = serde_json::from_str::<&RawValue>("\"42 //\"").unwrap_err();
    assert!(err.to_string().contains("unclosed line comment"), "{}", err);
    let err =
        Box::<RawValue>::deserialize(serde::de::value::StrDeserializer::<Error>::new("42 //"))
            .unwrap_err();
    assert!(err.to_string().contains("unclosed line comment"), "{}", err);

    // extensions cannot be enabled inside a raw value
    let err =
        serde_json::from_str::<WithRawValue>("{\"a\":true,\"b\":\"#![enable(implicit_some)] 42\"}")
            .unwrap_err();
    assert!(
        err.to_string().contains("cannot enable extensions"),
        "{}",
        err
    );

    let with: WithRawValue = serde_json::from_str("{\"a\":true,\"b\":\"42 //\\n\"}").unwrap();
    assert_eq!(with.b.get_ron(), "42 //\n");
    assert_eq!(
        ron::from_str::<WithRawValue>(&ron::to_string(&with).unwrap())
            .unwrap()
            .b
            .get_ron(),
        "42 //\n"
    );
}

#[test]
fn test_raw_value_rejects_ignored_attributes() {
    // leading whitespace and comments are still fine
    assert_eq!(
        RawValue::from_ron(" /* a */ 42").unwrap().get_ron(),
        " /* a */ 42"
    );

    // the attribute would end up in the middle of the serialised document
    for ron in ["#![type = \"x\"] 42", " #![schema = \"s.ron\"] (a: 1)"] {
        let err = RawValue::from_ron(ron).unwrap_err();
        assert_eq!(
            err.code,
            Error::Message(String::from(
                "ron::value::RawValue cannot contain attributes"
            ))
        );
        assert!(RawValue::from_boxed_ron(String::from(ron).into_boxed_str()).is_err());
    }

    let err =
        serde_json::from_str::<WithRawValue>("{\"a\":true,\"b\":\"#![type = \\\"x\\\"] 42\"}")
            .unwrap_err();
    assert!(
        err.to_string().contains("cannot contain attributes"),
        "{}",
        err
    );

    // attributes are still allowed at the start of a document holding a raw value
    let with: WithRawValue = ron::from_str("#![type = \"x\"] (a: true, b: 42)").unwrap();
    assert_eq!(with.b.get_ron(), "42");
}

#[test]
fn test_raw_value_clone_into() {
    let raw = RawValue::from_boxed_ron(String::from("(None, 4.2)").into_boxed_str()).unwrap();
    let raw2 = raw.clone();
    assert_eq!(raw, raw2);

    let boxed_str: Box<str> = raw2.into();
    assert_eq!(&*boxed_str, "(None, 4.2)");
}

#[test]
fn test_raw_value_debug_display() {
    let raw = RawValue::from_ron("/* hi */ (None, 4.2) /* bye */").unwrap();

    assert_eq!(format!("{}", raw), "/* hi */ (None, 4.2) /* bye */");
    assert_eq!(
        format!("{:#?}", raw),
        "\
RawValue(
    /* hi */ (None, 4.2) /* bye */,
)\
    "
    );
}

#[test]
fn test_boxed_raw_value_deserialise_from_string() {
    let string = serde::de::value::StringDeserializer::<Error>::new(String::from("4.2"));

    let err = <&RawValue>::deserialize(string.clone()).unwrap_err();
    assert_eq!(
        err,
        Error::InvalidValueForType {
            expected: String::from("any valid borrowed RON-value-string"),
            found: String::from("the string \"4.2\""),
        }
    );

    let boxed_raw = Box::<RawValue>::deserialize(string).unwrap();
    assert_eq!(boxed_raw.get_ron(), "4.2");

    let string = serde::de::value::StringDeserializer::<Error>::new(String::from("["));

    let err = Box::<RawValue>::deserialize(string).unwrap_err();
    assert_eq!(
        err,
        Error::Message(String::from(
            "invalid RON value at 1:1-1:2: Unexpected end of RON"
        ))
    );
}

#[test]
fn test_whitespace_rountrip_issues_and_trim() {
    let mut v = WithRawValue {
        a: true,
        b: RawValue::from_ron("42 ").unwrap().to_owned(),
    };
    v = ron::from_str(&ron::to_string(&v).unwrap()).unwrap();
    assert_eq!(v.b.get_ron(), "42");
    assert_eq!(v.b.trim().get_ron(), "42");
    assert_eq!(v.b.to_owned().trim_boxed().get_ron(), "42");

    for _ in 1..=10 {
        v = ron::from_str(
            &ron::ser::to_string_pretty(&v, ron::ser::PrettyConfig::default()).unwrap(),
        )
        .unwrap();
        assert_eq!(v.b.get_ron(), "42");
        assert_eq!(v.b.trim().get_ron(), "42");
        assert_eq!(v.b.to_owned().trim_boxed().get_ron(), "42");
    }

    assert_eq!(RawValue::from_ron("42").unwrap().trim().get_ron(), "42");
    assert_eq!(
        RawValue::from_ron(" /* start */ 42    // end\n ")
            .unwrap()
            .trim()
            .get_ron(),
        "42"
    );

    assert_eq!(
        RawValue::from_ron("42")
            .unwrap()
            .to_owned()
            .trim_boxed()
            .get_ron(),
        "42"
    );
    assert_eq!(
        RawValue::from_ron(" /* start */ 42    // end\n ")
            .unwrap()
            .to_owned()
            .trim_boxed()
            .get_ron(),
        "42"
    );
    assert_eq!(
        RawValue::from_ron("\u{2028}/* ä */\u{85}\"ö\"\u{2029}// ü\n\u{200E}")
            .unwrap()
            .to_owned()
            .trim_boxed()
            .get_ron(),
        "\"ö\""
    );
}

#[test]
fn test_fuzzer_found_issue() {
    assert_eq!(
        RawValue::from_ron(""),
        Err(SpannedError {
            code: Error::Eof,
            span: Span {
                start: Position { line: 1, col: 1 },
                end: Position { line: 1, col: 1 },
            }
        })
    );

    assert_eq!(ron::from_str("C"), RawValue::from_ron("C"));

    assert_eq!(ron::from_str(" t "), RawValue::from_ron(" t "));

    assert_eq!(
        RawValue::from_ron("#![enable(implicit_some)] 42"),
        Err(SpannedError {
            code: Error::Message(String::from(
                "ron::value::RawValue cannot enable extensions"
            )),
            span: Span {
                start: Position { line: 1, col: 27 },
                end: Position { line: 1, col: 27 },
            }
        })
    );

    assert_eq!(
        RawValue::from_boxed_ron(String::from("#![enable(implicit_some)] 42").into_boxed_str()),
        Err(SpannedError {
            code: Error::Message(String::from(
                "ron::value::RawValue cannot enable extensions"
            )),
            span: Span {
                start: Position { line: 1, col: 27 },
                end: Position { line: 1, col: 27 },
            }
        })
    );

    assert_eq!(
        RawValue::from_ron("42 //"),
        Err(SpannedError {
            code: Error::UnclosedLineComment,
            span: Span {
                start: Position { line: 1, col: 6 },
                end: Position { line: 1, col: 6 },
            }
        })
    );
    assert_eq!(
        ron::from_str::<&RawValue>("42 //"),
        Err(SpannedError {
            code: Error::UnclosedLineComment,
            span: Span {
                start: Position { line: 1, col: 6 },
                end: Position { line: 1, col: 6 },
            }
        })
    );
    assert_eq!(
        ron::from_str::<&RawValue>("42 //\n").unwrap(),
        RawValue::from_ron("42 //\n").unwrap()
    );
    assert_eq!(
        ron::from_str::<&RawValue>("42 /**/").unwrap(),
        RawValue::from_ron("42 /**/").unwrap()
    );
    assert_eq!(
        ron::from_str::<&RawValue>("42 ").unwrap(),
        RawValue::from_ron("42 ").unwrap()
    );

    assert_eq!(
        RawValue::from_ron("a//"),
        Err(SpannedError {
            code: Error::UnclosedLineComment,
            span: Span {
                start: Position { line: 1, col: 4 },
                end: Position { line: 1, col: 4 },
            }
        })
    );
    assert_eq!(
        ron::from_str::<&RawValue>("a//"),
        Err(SpannedError {
            code: Error::UnclosedLineComment,
            span: Span {
                start: Position { line: 1, col: 4 },
                end: Position { line: 1, col: 4 },
            }
        })
    );
    assert_eq!(
        ron::from_str::<&RawValue>("a//\n").unwrap(),
        RawValue::from_ron("a//\n").unwrap()
    );
    assert_eq!(
        ron::from_str::<&RawValue>("a/**/").unwrap(),
        RawValue::from_ron("a/**/").unwrap()
    );
    assert_eq!(
        ron::from_str::<&RawValue>("a").unwrap(),
        RawValue::from_ron("a").unwrap()
    );

    assert_eq!(
        ron::to_string(&Some(Some(RawValue::from_ron("None").unwrap()))).unwrap(),
        "Some(Some(None))"
    );
    // Since a RawValue can contain anything, no implicit Some are allowed around it
    assert_eq!(
        ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .to_string(&Some(Some(RawValue::from_ron("None").unwrap())))
            .unwrap(),
        "Some(Some(None))"
    );
    assert_eq!(
        ron::from_str::<Option<Option<&RawValue>>>("Some(Some(None))").unwrap(),
        Some(Some(RawValue::from_ron("None").unwrap()))
    );
}

#[test]
fn test_raw_value_trims_surrounding_whitespace() {
    let raw: WithRawValue = from_str("(a: false, b:   [1, 2]  \n )").unwrap();
    assert_eq!(raw.b.get_ron(), "[1, 2]");

    // line comments must stay closed
    let raw: WithRawValue = from_str("(a: false, b: 4 // yes\n  \n )").unwrap();
    assert_eq!(raw.b.get_ron(), "4 // yes\n");

    // ... including when there is extra whitespace before the first line break
    let raw: WithRawValue = from_str("(a: false, b: 4   // yes  \n  \n )").unwrap();
    assert_eq!(raw.b.get_ron(), "4   // yes  \n");

    let raw: WithRawValue = from_str("(a: false, b:\n  // so\n  4  )").unwrap();
    assert_eq!(raw.b.get_ron(), "// so\n  4");
}

#[test]
fn test_raw_value_pretty_roundtrip_is_stable() {
    let config = ron::ser::PrettyConfig::default();

    for ron in [
        "(a: false, b: [1, 2])",
        "(a: false, b: /* hi */ [1, 2] /* bye */)",
        "(a: false, b: 4 // bye\n)",
    ] {
        let first =
            ron::ser::to_string_pretty(&from_str::<WithRawValue>(ron).unwrap(), config.clone())
                .unwrap();
        let second =
            ron::ser::to_string_pretty(&from_str::<WithRawValue>(&first).unwrap(), config.clone())
                .unwrap();
        assert_eq!(first, second);
    }

    let first = ron::ser::to_string_pretty(
        &from_str::<WithRawValue>("(a: false, b: [1, 2])").unwrap(),
        config,
    )
    .unwrap();
    assert_eq!(first, "(\n    a: false,\n    b: [1, 2],\n)");
}
