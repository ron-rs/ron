use ron::ser::{to_string_pretty, PrettyConfig};

#[test]
fn small_array() {
    let arr = &[(), (), ()][..];
    assert_eq!(
        to_string_pretty(
            &arr,
            PrettyConfig::new()
                .with_new_line("\n".to_string())
                .with_compact_arrays(false),
        )
        .unwrap(),
        "[
    (),
    (),
    (),
]"
    );
    assert_eq!(
        to_string_pretty(
            &arr,
            PrettyConfig::new()
                .with_new_line("\n".to_string())
                .with_compact_arrays(true)
        )
        .unwrap(),
        "[(),(),()]"
    );
}
