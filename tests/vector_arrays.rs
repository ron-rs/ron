extern crate ron;
#[macro_use]
extern crate serde;

#[derive(Deserialize, Serialize)]
struct VectorArrays {
    arrays: Vec<[u8; 3]>
}

const RAW: &'static str = "(
    arrays: [
        [0, 0, 0],
    ],
)";

#[test]
fn deserialize_vector_arrays() {
    ron::de::from_str::<VectorArrays>(RAW).unwrap();
}
