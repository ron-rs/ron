## RON extensions

RON has extensions that can be enabled by adding the following attribute at the top of your RON document:

`#![enable(...)]`

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

``` ron
(
    new_type: (5),
)
```

With the `unwrap_newtypes` extension, this coercion is done automatically. So `5` will be interpreted as `(5)`.

``` ron
#![enable(unwrap_newtypes)]
(
    new_type: 5,
)
```

# implicit_some

You can add this extension by adding the following attribute at the top of your RON document:

`#![enable(implicit_some)]`

This feature enables RON to automatically convert any value to `Some(value)` if the deserialized struct requires it.

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

# implicit_outmost_struct

You can add this extension by adding the following attribute at the top of your RON document:

`#![enable(implicit_outmost_struct)]`

This feature enables RON to ignore the parentheses of the outmost structure.

```rust
struct Rgb(pub f64, pub f64, pub f64);
struct ImageConfig {
    pub size: (usize, usize),
    pub background: Rgb,
}
```

Without this feature, you would have to write this RON document.

```ron
(
    size: (1920, 1080),
    background: Rgb(1.0, 0.0, 0.0),
)
```

With the `unwrap_newtypes` extension, you are allowed to write it without the outmost parentheses.

```ron
#![enable(implicit_outmost_struct)]

size: (1920, 1080),
background: Rgb(1.0, 0.0, 0.0),
```