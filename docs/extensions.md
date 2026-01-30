## RON extensions

RON has extensions that can be enabled by adding the following attribute at the top of your RON document:

`#![enable(...)]`

RON also accepts the following document attributes for compatibility with tooling like `ron2`.
They are parsed and ignored by this crate:

`#![type = "path::To::Type"]`

`#![schema = "./path/to/schema.ron"]`

# unwrap_newtypes

You can add this extension by adding the following attribute at the top of your RON document:

`#![enable(unwrap_newtypes)]`

This feature enables RON to automatically unwrap simple tuples.

```rust
struct NewType(u32);
struct Object {
    pub new_type: NewType,
}
```

Without `unwrap_newtypes`, because the value `5` can not be saved into `NewType(u32)`, your RON document would look like this:

```ron
(
    new_type: (5),
)
```

With the `unwrap_newtypes` extension, this coercion is done automatically. So `5` will be interpreted as `(5)`.

```ron
#![enable(unwrap_newtypes)]
(
    new_type: 5,
)
```

# implicit_some

You can add this extension by adding the following attribute at the top of your RON document:

`#![enable(implicit_some)]`

This feature enables RON to automatically convert any value to `Some(value)` if the deserialized type requires it.

```rust
struct Object {
    pub value: Option<u32>,
}
```

Without this feature, you would have to write this RON document.

```ron
(
    value: Some(5),
)
```

Enabling the feature would automatically infer `Some(x)` if `x` is given. In this case, RON automatically casts this `5` into a `Some(5)`.

```ron
(
    value: 5,
)
```

With this extension enabled, explicitly given `None` and `Some(..)` will be matched eagerly on `Option<Option<Option<u32>>>`, i.e.
* `5` -> `Some(Some(Some(5)))`
* `None` -> `None`
* `Some(5)` -> `Some(Some(Some(5)))`
* `Some(None)` -> `Some(None)`
* `Some(Some(5))` -> `Some(Some(Some(5)))`
* `Some(Some(None))` -> `Some(Some(None))`
* `Some(Some(Some(5)))` -> `Some(Some(Some(5)))`

# unwrap_variant_newtypes

You can add this extension by adding the following attribute at the top of your RON document:

`#![enable(unwrap_variant_newtypes)]`

This feature enables RON to automatically unwrap newtype enum variants.

```rust
#[derive(Deserialize)]
struct Inner {
    pub a: u8,
    pub b: bool,
}
#[derive(Deserialize)]
pub enum Enum {
    A(Inner),
    B,
}
```

Without `unwrap_variant_newtypes`, your RON document would look like this:

```ron
(
    variant: A(Inner(a: 4, b: true)),
)
```

With the `unwrap_variant_newtypes` extension, the first structural layer inside a newtype variant will be unwrapped automatically:

```ron
#![enable(unwrap_newtypes)]
(
    variant: A(a: 4, b: true),
)
```

Note that when the `unwrap_variant_newtypes` extension is enabled, the first layer inside a newtype variant will **always** be unwrapped, i.e. it is no longer possible to write `A(Inner(a: 4, b: true))` or `A((a: 4, b: true))`.

# explicit_struct_names
During serialization, this extension emits struct names. For instance, this would be emitted:
```ron
Foo(
    bar: Bar(42),
)
```

During deserialization, this extension requires that all structs have names attached to them. For example, the following deserializes perfectly fine:
```ron
Foo(
    bar: Bar(42),
)
```

However, with the `explicit_struct_names` extension enabled, the following will throw an `ExpectedStructName` error:
```ron
(
    bar: Bar(42),
)
```

Similarly, the following will throw the same error:
```ron
Foo(
    bar: (42),
)
```

Note that if what you are parsing is spread across many files, you would likely use `Options::with_default_extension` to enable `Extensions::EXPLICIT_STRUCT_NAMES` before the parsing stage. This is because prepending `#![enable(explicit_struct_names)]` to the contents of every file you parse would violate DRY (Don't Repeat Yourself).

Here is an example of how to enable `explicit_struct_names` using this method:
```rust
use ron::extensions::Extensions;
use ron::options::Options;

// Setup the options
let options = Options::default().with_default_extension(Extensions::EXPLICIT_STRUCT_NAMES);
// Retrieve the contents of the file
let file_contents: &str = /* ... */;
// Parse the file's contents
let foo: Foo = options.from_str(file_contents)?;
```
