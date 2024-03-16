use std::ops::Bound;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, Debug)]
#[serde(untagged)]
enum EitherInterval<V> {
    B(Interval<V>),
    D(V, Option<V>),
}

type Interval<V> = (Bound<V>, Bound<V>);

#[test]
fn serde_roundtrip() {
    let interval = EitherInterval::B((Bound::Excluded(0u8), Bound::Unbounded));

    let ron: String = ron::ser::to_string(&interval).unwrap();
    assert_eq!(ron, "(Excluded(0),Unbounded)");

    let de = ron::de::from_str(&ron).unwrap();
    assert_eq!(interval, de);
}
