use std::fmt::Write;

use serde::{Deserialize, Serialize};

#[test]
fn typetag_usage() {
    #[derive(Deserialize, Serialize, Debug)]
    struct Component1 {
        value: f32,
        value_2: f32,
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct Component2 {
        value: f32,
    }

    #[typetag::serde(tag = "type")]
    trait MyTrait: std::fmt::Debug {
        fn do_stuff(&self, buffer: &mut String);
    }

    #[typetag::serde]
    impl MyTrait for Component1 {
        fn do_stuff(&self, buffer: &mut String) {
            let _ = writeln!(buffer, "{:#?}", self);
        }
    }

    #[typetag::serde]
    impl MyTrait for Component2 {
        fn do_stuff(&self, buffer: &mut String) {
            let _ = writeln!(buffer, "{:#?}", self);
        }
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct EntityConfig {
        name: String,
        components: Vec<Box<dyn MyTrait>>,
    }

    let ron_config = r#"EntityConfig(
    name: "some name",
    components: [
        {
            "type": "Component1",
            "value": 22.0,
            "value_2": 35.0,
        },
        {
            "type": "Component2",
            "value": 3.14,
        },
    ],
)"#;

    let entity_config: EntityConfig = ron::from_str(ron_config).unwrap();

    assert_eq!(entity_config.name, "some name");
    assert_eq!(entity_config.components.len(), 2);

    let mut buffer = String::new();

    for component in &entity_config.components {
        component.do_stuff(&mut buffer);
    }

    assert_eq!(
        buffer,
        r#"Component1 {
    value: 22.0,
    value_2: 35.0,
}
Component2 {
    value: 3.14,
}
"#
    );

    let ron = ron::ser::to_string_pretty(
        &entity_config,
        ron::ser::PrettyConfig::default().struct_names(true),
    )
    .unwrap();
    assert_eq!(ron, ron_config);
}

#[test]
fn typetag_with_enum() {
    #[derive(Deserialize, Serialize, Debug)]
    enum SampleTestEnum {
        Test(i32),
        AnotherTest(f32, f32),
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct Component1 {
        value: f32,
        value_2: f32,
        something: SampleTestEnum,
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct Component2 {
        value: f32,
    }

    #[typetag::serde(tag = "type")]
    trait MyTrait: std::fmt::Debug {
        fn do_stuff(&self, buffer: &mut String);
    }

    #[typetag::serde]
    impl MyTrait for Component1 {
        fn do_stuff(&self, buffer: &mut String) {
            match self.something {
                SampleTestEnum::Test(number) => {
                    let _ = writeln!(buffer, "my number: {:#?}", number);
                }
                SampleTestEnum::AnotherTest(float_1, float_2) => {
                    let _ = writeln!(buffer, "f1: {:#?}, f2: {:#?}", float_1, float_2);
                }
            }
        }
    }

    #[typetag::serde]
    impl MyTrait for Component2 {
        fn do_stuff(&self, buffer: &mut String) {
            let _ = writeln!(buffer, "{:#?}", self.value);
        }
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct EntityConfig {
        name: String,
        components: Vec<Box<dyn MyTrait>>,
    }

    let ron_config = r#"EntityConfig(
    name: "some other name",
    components: [
        {
            "type": "Component1",
            "value": 22.0,
            "value_2": 35.0,
            "something": Test(22),
        },
        {
            "type": "Component1",
            "value": 12.0,
            "value_2": 11.0,
            "something": AnotherTest(11.0, 22.0),
        },
        {
            "type": "Component2",
            "value": 3.1,
        },
    ],
)"#;

    let entity_config: EntityConfig = ron::from_str(ron_config).unwrap();

    assert_eq!(entity_config.name, "some other name");
    assert_eq!(entity_config.components.len(), 3);

    let mut buffer = String::new();

    for component in &entity_config.components {
        component.do_stuff(&mut buffer);
    }

    assert_eq!(
        buffer,
        r#"my number: 22
f1: 11.0, f2: 22.0
3.1
"#
    );

    let ron = ron::ser::to_string_pretty(
        &entity_config,
        ron::ser::PrettyConfig::default().struct_names(true),
    )
    .unwrap();
    assert_eq!(ron, ron_config);
}
