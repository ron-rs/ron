use serde::{Deserialize, Serialize};

#[test]
fn nested_untagged_enum() {
    // Contributed by @caelunshun in https://github.com/ron-rs/ron/issues/217

    #[derive(Debug, PartialEq, Deserialize)]
    struct Root {
        value: Either,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(transparent)]
    struct Newtype(Either);

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(untagged)]
    enum Either {
        One(One),
        Two(Two),
    }

    #[derive(Debug, PartialEq, Deserialize)]
    enum One {
        OneA,
        OneB,
        OneC,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    enum Two {
        TwoA,
        TwoB,
        TwoC,
    }

    assert_eq!(ron::de::from_str("OneA"), Ok(One::OneA));
    assert_eq!(ron::de::from_str("OneA"), Ok(Either::One(One::OneA)));
    assert_eq!(
        ron::de::from_str("(value: OneA)"),
        Ok(Root {
            value: Either::One(One::OneA)
        })
    );
    assert_eq!(
        ron::de::from_str("OneA"),
        Ok(Newtype(Either::One(One::OneA)))
    );
}

#[test]
fn untagged_enum_of_enum_list() {
    // Contributed by @joonazan in https://github.com/ron-rs/ron/issues/217

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub enum UnitType {
        Explorer,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    #[serde(untagged)]
    enum CardTextNumberFlat {
        JustNum(u8),
        RangeW(Range_),
        CountUnitW(CountUnit_),
        PowerCardsPlayedW(PowerCardsPlayed),
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Range_ {
        Range: u8,
    }
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    #[allow(non_snake_case)]
    struct CountUnit_ {
        CountUnit: Vec<UnitType>,
    }
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct PowerCardsPlayed;

    let units = CardTextNumberFlat::CountUnitW(CountUnit_ {
        CountUnit: vec![UnitType::Explorer],
    });
    let range = CardTextNumberFlat::RangeW(Range_ { Range: 1 });

    let units_ron: String = ron::to_string(&units).unwrap();
    let range_ron = ron::to_string(&range).unwrap();

    assert_eq!(units_ron, "(CountUnit:[Explorer])");
    assert_eq!(range_ron, "(Range:1)");

    let units_de: CardTextNumberFlat = ron::from_str(&units_ron).unwrap();
    let range_de: CardTextNumberFlat = ron::from_str(&range_ron).unwrap();

    assert_eq!(units_de, units);
    assert_eq!(range_de, range);
}
