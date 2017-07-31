extern crate ron;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use std::fs::File;

use ron::ser::to_string;

#[derive(Serialize)]
struct Config {
    boolean: bool,
    float: f32,
    map: HashMap<u8, char>,
    nested: Nested,
}

#[derive(Serialize)]
struct Nested {
    a: String,
    b: char,
}

fn main() {
    use std::io::Write;
    use std::iter::FromIterator;

    let mut file = File::create("config.ron").expect("Failed to create file");

    let s = to_string(&Config {
        boolean: false,
        float: 2.18,
        map: HashMap::from_iter(vec![(0, '1'), (1, '2'), (3, '5'), (8, '1')]),
        nested: Nested {
            a: "Hello from \"RON\"".to_string(),
            b: 'b',
        },
    }).expect("Serialization failed");

    file.write(s.as_bytes()).expect("Failed to write data to file");
}
