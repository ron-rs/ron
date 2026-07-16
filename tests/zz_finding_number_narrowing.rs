//! The `roundtrip` fuzz target auto-discovered this: parsing a numeric literal
//! that carries an explicit type suffix (a first-class RON feature) yields a
//! `Value::Number` of that width, but the default serializer emits NO suffix,
//! so re-parsing narrows the number to the smallest fitting type. The numeric
//! *value* survives; the numeric *type* does not. `Value` round-trip is
//! therefore not type-preserving. Correctness/fidelity note, not a soundness
//! hole — but worth knowing before relying on Value round-trips.

use ron::value::{Number, Value};

fn parse(s: &str) -> Value {
    ron::from_str(s).unwrap()
}

#[test]
fn suffix_number_type_not_preserved_across_roundtrip() {
    // f64-suffixed float parses as F64 ...
    assert_eq!(parse("1.5f64"), Value::Number(Number::F64(1.5f64.into())));
    // ... but serializes without a suffix ...
    let s = ron::to_string(&parse("1.5f64")).unwrap();
    assert_eq!(s, "1.5");
    // ... and re-parses as F32. Type changed f64 -> f32 across the round-trip.
    assert_eq!(parse(&s), Value::Number(Number::F32(1.5f32.into())));
    assert_ne!(parse("1.5f64"), parse(&s));

    // Same for the NaN input the fuzzer actually minimised to:
    assert_eq!(parse("NaNf64"), Value::Number(Number::F64(f64::NAN.into())));
    assert_eq!(parse(&ron::to_string(&parse("NaNf64")).unwrap()),
               Value::Number(Number::F32(f32::NAN.into())));

    // Integers narrow the same way: a u16 literal comes back as u8.
    assert_eq!(parse("5u16"), Value::Number(Number::U16(5)));
    assert_eq!(parse(&ron::to_string(&parse("5u16")).unwrap()),
               Value::Number(Number::U8(5)));
}
