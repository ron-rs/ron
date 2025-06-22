use ron::Number;

#[test]
fn exhaustive_number_match() {
    match Number::U8(42) {
        Number::I8(v) => println!("{v}"),
        Number::I16(v) => println!("{v}"),
        Number::I32(v) => println!("{v}"),
        Number::I64(v) => println!("{v}"),
        #[cfg(feature = "integer128")]
        Number::I128(v) => println!("{v}"),
        Number::U8(v) => println!("{v}"),
        Number::U16(v) => println!("{v}"),
        Number::U32(v) => println!("{v}"),
        Number::U64(v) => println!("{v}"),
        #[cfg(feature = "integer128")]
        Number::U128(v) => println!("{v}"),
        Number::F32(v) => println!("{}", v.0),
        Number::F64(v) => println!("{}", v.0),
        #[cfg(not(doc))]
        Number::__NonExhaustive(never) => never.never(),
    }
}

#[test]
fn non_exhaustive_number_match() {
    match Number::U8(42) {
        Number::I8(v) => println!("{v}"),
        Number::I16(v) => println!("{v}"),
        Number::I32(v) => println!("{v}"),
        Number::I64(v) => println!("{v}"),
        Number::U8(v) => println!("{v}"),
        Number::U16(v) => println!("{v}"),
        Number::U32(v) => println!("{v}"),
        Number::U64(v) => println!("{v}"),
        Number::F32(v) => println!("{}", v.0),
        Number::F64(v) => println!("{}", v.0),
        v => println!("{v:?}"),
    }
}
