# Rusty Object Notation

[![CI](https://github.com/ron-rs/ron/actions/workflows/ci.yaml/badge.svg)](https://github.com/ron-rs/ron/actions/workflows/ci.yaml)
[![codecov](https://img.shields.io/codecov/c/github/ron-rs/ron/codecov?token=x4Q5KA51Ul)](https://codecov.io/gh/ron-rs/ron)
[![Crates.io](https://img.shields.io/crates/v/ron.svg)](https://crates.io/crates/ron)
[![MSRV](https://img.shields.io/badge/MSRV-1.64.0-orange)](https://github.com/ron-rs/ron)
[![Docs](https://docs.rs/ron/badge.svg)](https://docs.rs/ron)
[![Matrix](https://img.shields.io/matrix/ron-rs:matrix.org.svg)](https://matrix.to/#/#ron-rs:matrix.org)

RON is a simple readable data serialization format that looks similar to Rust syntax.
It's designed to support all of [Serde's data model](https://serde.rs/data-model.html), so
structs, enums, tuples, arrays, generic maps, and primitive values.

## Example

```rust,ignore
GameConfig( // optional struct name
    window_size: (800, 600),
    window_title: "PAC-MAN",
    fullscreen: false,
    
    mouse_sensitivity: 1.4,
    key_bindings: {
        "up": Up,
        "down": Down,
        "left": Left,
        "right": Right,
        
        // Uncomment to enable WASD controls
        /*
        "W": Up,
        "A": Down,
        "S": Left,
        "D": Right,
        */
    },
    
    difficulty_options: (
        start_difficulty: Easy,
        adaptive: false,
    ),
)
```

## Why RON?

### Example in JSON

```json
{
   "materials": {
        "metal": {
            "reflectivity": 1.0
        },
        "plastic": {
            "reflectivity": 0.5
        }
   },
   "entities": [
        {
            "name": "hero",
            "material": "metal"
        },
        {
            "name": "monster",
            "material": "plastic"
        }
   ]
}
```

### Same example in RON

```rust,ignore
Scene( // class name is optional
    materials: { // this is a map
        "metal": (
            reflectivity: 1.0,
        ),
        "plastic": (
            reflectivity: 0.5,
        ),
    },
    entities: [ // this is an array
        (
            name: "hero",
            material: "metal",
        ),
        (
            name: "monster",
            material: "plastic",
        ),
    ],
)
```

Note the following advantages of RON over JSON:

* trailing commas allowed
* single- and multi-line comments
* field names aren't quoted, so it's less verbose
* optional struct names improve readability
* enums are supported (and less verbose than their JSON representation)

## Limitations

RON is not designed to be a fully self-describing format (unlike JSON) and is thus not guaranteed to work when [`deserialize_any`](https://docs.rs/serde/latest/serde/trait.Deserializer.html#tymethod.deserialize_any) is used instead of its typed alternatives. In particular, the following Serde attributes only have limited support:

- `#[serde(tag = "tag")]`, i.e. internally tagged enums [^serde-enum-hack]
- `#[serde(tag = "tag", content = "content")]`, i.e. adjacently tagged enums [^serde-enum-hack]
- `#[serde(untagged)]`, i.e. untagged enums [^serde-enum-hack]
- `#[serde(flatten)]`, i.e. flattening of structs into maps [^serde-flatten-hack]

While data structures with any of these attributes should roundtrip through RON, their textual representation may not always match your expectation. For instance, flattened structs are only serialised as maps and deserialised from maps.

[^serde-enum-hack]: Deserialising an internally, adjacently, or un-tagged enum requires detecting `serde`'s internal `serde::__private::de::content::Content` content type so that RON can describe the deserialised data structure in serde's internal JSON-like format. This detection only works for the automatically-derived [`Deserialize`](https://docs.rs/serde/latest/serde/de/trait.Deserialize.html) impls on enums. See [#451](https://github.com/ron-rs/ron/pull/451) for more details.
[^serde-flatten-hack]: Deserialising a flattened struct from a map requires that the struct's [`Visitor::expecting`](https://docs.rs/serde/latest/serde/de/trait.Visitor.html#tymethod.expecting) implementation formats a string starting with `"struct "`. This is the case for automatically-derived [`Deserialize`](https://docs.rs/serde/latest/serde/de/trait.Deserialize.html) impls on structs. See [#455](https://github.com/ron-rs/ron/pull/455) for more details.

## RON syntax overview

* Numbers: `42`, `3.14`, `0xFF`, `0b0110`
* Strings: `"Hello"`, `"with\\escapes\n"`, `r#"raw string, great for regex\."#`
* Booleans: `true`, `false`
* Chars: `'e'`, `'\n'`
* Optionals: `Some("string")`, `Some(Some(1.34))`, `None`
* Tuples: `("abc", 1.23, true)`, `()`
* Lists: `["abc", "def"]`
* Structs: `( foo: 1.0, bar: ( baz: "I'm nested" ) )`
* Maps: `{ "arbitrary": "keys", "are": "allowed" }`

> **Note:** Serde's data model represents fixed-size Rust arrays as tuple (instead of as list)

## Quickstart

### `Cargo.toml`

```toml
[dependencies]
ron = "0.8"
serde = { version = "1", features = ["derive"] }
```

### `main.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct MyStruct {
    boolean: bool,
    float: f32,
}

fn main() {
    let x: MyStruct = ron::from_str("(boolean: true, float: 1.23)").unwrap();
    
    println!("RON: {}", ron::to_string(&x).unwrap());

    println!("Pretty RON: {}", ron::ser::to_string_pretty(
        &x, ron::ser::PrettyConfig::default()).unwrap(),
    );
}
```

## Tooling

| Editor       | Plugin                                                      |
| ------------ | ----------------------------------------------------------- |
| IntelliJ     | [intellij-ron](https://github.com/ron-rs/intellij-ron)      |
| VS Code      | [a5huynh/vscode-ron](https://github.com/a5huynh/vscode-ron) |
| Sublime Text | [RON](https://packagecontrol.io/packages/RON)               |
| Atom         | [language-ron](https://atom.io/packages/language-ron)       |
| Vim          | [ron-rs/ron.vim](https://github.com/ron-rs/ron.vim)         |
| EMACS        | [emacs-ron]                                                 |

[emacs-ron]: https://chiselapp.com/user/Hutzdog/repository/ron-mode/home

## Specification

There is a very basic, work in progress specification available on
[the wiki page](https://github.com/ron-rs/ron/wiki/Specification).
A more formal and complete grammar is available [here](docs/grammar.md).


## License

RON is dual-licensed under Apache-2.0 and MIT.

Any contribution intentionally submitted for inclusion in the work must be provided under the same dual-license terms.
