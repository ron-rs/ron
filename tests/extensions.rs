use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct UnitStruct;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct NewType(f32);

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct TupleStruct(UnitStruct, i8);

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
struct Key(u32);

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
enum Enum {
    Unit,
    Bool(bool),
    Chars(char, String),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Struct {
    tuple: ((), NewType, TupleStruct),
    vec: Vec<Option<UnitStruct>>,
    map: HashMap<Key, Enum>,
}

const CONFIG_U_NT: &str = "
#![enable(unwrap_newtypes)]

(
    tuple: ((), 0.5, ((), -5)),
    vec: [
        None,
        Some(()),
    ],
    map: {
        7: Bool(true),
        9: Chars('x', \"\"),
        6: Bool(false),
        5: Unit,
    },
)
";

#[test]
fn unwrap_newtypes() {
    let d: Struct = ron::de::from_str(&CONFIG_U_NT).expect("Failed to deserialize");

    println!("unwrap_newtypes: {:#?}", d);
}

const CONFIG_I_S: &str = "
#![enable(implicit_some)]

(
    tuple: ((), (0.5), ((), -5)),
    vec: [
        None,
        (),
        UnitStruct,
        None,
        (),
    ],
    map: {
        (7): Bool(true),
        (9): Chars('x', \"\"),
        (6): Bool(false),
        (5): Unit,
    },
)
";

#[test]
fn implicit_some() {
    let d: Struct = ron::de::from_str(&CONFIG_I_S).expect("Failed to deserialize");

    println!("implicit_some: {:#?}", d);
}

#[cfg(feature = "enum-repr-extension")]
mod enum_repr {
    use super::*;
    use ron::error::Result;

    #[derive(Debug, Deserialize)]
    struct GraphData {
        nodes: HashMap<u8, String>,
        edges: HashMap<String, (u8, u8)>,
    }

    const CONFIG_E_R: &str = r##"
// chaining the parsers.
// First we parse the type environment.
#[repr(u8)]
enum Vertices {
    Foo,
    Bar,
    Baz
}
// It should hit this (invalid rust since it isn't at the beginning of a file or block).
// then we parse the remained.
#![enable(enum_repr)]

GraphData(
    nodes: {
        Foo: "foo",
        Bar: "bar",
        Baz: "baz",
    },

    edges: {
        "foo -> bar": (Foo, Bar),
        "bar -> baz": (Bar, Baz),
        "baz -> foo": (Baz, Foo),

    }
)
"##;

    const TYPE_ENV_E_R_2: &str = r##"
#[repr(u8)]
enum Vertices {
    Foo,
    Bar,
    Baz
}"##;

    const CONFIG_E_R_2: &str = r##"#![enable(enum_repr)]
GraphData(
    nodes: {
        Foo: "foo",
        Bar: "bar",
        Baz: "baz",
    },

    edges: {
        "foo -> bar": (Foo, Bar),
        "bar -> baz": (Bar, Baz),
        "baz -> foo": (Baz, Foo),

    }
)"##;

    #[test]
    fn enum_repr_deserialize_combined() -> Result<()> {
        let mut tenv = ron::type_env::TypeParser::new(CONFIG_E_R.as_bytes()).parse_enum_decls()?;
        let (tenv, remainder) = tenv.finish();
        let d: GraphData = ron::de::with_type_env(tenv, remainder).expect("Failed to deserialize");
        Ok(println!("enum_repr {:#?}", d))
    }

    #[test]
    fn enum_repr_deserialize_combined_correctly() -> Result<()> {
        let mut tenv = ron::type_env::TypeParser::new(CONFIG_E_R.as_bytes()).parse_enum_decls()?;
        let (tenv, remainder) = tenv.finish();
        let d: GraphData = ron::de::with_type_env(tenv, remainder).expect("Failed to deserialize");
        assert_eq!(d.nodes[&0], "foo");
        assert_eq!(d.edges["foo -> bar"], (0, 1));
        assert_eq!(d.edges["bar -> baz"], (1, 2));
        assert_eq!(d.edges["baz -> foo"], (2, 0));
        Ok(())
    }

    #[test]
    fn enum_repr_deserialize_split_correctly() -> Result<()> {
        let mut tenv =
            ron::type_env::TypeParser::new(TYPE_ENV_E_R_2.as_bytes()).parse_enum_decls()?;
        let (tenv, _) = tenv.finish();
        let d: GraphData =
            ron::de::with_type_env(tenv, CONFIG_E_R_2.as_bytes()).expect("Failed to deserialize");
        assert_eq!(d.nodes[&0], "foo");
        assert_eq!(d.edges["foo -> bar"], (0, 1));
        assert_eq!(d.edges["bar -> baz"], (1, 2));
        assert_eq!(d.edges["baz -> foo"], (2, 0));
        Ok(())
    }
}
