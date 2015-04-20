# Rusty Object Notation

## Background

I have a scene [exporter](https://github.com/kvark/claymore/blob/master/etc/blender/io_kri_scene/scene.py) from Blender, where the result is loaded by the Rust [code](https://github.com/kvark/claymore/blob/master/src/load/scene.rs). The scene structure I'd like to see with my eyes, thus text form is preferred, while mesh contents and animation curves are passed in a custom binary format. I used JSON for the scene format, since it's been well-supported in Rust, but it appeared to have seveal issues. Here I propose another textual self-describing format to replace JSON.

## Example in JSON

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

Notice these problems:
  1. Struct and maps are the same
    - random order of exported fields
      - annoying and inconvenient for reading
      - doesn't work well with version control
    - quoted field names
      - too verbose
    - no support for enums
  2. No trailing comma allowed
  3. No comments allowed

## Same example in RON

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

The new format uses `(` and `)` brackets for heterogeneous structures (classes), while preserving the `{` and `}` for maps. This distinction allows to solve the biggest problem with JSON.

Here are the general rules to parse the heterogeneous structures:

| class is named? | fields are named? | what is it?               | example           |
| --------------- | ------------------| ------------------------- | ----------------- |
| no              | no                | tuple / tuple struct      | `(a, b)`          |
| yes             | no                | enum value / tuple struct | `Name(a, b)`      |
| yes/no          | yes               | struct                    | `(f1: a, f2: b)`  |

## Grammar
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
   // where all alements are of the same type

struct:
   [Name] `(` field1: elem1, field2: elem2, ... `)`
```

## Appendix

Why not XML?
  - too verbose
  - unclear how to treat attributes vs contents

Why not YAML?
  - significant white-space 
  - specification is too big

Why not XXX?
  - if you know a better format, tell me!
