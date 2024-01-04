use ron::{extensions::Extensions, from_str, Error, Options};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
struct Id(u32);

#[derive(Debug, Deserialize)]
struct Position(f32, f32);

#[derive(Debug, Deserialize)]
struct Foo {
    #[allow(unused)]
    pub id: Id,
    #[allow(unused)]
    pub position: Position,
}

const EXPECT_ERROR_MESSAGE: &'static str =
    "expected `Err(Error::ExpectedStructName)`, deserializer returned `Ok`";

#[test]
fn explicit_struct_names() {
    let options = Options::default().with_default_extension(Extensions::EXPLICIT_STRUCT_NAMES);

    // phase 1 (regular structs)
    let content_regular = r#"(
        id: Id(3),
        position: Position(0.0, 8.72),
    )"#;
    let foo = options.from_str::<Foo>(content_regular);
    assert_eq!(
        foo.expect_err(EXPECT_ERROR_MESSAGE).code,
        Error::ExpectedStructName("Foo".to_string())
    );

    // phase 2 (newtype structs)
    let content_newtype = r#"Foo(
        id: (3),
        position: Position(0.0, 8.72),
    )"#;
    let foo = options.from_str::<Foo>(content_newtype);
    assert_eq!(
        foo.expect_err(EXPECT_ERROR_MESSAGE).code,
        Error::ExpectedStructName("Id".to_string())
    );

    // phase 3 (tuple structs)
    let content_tuple = r#"Foo(
        id: Id(3),
        position: (0.0, 8.72),
    )"#;
    let foo = options.from_str::<Foo>(content_tuple);
    assert_eq!(
        foo.expect_err(EXPECT_ERROR_MESSAGE).code,
        Error::ExpectedStructName("Position".to_string())
    );

    // phase 4 (test without this extension)
    let _foo1 = from_str::<Foo>(content_regular).unwrap();
    let _foo2 = from_str::<Foo>(content_newtype).unwrap();
    let _foo3 = from_str::<Foo>(content_tuple).unwrap();
}
