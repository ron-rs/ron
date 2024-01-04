use ron::{Options, extensions::Extensions, Error, from_str};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
struct Id(u32);

#[derive(Debug, Deserialize)]
struct Foo {
    #[allow(unused)]
    pub id: Id,
}

const EXPECT_ERROR_MESSAGE: &'static str = "expected `Err(Error::ExpectedStructName)`, deserializer returned `Ok`";
const INCORRECT_ERROR_MESSAGE: &'static str = "expected error ExpectedStructName, found";

#[test]
fn explicit_struct_names() {
    let options = Options::default()
        .with_default_extension(Extensions::EXPLICIT_STRUCT_NAMES);

    // phase 1
    let content = r#"(
        id: Id(3),
    )"#;
    let foo = options.from_str::<Foo>(content);
    match foo.expect_err(EXPECT_ERROR_MESSAGE).code {
        Error::ExpectedStructName(_) => {},
        err => panic!("{INCORRECT_ERROR_MESSAGE} \"{err}\""),
    }

    // phase 2
    let content = r#"Foo(
        id: (3),
    )"#;
    let foo = options.from_str::<Foo>(content);
    match foo.expect_err(EXPECT_ERROR_MESSAGE).code {
        Error::ExpectedStructName(_) => {},
        err => panic!("{INCORRECT_ERROR_MESSAGE} \"{err}\""),
    }

    // phase 3 (use content from phase 2)
    let _foo = from_str::<Foo>(content).unwrap();
}
