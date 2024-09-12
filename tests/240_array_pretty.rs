use ron::ser::{to_string_pretty, PrettyConfig};

#[test]
fn small_array() {
    let arr = &[(), (), ()][..];
    assert_eq!(
        to_string_pretty(&arr, PrettyConfig::new().new_line("\n")).unwrap(),
        "[
    (),
    (),
    (),
]"
    );
    assert_eq!(
        to_string_pretty(
            &arr,
            PrettyConfig::new().new_line("\n").compact_arrays(true)
        )
        .unwrap(),
        "[(), (), ()]"
    );
    assert_eq!(
        to_string_pretty(
            &arr,
            PrettyConfig::new()
                .new_line("\n")
                .compact_arrays(true)
                .separator("")
        )
        .unwrap(),
        "[(),(),()]"
    );
    assert_eq!(
        to_string_pretty(
            &vec![(1, 2), (3, 4)],
            PrettyConfig::new()
                .new_line("\n")
                .separate_tuple_members(true)
                .compact_arrays(true)
        )
        .unwrap(),
        "[(
    1,
    2,
), (
    3,
    4,
)]"
    );
}
