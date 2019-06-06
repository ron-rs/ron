// Addresses unresolved questions of PR #93
// TODO(torkleyy): only question 1 is resolved, 2 is not yet resolved

use ron::ser::{to_string, to_string_pretty, PrettyConfig};

#[test]
fn omit_trailing_comma_non_pretty() {
    let x = vec![vec![1u32, 2, 3], vec![4, 5, 6]];

    let s = to_string(&x).unwrap();
    assert_eq!(s, "[[1,2,3],[4,5,6]]");
}
