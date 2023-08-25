use std::f64;

use ron::{
    error::Error,
    value::{Map, Number, Value},
};
use serde_derive::{Deserialize, Serialize};

#[test]
fn bool() {
    assert_eq!("true".parse(), Ok(Value::Bool(true)));
    assert_eq!("false".parse(), Ok(Value::Bool(false)));

    assert_eq!(ron::to_string(&Value::Bool(true)).unwrap(), "true");
    assert_eq!(ron::to_string(&Value::Bool(false)).unwrap(), "false");
}

#[test]
fn char() {
    assert_eq!("'a'".parse(), Ok(Value::Char('a')));

    assert_eq!(ron::to_string(&Value::Char('a')).unwrap(), "'a'");
}

#[test]
fn map() {
    let mut map = Map::new();
    map.insert(Value::Char('a'), Value::Number(Number::U8(1)));
    map.insert(Value::Char('b'), Value::Number(Number::new(2f32)));
    let map = Value::Map(map);

    assert_eq!(ron::to_string(&map).unwrap(), "{'a':1,'b':2.0}");

    assert_eq!("{ 'a': 1, 'b': 2.0 }".parse(), Ok(map));
}

#[test]
fn number() {
    assert_eq!("42".parse(), Ok(Value::Number(Number::U8(42))));
    assert_eq!(
        "3.141592653589793".parse(),
        Ok(Value::Number(Number::new(f64::consts::PI)))
    );

    assert_eq!(
        ron::to_string(&Value::Number(Number::U8(42))).unwrap(),
        "42"
    );
    assert_eq!(
        ron::to_string(&Value::Number(Number::F64(f64::consts::PI.into()))).unwrap(),
        "3.141592653589793"
    );
}

#[test]
fn option() {
    let opt = Some(Box::new(Value::Char('c')));
    assert_eq!("Some('c')".parse(), Ok(Value::Option(opt)));
    assert_eq!("None".parse(), Ok(Value::Option(None)));

    assert_eq!(
        ron::to_string(&Value::Option(Some(Box::new(Value::Char('c'))))).unwrap(),
        "Some('c')"
    );
    assert_eq!(ron::to_string(&Value::Option(None)).unwrap(), "None");
}

#[test]
fn string() {
    let normal = "\"String\"";
    assert_eq!(normal.parse(), Ok(Value::String("String".into())));
    assert_eq!(
        ron::to_string(&Value::String("String".into())).unwrap(),
        "\"String\""
    );

    let raw = "r\"Raw String\"";
    assert_eq!(raw.parse(), Ok(Value::String("Raw String".into())));

    let raw_hashes = "r#\"Raw String\"#";
    assert_eq!(raw_hashes.parse(), Ok(Value::String("Raw String".into())));

    let raw_escaped = "r##\"Contains \"#\"##";
    assert_eq!(
        raw_escaped.parse(),
        Ok(Value::String("Contains \"#".into()))
    );

    let raw_multi_line = "r\"Multi\nLine\"";
    assert_eq!(
        raw_multi_line.parse(),
        Ok(Value::String("Multi\nLine".into()))
    );
    assert_eq!(
        ron::to_string(&Value::String("Multi\nLine".into())).unwrap(),
        "\"Multi\\nLine\""
    );
}

#[test]
fn byte_string() {
    assert_eq!(
        "b\"\\x01\\u{2}\\0\\x04\"".parse(),
        Ok(Value::Bytes(vec![1, 2, 0, 4]))
    );
    assert_eq!(
        ron::to_string(&Value::Bytes(vec![1, 2, 0, 4])).unwrap(),
        "b\"\\x01\\x02\\x00\\x04\""
    );
}

#[test]
fn seq() {
    let seq = Value::Seq(vec![
        Value::Number(Number::U8(1)),
        Value::Number(Number::new(2f32)),
    ]);
    assert_eq!(ron::to_string(&seq).unwrap(), "[1,2.0]");
    assert_eq!("[1, 2.0]".parse(), Ok(seq));

    let err = Value::Seq(vec![Value::Number(Number::new(1))])
        .into_rust::<[i32; 2]>()
        .unwrap_err();

    assert_eq!(
        err,
        Error::ExpectedDifferentLength {
            expected: String::from("an array of length 2"),
            found: 1,
        }
    );

    let err = Value::Seq(vec![
        Value::Number(Number::new(1)),
        Value::Number(Number::new(2)),
        Value::Number(Number::new(3)),
    ])
    .into_rust::<[i32; 2]>()
    .unwrap_err();

    assert_eq!(
        err,
        Error::ExpectedDifferentLength {
            expected: String::from("a sequence of length 2"),
            found: 3,
        }
    );
}

#[test]
fn unit() {
    use ron::error::{Error, Position, SpannedError};

    assert_eq!("()".parse(), Ok(Value::Unit));
    assert_eq!("Foo".parse(), Ok(Value::Unit));

    assert_eq!(
        "".parse::<Value>(),
        Err(SpannedError {
            code: Error::Eof,
            position: Position { col: 1, line: 1 }
        })
    );

    assert_eq!(ron::to_string(&Value::Unit).unwrap(), "()");
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Scene(Option<(u32, u32)>);

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Scene2 {
    foo: Option<(u32, u32)>,
}

#[test]
fn roundtrip() {
    use ron::{de::from_str, ser::to_string};

    {
        let v = Scene2 {
            foo: Some((122, 13)),
        };
        let s = to_string(&v).unwrap();
        println!("{}", s);
        let val: Value = from_str(&s).unwrap();
        println!("{:?}", val);
        let v2 = val.into_rust::<Scene2>();
        assert_eq!(v2, Ok(v));
    }
    {
        let v = Scene(Some((13, 122)));
        let s = to_string(&v).unwrap();
        println!("{}", s);
        let val: Value = from_str(&s).unwrap();
        println!("{:?}", val);
        let v2 = val.into_rust::<Scene>();
        assert_eq!(v2, Ok(v));
    }
    {
        let v = (42,);
        let s = to_string(&v).unwrap();
        println!("{}", s);
        let val: Value = from_str(&s).unwrap();
        println!("{:?}", val);
        let v2 = val.into_rust::<(i32,)>();
        assert_eq!(v2, Ok(v));
    }
}

#[test]
fn map_roundtrip_338() {
    // https://github.com/ron-rs/ron/issues/338

    let v: Value = ron::from_str("{}").unwrap();
    println!("{:?}", v);

    let ser = ron::to_string(&v).unwrap();
    println!("{:?}", ser);

    let roundtrip = ron::from_str(&ser).unwrap();
    println!("{:?}", roundtrip);

    assert_eq!(v, roundtrip);
}
