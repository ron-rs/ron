#[test]
fn raw_value() {
    let _: Vec<Box<ron::value::RawValue>> = ron::from_str(
        r#"
[abc, 922e37, Value, [[]], None, (a: 7), {a: 7}]
"#,
    )
    .unwrap();

    let _: Vec<Box<ron::value::RawValue>> = ron::from_str(
        r#"
#![enable(braced_structs)]
[abc, 922e37, Value, [[]], None, {a: 7}]
"#,
    )
    .unwrap();
}
