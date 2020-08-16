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

# enum_repr

You can add this extension by adding the following attribute at the top of your RON document:

`#![enable(enum_repr)]`

This feature enables RON to declare unitary enums with numerical representations,
There are a number of limitations to this feature:

```rust
#[repr(u8)]
enum Utensils {
  Bowl,
  Whisk
}

#[repr(u64)]
enum Ingredients {
  Sugar,
  EggWhites
}
```

Where the rust portion gets passed to `ron::parse::TypeEnv`

```ron
Recipe("meringue": [([Whisk, Bowl], [EggWhites, Sugar])])
```

From there when you pass the TypeEnv parsed above to `ron::de::with_type_env`.
You can decode the RON above using the compiled rust:

```rust
struct Recipe<String, Vec<(Vec<u8>, Vec<u64>)>>;
```

There is some support for prepending the rust code block to the RON code block,
parsing both from the same string.

Known limitations (Not expected to change):

1. Enum declarations must *only* contain unitary variants.
  enums with variants such as `Foo(String)` can not be converted into a primitive representation.
2. Declarations are only supported at the top of the file between enabling of the extension, and values.
 Once a value has been defined, declaring an enum is a parse error.
 As such this extension may not be useful for some streaming
3. Deserialization does not perform type checking between 2 enums of the same representation, or numeric literals[1].
 This however does not preclude having an external type checker in the future.

WIP/TODO:

- [x] Proof of concept
- [x] Parsing
- [ ] [Custom discriminant](https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-fieldless-enumerations)
- [x] Deserialization
- [x] Feature gate (enum-repr-extension)
- [ ] Serialization?
