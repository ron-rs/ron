[![Build Status](https://travis-ci.org/kvark/ron.png?branch=master)](https://travis-ci.org/kvark/ron)
[![Docs](https://docs.rs/ron/badge.svg)](https://docs.rs/ron)
[![Crates.io](https://img.shields.io/crates/v/ron.svg?maxAge=2592000)](https://crates.io/crates/ron)
## Rusty Object Notation

RON is a simple readable data serialization format that looks like Rust. It's designed to support structs, enums, tuples, arrays, generic maps, and primitive values.

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
            "name": "moster",
            "material": "plastic"
        }
   ]
}
```

Notice these issues:
  1. Struct and maps are the same
    - random order of exported fields
      - annoying and inconvenient for reading
      - doesn't work well with version control
    - quoted field names
      - too verbose
    - no support for enums
  2. No trailing comma allowed
  3. No comments allowed

### Same example in RON

```rust
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

The new format uses `(`..`)` brackets for *heterogeneous* structures (classes), while preserving the `{`..`}` for maps, and `[`..`]` for *homogeneous* structures (arrays). This distinction allows to solve the biggest problem with JSON.

Here are the general rules to parse the heterogeneous structures:

| class is named? | fields are named? | what is it?               | example           |
| --------------- | ------------------| ------------------------- | ----------------- |
| no              | no                | tuple / tuple struct      | `(a, b)`          |
| yes             | no                | enum value / tuple struct | `Name(a, b)`      |
| yes/no          | yes               | struct                    | `(f1: a, f2: b)`  |

### Grammar
```
element:
   struct
   array
   map
   constant

constant:
   string
   number
   boolean

map:
   `{` key1: value1, key2: value2, ... `}`
   // where all keys are constants of the same type
   // and all values are elements of the same type 

array:
   `[` elem1, elem2, ... `]`
   // where all elements are of the same type

struct:
   [Name] `(` field1: elem1, field2: elem2, ... `)`
```

### Appendix

Why not XML?
  - too verbose
  - unclear how to treat attributes vs contents

Why not YAML?
  - significant white-space 
  - specification is too big

Why not TOML?
  - alien syntax
  - absolute paths are not scalable

Why not XXX?
  - if you know a better format, tell me!
