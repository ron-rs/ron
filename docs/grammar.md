# RON grammar

This file describes the structure of a RON file in [EBNF notation][ebnf].
If extensions are enabled, some rules will be replaced. For that, see the
[extensions document][exts] which describes all extensions and what they override.

[ebnf]: https://en.wikipedia.org/wiki/Extended_Backus–Naur_form
[exts]: ./extensions.md

## RON file

```ebnf
RON = [extensions], ws, value, ws;
```

## Whitespace and comments

```ebnf
ws = { ws_single, comment };
ws_single = "\n" | "\t" | "\r" | " ";
comment = ["//", { no_newline }, "\n"];
```

## Commas

```ebnf
comma = ws, ",", ws;
```

## Extensions

```ebnf
extensions = { "#", ws, "!", ws, "[", ws, extensions_inner, ws, "]", ws };
extensions_inner = "enable", ws, "(", extension_name, { comma, extension_name }, [comma], ws, ")";
```

For the extension names see the [`extensions.md`][exts] document.

## Value

```ebnf
value = unsigned | signed | float | string | char | bool | option | list | map | tuple | struct | enum_variant;
```

## Numbers

```ebnf
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
unsigned = digit, { digit };
signed = ["+" | "-"], unsigned;
float = float_std | float_frac;
float_std = ["+" | "-"], digit, { digit }, ".", {digit}, [float_exp];
float_frac = ".", digit, {digit}, [float_exp];
float_exp = ("e" | "E"), digit, {digit};
```

## String

```ebnf
string = "\"", { no_double_quotation_marks | string_escape }, "\"";
string_escape = "\\", ("\"" | "\\" | "b" | "f" | "n" | "r" | "t" | ("u", unicode_hex));
```

## Char

```ebnf
char = "'", (no_apostrophe | "\\\\" | "\\'"), "'";
```

## Boolean

```ebnf
bool = "true" | "false";
```

## Optional

```ebnf
option = "Some", ws, "(", ws, value, ws, ")";
```

## List

```ebnf
list = "[", [value, { comma, value }, [comma]], "]";
```

## Map

```ebnf
map = "{", [map_entry, { comma, map_entry }, [comma]], "}";
map_entry = value, ws, ":", ws, value;
```

## Tuple

```ebnf
tuple = "(", [value, { comma, value }, [comma]], ")";
```

## Struct

```ebnf
struct = unit_struct | tuple_struct | named_struct;
unit_struct = ident | "()";
tuple_struct = [ident], ws, tuple;
named_struct = [ident], ws, "(", [named_field, { comma, named_field }, [comma]], ")";
named_field = ident, ws, ":", value;
```

## Enum

```ebnf
enum_variant = enum_variant_unit | enum_variant_tuple | enum_variant_named;
enum_variant_unit = ident;
enum_variant_tuple = ident, ws, tuple;
enum_variant_named = ident, ws, "(", [named_field, { comma, named_field }, [comma]], ")";
```
