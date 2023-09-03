use ron::{extensions::Extensions, ser::PrettyConfig, Options};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Newtype(f64);

#[derive(Serialize, Deserialize)]
struct Struct(Option<u32>, Newtype);

#[test]
fn default_options() {
    let ron = Options::default();

    let de: Struct = ron.from_str("(Some(42),(4.2))").unwrap();
    let ser = ron.to_string(&de).unwrap();

    assert_eq!(ser, "(Some(42),(4.2))");
}

#[test]
fn without_any_options() {
    let mut ron = Options::default().with_default_extension(Extensions::all());
    for extension in Extensions::all().iter() {
        ron = ron.without_default_extension(extension);
    }

    let de: Struct = ron.from_str("(Some(42),(4.2))").unwrap();
    let ser = ron.to_string(&de).unwrap();

    assert_eq!(ser, "(Some(42),(4.2))");
}

#[test]
fn single_default_extension() {
    let ron = Options::default().with_default_extension(Extensions::IMPLICIT_SOME);

    let de: Struct = ron.from_str("(42,(4.2))").unwrap();
    let ser = ron.to_string(&de).unwrap();

    assert_eq!(ser, "(42,(4.2))");

    let de: Struct = ron.from_str("#![enable(implicit_some)](42,(4.2))").unwrap();
    let ser = ron.to_string(&de).unwrap();

    assert_eq!(ser, "(42,(4.2))");

    let de: Struct = ron
        .from_str("#![enable(implicit_some)]#![enable(unwrap_newtypes)](42,4.2)")
        .unwrap();
    let ser = ron.to_string(&de).unwrap();

    assert_eq!(ser, "(42,(4.2))");

    let de: Struct = ron
        .from_str("#![enable(implicit_some)]#![enable(unwrap_newtypes)](42,4.2)")
        .unwrap();
    let ser = ron
        .to_string_pretty(
            &de,
            PrettyConfig::default().extensions(Extensions::UNWRAP_NEWTYPES),
        )
        .unwrap();

    assert_eq!(ser, "#![enable(unwrap_newtypes)]\n(42, 4.2)");
}

#[test]
fn reader_io_error() {
    struct Reader<'a> {
        buf: &'a [u8],
    }

    impl<'a> std::io::Read for Reader<'a> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let written = self.buf.read(buf)?;
            if written == 0 {
                Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "oh no"))
            } else {
                Ok(written)
            }
        }
    }

    assert_eq!(
        ron::de::from_reader::<Reader, ()>(Reader { buf: b"" }).unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::Io(String::from("oh no")),
            position: ron::error::Position { line: 1, col: 1 },
        }
    );
    assert_eq!(
        ron::de::from_reader::<Reader, ()>(Reader { buf: b"hello" }).unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::Io(String::from("oh no")),
            position: ron::error::Position { line: 1, col: 6 },
        }
    );
    assert_eq!(
        ron::de::from_reader::<Reader, ()>(Reader {
            buf: b"hello\nmy \xff"
        })
        .unwrap_err(),
        ron::error::SpannedError {
            code: ron::Error::Io(String::from("oh no")),
            position: ron::error::Position { line: 2, col: 4 },
        }
    );
}
