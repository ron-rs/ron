use ron::error::{Error, Position, Span, SpannedError};

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct MaybeFields {
    f1: i64,
    f2: Option<i64>,
    f3: Option<Option<i64>>,
}

#[test]
fn test_recursive_implicit_some() {
    // Test case provided by d86leader in
    //  https://github.com/ron-rs/ron/issues/367#issue-1147920589

    let x1: std::result::Result<MaybeFields, _> =
        ron::from_str("#![enable(implicit_some)]\n(f1: 1)");
    let x2: std::result::Result<MaybeFields, _> =
        ron::from_str("#![enable(implicit_some)]\n(f1: 1, f2: None, f3: None)");
    let x3: std::result::Result<MaybeFields, _> =
        ron::from_str("#![enable(implicit_some)]\n(f1: 1, f2: 2, f3: 3)");
    let x4: std::result::Result<MaybeFields, _> =
        ron::from_str("#![enable(implicit_some)]\n(f1: 1, f2: 2, f3: Some(3))");
    let x5: std::result::Result<MaybeFields, _> =
        ron::from_str("#![enable(implicit_some)]\n(f1: 1, f2: 2, f3: Some(Some(3)))");
    let x6: std::result::Result<MaybeFields, _> =
        ron::from_str("#![enable(implicit_some)]\n(f1: 1, f2: 2, f3: Some(None))");

    assert_eq!(
        x1,
        Ok(MaybeFields {
            f1: 1,
            f2: None,
            f3: None
        })
    );
    assert_eq!(
        x2,
        Ok(MaybeFields {
            f1: 1,
            f2: None,
            f3: None
        })
    );
    assert_eq!(
        x3,
        Ok(MaybeFields {
            f1: 1,
            f2: Some(2),
            f3: Some(Some(3))
        })
    );
    assert_eq!(
        x4,
        Ok(MaybeFields {
            f1: 1,
            f2: Some(2),
            f3: Some(Some(3))
        })
    );
    assert_eq!(
        x5,
        Ok(MaybeFields {
            f1: 1,
            f2: Some(2),
            f3: Some(Some(3))
        })
    );
    assert_eq!(
        x6,
        Ok(MaybeFields {
            f1: 1,
            f2: Some(2),
            f3: Some(None)
        })
    );
}

#[test]
fn test_nested_implicit_some() {
    assert_eq!(
        ron::from_str::<Option<Option<Option<u32>>>>("#![enable(implicit_some)]\n5"),
        Ok(Some(Some(Some(5))))
    );
    assert_eq!(
        ron::from_str::<Option<Option<Option<u32>>>>("#![enable(implicit_some)]\nNone"),
        Ok(None)
    );
    assert_eq!(
        ron::from_str::<Option<Option<Option<u32>>>>("#![enable(implicit_some)]\nSome(5)"),
        Ok(Some(Some(Some(5))))
    );
    assert_eq!(
        ron::from_str::<Option<Option<Option<u32>>>>("#![enable(implicit_some)]\nSome(None)"),
        Ok(Some(None))
    );
    assert_eq!(
        ron::from_str::<Option<Option<Option<u32>>>>("#![enable(implicit_some)]\nSome(Some(5))"),
        Ok(Some(Some(Some(5))))
    );
    assert_eq!(
        ron::from_str::<Option<Option<Option<u32>>>>("#![enable(implicit_some)]\nSome(Some(None))"),
        Ok(Some(Some(None)))
    );
    assert_eq!(
        ron::from_str::<Option<Option<Option<u32>>>>(
            "#![enable(implicit_some)]\nSome(Some(Some(5)))"
        ),
        Ok(Some(Some(Some(5))))
    );
}

#[test]
fn test_implicit_some_requires_parens_after_some() {
    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct Config {
        name: Option<String>,
        limit: Option<Option<u32>>,
    }

    // whitespace and comments are still allowed between `Some` and `(`
    assert_eq!(
        ron::from_str::<Config>(
            "#![enable(implicit_some)]\n(name: Some (\"a\"), limit: Some /* x */ (None))"
        ),
        Ok(Config {
            name: Some(String::from("a")),
            limit: Some(None),
        })
    );

    // `Some` without `(` is rejected like without implicit_some,
    //  instead of being dropped before the implicit `Some`
    assert_eq!(
        ron::from_str::<Config>("(name: Some \"a\", limit: None)"),
        Err(SpannedError {
            code: Error::ExpectedOption,
            span: Span {
                start: Position { line: 1, col: 12 },
                end: Position { line: 1, col: 13 },
            }
        })
    );
    assert_eq!(
        ron::from_str::<Config>("#![enable(implicit_some)]\n(name: Some \"a\", limit: None)"),
        Err(SpannedError {
            code: Error::ExpectedOption,
            span: Span {
                start: Position { line: 2, col: 12 },
                end: Position { line: 2, col: 13 },
            }
        })
    );
    assert_eq!(
        ron::from_str::<Config>("#![enable(implicit_some)]\n(name: \"a\", limit: Some None)"),
        Err(SpannedError {
            code: Error::ExpectedOption,
            span: Span {
                start: Position { line: 2, col: 24 },
                end: Position { line: 2, col: 25 },
            }
        })
    );
    assert_eq!(
        ron::from_str::<ron::Value>("#![enable(implicit_some)]\n[Some 5]"),
        Err(SpannedError {
            code: Error::ExpectedOption,
            span: Span {
                start: Position { line: 2, col: 6 },
                end: Position { line: 2, col: 7 },
            }
        })
    );
}

#[test]
fn fuzzer_found_issues() {
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Some_();

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct None_();

    let ser_some = ron::ser::to_string_pretty(
        &Some(Some_()),
        ron::ser::PrettyConfig::default()
            .struct_names(true)
            .extensions(ron::extensions::Extensions::IMPLICIT_SOME),
    )
    .unwrap();
    assert_eq!(ser_some, "#![enable(implicit_some)]\nSome_()");

    let ser_none = ron::ser::to_string_pretty(
        &Some(None_()),
        ron::ser::PrettyConfig::default()
            .struct_names(true)
            .extensions(ron::extensions::Extensions::IMPLICIT_SOME),
    )
    .unwrap();
    assert_eq!(ser_none, "#![enable(implicit_some)]\nNone_()");

    assert_eq!(
        ron::from_str::<Option<Some_>>(&ser_some).unwrap(),
        Some(Some_())
    );
    assert_eq!(
        ron::from_str::<Option<None_>>(&ser_none).unwrap(),
        Some(None_())
    );
}
