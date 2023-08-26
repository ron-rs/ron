use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum EnumWithUnicode {
    Äöß,
    你好世界,
}

#[test]
fn roundtrip_unicode_ident() {
    let value = [EnumWithUnicode::Äöß, EnumWithUnicode::你好世界];
    let serial = ron::ser::to_string(&value).unwrap();

    println!("Serialized: {}", serial);

    let deserial = ron::de::from_str(&serial);

    assert_eq!(Ok(value), deserial);
}

#[test]
fn fuzzer_issues() {
    assert_eq!(
        ron::from_str::<ron::Value>("(__: ())").unwrap(),
        ron::Value::Map(
            [(ron::Value::String(String::from("__")), ron::Value::Unit)]
                .into_iter()
                .collect()
        )
    );
}
