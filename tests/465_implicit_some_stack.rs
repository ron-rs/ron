#[test]
fn roundtrip_implicit_some_stack() {
    check_roundtrip(Option::<()>::None, "None");
    check_roundtrip(Some(()), "()");
    check_roundtrip(Some(Some(())), "()");
    check_roundtrip(Some(Some(Some(()))), "()");
    check_roundtrip(Some(Option::<()>::None), "Some(None)");
    check_roundtrip(Some(Some(Option::<()>::None)), "Some(Some(None))");
}

fn check_roundtrip<
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
>(
    val: T,
    check: &str,
) {
    let options =
        ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);

    let ron = options.to_string(&val).unwrap();
    assert_eq!(ron, check);

    let de: T = options.from_str(&ron).unwrap();
    assert_eq!(de, val);
}
