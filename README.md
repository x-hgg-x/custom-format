# custom-format

[![version](https://img.shields.io/crates/v/custom-format?color=blue&style=flat-square)](https://crates.io/crates/custom-format)
![Minimum supported Rust version](https://img.shields.io/badge/rustc-1.85+-important?logo=rust "Minimum Supported Rust Version")
[![Documentation](https://docs.rs/custom-format/badge.svg)](https://docs.rs/custom-format)

This crate extends the standard formatting syntax with custom format specifiers, by providing custom formatting macros.

It uses ` :` (a space and a colon) as a separator before the format specifier, which is not a syntax currently accepted and allows supporting standard specifiers in addition to custom specifiers. It also supports [format args capture](https://blog.rust-lang.org/2022/01/13/Rust-1.58.0.html#captured-identifiers-in-format-strings) even on older versions of Rust, since it manually adds the named parameter if missing.

This library comes in two flavors, corresponding to the following features:

- `compile-time` (*enabled by default*)

    The set of possible custom format specifiers is defined at compilation, so invalid specifiers can be checked at compile-time.
    This allows the library to have the same performance as when using the standard library formatting traits.

- `runtime` (*enabled by default*)

    The formatting method dynamically checks the format specifier at runtime for each invocation.
    This is a slower version, but it has additional flexibility.

## Documentation

Documentation is hosted on [docs.rs](https://docs.rs/custom-format/latest/).

## Example

<details>
<summary>Code</summary>

```rust
use custom_format as cfmt;

use core::fmt;

pub struct DateTime {
    year: i32,
    month: u8,
    month_day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    nanoseconds: u32,
}

macro_rules! impl_custom_format_for_datetime {
    (match spec { $($spec:literal => $func:expr $(,)?)* }) => {
        use cfmt::compile_time::{spec, CustomFormat};
        $(
            impl CustomFormat<{ spec($spec) }> for DateTime {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    ($func as fn(&Self, &mut fmt::Formatter) -> fmt::Result)(self, f)
                }
            }
        )*
    };
}

// Static format specifiers, checked at compile-time
impl_custom_format_for_datetime!(match spec {
    // Year with pad for at least 4 digits
    "%Y" => |this, f| write!(f, "{:04}", this.year),
    // Year % 100 (00..99)
    "%y" => |this, f| write!(f, "{:02}", (this.year % 100).abs()),
    // Month of the year, zero-padded (01..12)
    "%m" => |this, f| write!(f, "{:02}", this.month),
    // Day of the month, zero-padded (01..31)
    "%d" => |this, f| write!(f, "{:02}", this.month_day),
    // Hour of the day, 24-hour clock, zero-padded (00..23)
    "%H" => |this, f| write!(f, "{:02}", this.hour),
    // Minute of the hour (00..59)
    "%M" => |this, f| write!(f, "{:02}", this.minute),
    // Second of the minute (00..60)
    "%S" => |this, f| write!(f, "{:02}", this.second),
    // Date (%m/%d/%y)
    "%D" => {
        |this, f| {
            let month = cfmt::custom_formatter!("%m", this);
            let day = cfmt::custom_formatter!("%d", this);
            let year = cfmt::custom_formatter!("%y", this);
            write!(f, "{}/{}/{}", month, day, year)
        }
    }
});

// Dynamic format specifiers, checked at runtime
impl cfmt::runtime::CustomFormat for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter, spec: &str) -> fmt::Result {
        let mut chars = spec.chars();
        match (chars.next(), chars.next_back()) {
            // Nanoseconds with n digits (%nN)
            (Some('%'), Some('N')) => match chars.as_str().parse() {
                Ok(n) if n > 0 => {
                    if n <= 9 {
                        write!(f, "{:0width$}", self.nanoseconds / 10u32.pow(9 - n as u32), width = n)
                    } else {
                        write!(f, "{:09}{:0width$}", self.nanoseconds, 0, width = n - 9)
                    }
                }
                _ => Err(fmt::Error),
            },
            _ => Err(fmt::Error),
        }
    }
}

let dt = DateTime {
    year: 1836,
    month: 5,
    month_day: 18,
    hour: 23,
    minute: 45,
    second: 54,
    nanoseconds: 123456789,
};

// Expands to:
//
// match (&("DateTime"), &dt) {
//     (arg0, arg1) => ::std::println!(
//         "The {0:?} is: {1}-{2}-{3} {4}:{5}:{6}.{7}",
//         arg0,
//         ::custom_format::custom_formatter!("%Y", arg1),
//         ::custom_format::custom_formatter!("%m", arg1),
//         ::custom_format::custom_formatter!("%d", arg1),
//         ::custom_format::custom_formatter!("%H", arg1),
//         ::custom_format::custom_formatter!("%M", arg1),
//         ::custom_format::custom_formatter!("%S", arg1),
//         ::custom_format::runtime::CustomFormatter::new("%6N", arg1)
//     ),
// }
//
// Output: `The "DateTime" is: 1836-05-18 23:45:54.123456`
//
// The custom format specifier is interpreted as a compile-time specifier by default,
// or as a runtime specifier if it is inside "<>".
cfmt::println!(
    "The {ty:?} is: {dt :%Y}-{dt :%m}-{dt :%d} {dt :%H}:{dt :%M}:{dt :%S}.{dt :<%6N>}",
    ty = "DateTime",
);

// Compile-time error since "%h" is not a valid format specifier
// cfmt::println!("{0 :%h}", dt);

// Panic at runtime since "%h" is not a valid format specifier
// cfmt::println!("{0 :<%h>}", dt);
```

</details>

## Compiler support

Requires `rustc 1.85+`.

## License

This project is licensed under either of

- [Apache License, Version 2.0](https://github.com/x-hgg-x/custom-format/blob/master/LICENSE-Apache)
- [MIT license](https://github.com/x-hgg-x/custom-format/blob/master/LICENSE-MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
