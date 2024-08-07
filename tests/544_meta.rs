#[test]
fn serialize_field() {
    #[derive(serde::Serialize)]
    enum PetKind {
        Isopod,
    }

    #[derive(serde::Serialize)]
    struct Pet {
        name: &'static str,
        age: u8,
        kind: PetKind,
    }

    #[derive(serde::Serialize)]
    struct Person {
        name: &'static str,
        age: u8,
        knows: Vec<usize>,
        pet: Option<Pet>,
    }

    let value = vec![
        Person {
            name: "Alice",
            age: 29,
            knows: vec![1],
            pet: Some(Pet {
                name: "Herbert",
                age: 7,
                kind: PetKind::Isopod,
            }),
        },
        Person {
            name: "Bob",
            age: 29,
            knows: vec![0],
            pet: None,
        },
    ];

    let mut config = ron::ser::PrettyConfig::default();

    // layer 0
    config
        .meta
        .field("age")
        .with_meta("0@age (person)\nmust be within range 0..256");
    config
        .meta
        .field("knows")
        .with_meta("0@knows (person)\nmust be list of valid person indices");
    config.meta.field("pet").build_fields(|fields| {
        // layer 1
        fields
            .field("age")
            .with_meta("1@age (pet)\nmust be valid range 0..256");
        fields
            .field("kind")
            .with_meta("1@kind (pet)\nmust be `Isopod`");
    });

    // provide meta for a field that doesn't exist;
    // this should not end up anywhere in the final string
    config.meta.field("0").with_meta("unreachable");

    let s = ron::ser::to_string_pretty(&value, config).unwrap();

    assert_eq!(
        s,
        r#"[
    (
        name: "Alice",
        /// 0@age (person)
        /// must be within range 0..256
        age: 29,
        /// 0@knows (person)
        /// must be list of valid person indices
        knows: [
            1,
        ],
        pet: Some((
            name: "Herbert",
            /// 1@age (pet)
            /// must be valid range 0..256
            age: 7,
            /// 1@kind (pet)
            /// must be `Isopod`
            kind: Isopod,
        )),
    ),
    (
        name: "Bob",
        /// 0@age (person)
        /// must be within range 0..256
        age: 29,
        /// 0@knows (person)
        /// must be list of valid person indices
        knows: [
            0,
        ],
        pet: None,
    ),
]"#
    );
}
