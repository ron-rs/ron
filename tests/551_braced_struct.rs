#[test]
fn raw_value() {
    let _: Vec<Box<ron::value::RawValue>> = ron::from_str(
        r#"
[abc, 922e37, Value, [[]], None, (a: 7), {a: 7}, Person { age: 42 }]
"#,
    )
    .unwrap();
}
