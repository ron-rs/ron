/*!
RON is a simple config format which looks similar to Rust syntax.

## Features

* Data types
    * Structs, typename optional
    * Tuples
    * Enums
    * Lists
    * Maps
    * Units (`()`)
    * Optionals
    * Primitives: booleans, numbers, string, char
* Allows nested layout (similar to JSON)
* Supports comments
* Trailing commas
* Pretty serialization

## Syntax example

```rust,ignore
Game(
    title: "Hello, RON!",
    level: Level( // We could just leave the `Level` out
        buildings: [
            (
                size: (10, 20),
                color: Yellow, // This as an enum variant
                owner: None,
            ),
            (
                size: (20, 25),
                color: Custom(0.1, 0.8, 1.0),
                owner: Some("guy"),
            ),
        ],
        characters: {
            "guy": (
                friendly: true,
            ),
        },
    ),
)
```

## Usage

Just add it to your `Cargo.toml`:

```toml
[dependencies]
ron = "*"
```

Serializing / Deserializing is as simple as calling `to_string` / `from_str`.

!*/

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate serde;

pub mod de;
pub mod ser;
pub mod value;

mod parse;

// Export the following so dependent crates would be able to use this in
// type constraints without depending directly on ron's version of serde.

pub use serde::de::DeserializeOwned;
