use ron::value::{Number, Value};
use serde_derive::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
struct Config {
    id: u32,
    name: String,
}

#[test]
fn from_value_deserializes_struct() {
    let value: Value = ron::from_str("(id: 7, name: \"ron\")").unwrap();

    let config: Config = ron::de::from_value(value).unwrap();

    assert_eq!(
        config,
        Config {
            id: 7,
            name: "ron".to_owned(),
        }
    );
}

#[test]
fn from_value_matches_into_rust() {
    let value: Value = ron::from_str("(id: 1, name: \"a\")").unwrap();

    let via_fn: Config = ron::de::from_value(value.clone()).unwrap();
    let via_method: Config = value.into_rust().unwrap();

    assert_eq!(via_fn, via_method);
}

#[test]
fn from_value_reports_error_on_type_mismatch() {
    let value = Value::Number(Number::U8(1));

    let result: Result<Config, _> = ron::de::from_value(value);

    assert!(result.is_err());
}
