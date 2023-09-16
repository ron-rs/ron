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
ws = { ws_single | comment };
ws_single = "\n" | "\t" | "\r" | " " | U+000B | U+000C | U+0085 | U+200E | U+200F | U+2028 | U+2029;
comment = ["//", { no_newline }, "\n"] | ["/*", nested_block_comment, "*/"];
nested_block_comment = { ? any characters except "/*" or "*/" ? }, [ "/*", nested_block_comment, "*/", nested_block_comment ];
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
value = integer | byte | float | string | byte_string | char | bool | option | list | map | tuple | struct | enum_variant;
```

## Numbers

```ebnf
digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
digit_binary = "0" | "1";
digit_octal = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7";
digit_hexadecimal = digit | "A" | "a" | "B" | "b" | "C" | "c" | "D" | "d" | "E" | "e" | "F" | "f";

integer = ["+" | "-"], unsigned, [integer_suffix];
integer_suffix = ("i", "u"), ("8", "16", "32", "64", "128");

unsigned = unsigned_binary | unsigned_octal | unsigned_hexadecimal | unsigned_decimal;
unsigned_binary = "0b", digit_binary, { digit_binary | "_" };
unsigned_octal = "0o", digit_octal, { digit_octal | "_" };
unsigned_hexadecimal = "0x", digit_hexadecimal, { digit_hexadecimal | "_" };
unsigned_decimal = digit, { digit | "_" };

byte = "b", "'", byte_content, "'";
byte_content = ascii | ("\\", (escape_ascii | escape_byte));

float = ["+" | "-"], ("inf" | "NaN" | float_num), [float_suffix];
float_num = (float_int | float_std | float_frac), [float_exp];
float_int = digit, { digit | "_" };
float_std = digit, { digit | "_" }, ".", [digit, { digit | "_" }];
float_frac = ".", digit, { digit | "_" };
float_exp = ("e" | "E"), ["+" | "-"], { digit | "_" }, digit, { digit | "_" };
float_suffix = "f", ("32", "64");
```

> Note: `ascii` refers to any ASCII character, i.e. any byte in range `0x00 ..= 0x7F`.

## String

```ebnf
string = string_std | string_raw;
string_std = "\"", { no_double_quotation_marks | string_escape }, "\"";
string_escape = "\\", (escape_ascii | escape_byte | escape_unicode);
string_raw = "r", string_raw_content;
string_raw_content = ("#", string_raw_content, "#") | "\"", { unicode_non_greedy }, "\"";

escape_ascii = "'" | "\"" | "\\" | "n" | "r" | "t" | "0";
escape_byte = "x", digit_hexadecimal, digit_hexadecimal;
escape_unicode = "u", digit_hexadecimal, [digit_hexadecimal, [digit_hexadecimal, [digit_hexadecimal, [digit_hexadecimal, [digit_hexadecimal]]]]];
```

> Note: Raw strings start with an `r`, followed by n `#`s and a quotation mark
  `"`. They may contain any characters or escapes (except the end sequence).
  A raw string ends with a quotation mark (`"`), followed by n `#`s. n may be
  any number, including zero.
  Example:
  ```rust
r##"This is a "raw string". It can contain quotations or
backslashes (\)!"##
  ```
Raw strings cannot be written in EBNF, as they are context-sensitive.
Also see [the Rust document] about context-sensitivity of raw strings.

[the Rust document]: https://github.com/rust-lang/rust/blob/d046ffddc4bd50e04ffc3ff9f766e2ac71f74d50/src/grammar/raw-string-literal-ambiguity.md

## Byte String

```ebnf
byte_string = byte_string_std | byte_string_raw;
byte_string_std = "b\"", { no_double_quotation_marks | string_escape }, "\"";
byte_string_raw = "br", string_raw_content;
```

> Note: Byte strings are similar to normal strings but are not required to
  contain only valid UTF-8 text. RON's byte strings follow the updated Rust
  byte string literal rules as proposed in [RFC #3349], i.e. byte strings
  allow the exact same characters and escape codes as normal strings.

[RFC #3349](https://github.com/rust-lang/rfcs/pull/3349)

> Note: Raw byte strings start with an `br` prefix and follow the same rules
  as raw strings, which are outlined above.

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
option = "None" | option_some;
option_some = "Some", ws, "(", ws, value, ws, ")";
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
named_struct = [ident], ws, "(", ws, [named_field, { comma, named_field }, [comma]], ")";
named_field = ident, ws, ":", ws, value;
```

## Enum

```ebnf
enum_variant = enum_variant_unit | enum_variant_tuple | enum_variant_named;
enum_variant_unit = ident;
enum_variant_tuple = ident, ws, tuple;
enum_variant_named = ident, ws, "(", [named_field, { comma, named_field }, [comma]], ")";
```

## Identifier

```ebnf
ident = ident_std | ident_raw;
ident_std = ident_std_first, { ident_std_rest };
ident_std_first = XID_Start | "_";
ident_std_rest = XID_Continue;
ident_raw = "r", "#", ident_raw_rest, { ident_raw_rest };
ident_raw_rest = ident_std_rest | "." | "+" | "-";
```

> Note: [XID_Start](http://unicode.org/cldr/utility/list-unicodeset.jsp?a=%5B%3AXID_Start%3A%5D&abb=on&g=&i=) and [XID_Continue](http://unicode.org/cldr/utility/list-unicodeset.jsp?a=%5B%3AXID_Continue%3A%5D&abb=on&g=&i=) refer to Unicode character sets.
