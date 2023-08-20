#[test]
fn value_deserialises_r_name() {
    // deserialising "A('/')" into ron::Value previously failed as the struct type
    //  searcher reads into the char and then finds a weird comment starter there
    assert_eq!(
        ron::from_str("A('/')"),
        Ok(ron::Value::Seq(vec![ron::Value::Char('/')]))
    );
    assert_eq!(
        ron::from_str("A(\"/\")"),
        Ok(ron::Value::Seq(vec![ron::Value::String(String::from("/"))]))
    );
}
