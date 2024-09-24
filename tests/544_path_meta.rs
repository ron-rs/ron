use ron::ser::{path_meta::Field, PrettyConfig};

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

    let value = (
        Person {
            name: "Walter",
            age: 43,
            knows: vec![0, 1],
            pet: None,
        },
        vec![
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
        ],
    );

    let mut config = PrettyConfig::default();

    config
        .path_meta
        .get_or_insert_with(Field::empty)
        .build_fields(|fields| {
            fields
                .field("age")
                .with_doc("0@age (person)\nmust be within range 0..256");
            fields
                .field("knows")
                .with_doc("0@knows (person)\nmust be list of valid person indices");
            fields.field("pet").build_fields(|fields| {
                fields
                    .field("age")
                    .with_doc("1@age (pet)\nmust be valid range 0..256");
                fields
                    .field("kind")
                    .with_doc("1@kind (pet)\nmust be `Isopod`");
            });

            // provide meta for a field that doesn't exist;
            // this should not end up anywhere in the final string
            fields.field("0").with_doc("unreachable");
        });

    let s = ron::ser::to_string_pretty(&value, config).unwrap();

    assert_eq!(
        s,
        r#"((
    name: "Walter",
    /// 0@age (person)
    /// must be within range 0..256
    age: 43,
    /// 0@knows (person)
    /// must be list of valid person indices
    knows: [
        0,
        1,
    ],
    pet: None,
), [
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
])"#
    );
}
