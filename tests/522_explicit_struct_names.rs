use ron::{
    extensions::Extensions,
    from_str,
    ser::{to_string_pretty, PrettyConfig},
    Error, Options,
};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Id(u32);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Position(f32, f32);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Query {
    None,
    Creature(Id),
    Location(Position),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Foo {
    #[allow(unused)]
    pub id: Id,
    #[allow(unused)]
    pub position: Position,
    #[allow(unused)]
    pub query: Query,
}

const EXPECT_ERROR_MESSAGE: &'static str =
    "expected `Err(Error::ExpectedStructName)`, deserializer returned `Ok`";

#[test]
fn explicit_struct_names() {
    let options = Options::default().with_default_extension(Extensions::EXPLICIT_STRUCT_NAMES);
    let foo_ser = Foo {
        id: Id(3),
        position: Position(0.0, 8.72),
        query: Query::Creature(Id(4)),
    };

    // phase 1 (regular structs)
    let content_regular = r#"(
        id: Id(3),
        position: Position(0.0, 8.72),
        query: None,
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
        query: None,
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
        query: None,
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

    // phase 5 (test serialization)
    let pretty_config = PrettyConfig::new()
        .extensions(Extensions::EXPLICIT_STRUCT_NAMES | Extensions::UNWRAP_VARIANT_NEWTYPES);
    let content = to_string_pretty(&foo_ser, pretty_config).unwrap();
    assert_eq!(
        content,
        r#"#![enable(unwrap_variant_newtypes)]
#![enable(explicit_struct_names)]
Foo(
    id: Id(3),
    position: Position(0.0, 8.72),
    query: Creature(4),
)"#
    );
    let foo_de = from_str::<Foo>(&content);
    match foo_de {
        // GRCOV_EXCL_START
        Err(err) => panic!(
            "failed to deserialize with `explicit_struct_names` and `unwrap_variant_newtypes`: {}",
            err
        ),
        // GRCOV_EXCL_STOP
        Ok(foo_de) => assert_eq!(foo_de, foo_ser),
    }
}
