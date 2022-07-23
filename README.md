# custom-format

[![version](https://img.shields.io/crates/v/custom-format?color=blue&style=flat-square)](https://crates.io/crates/custom-format)
[![Documentation](https://docs.rs/custom-format/badge.svg)](https://docs.rs/custom-format)

This crate extends the standard formatting syntax with custom format specifiers, by providing custom formatting macros.

It uses ` :` (a space and a colon) as a separator before the format specifier, which is not a syntax currently accepted and allows supporting standard specifiers in addition to custom specifiers.

This library comes in two flavors:

* With the `compile-time` feature, the set of possible custom format specifiers is defined at compilation, so invalid specifiers can be checked at compile-time.
This allows the library to have the same performance as when using the standard library formatting traits.

* With the `runtime` feature, the formatting method dynamically checks the format specifier at runtime for each invocation. This is a slower version, but has a lower MSRV for greater compatibility.

## Example with the `compile-time` feature

<details>
<summary>Code</summary>

```rust
use custom_format::compile_time as cfmt;
use custom_format::custom_formatter;

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
        $(
            impl cfmt::CustomFormat<{ cfmt::spec($spec) }> for DateTime {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    ($func as fn(&Self, &mut fmt::Formatter) -> fmt::Result)(self, f)
                }
            }
        )*
    };
}

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
    // Nanosecond (9 digits)
    "%9N" => |this, f| write!(f, "{:09}", this.nanoseconds),
    // Date (%m/%d/%y)
    "%D" => {
        |this, f| {
            let month = custom_formatter!("%m", this);
            let day = custom_formatter!("%d", this);
            let year = custom_formatter!("%y", this);
            write!(f, "{}/{}/{}", month, day, year)
        }
    }
    // The ISO 8601 date format (%Y-%m-%d)
    "%F" => {
        |this, f| {
            let year = custom_formatter!("%Y", this);
            let month = custom_formatter!("%m", this);
            let day = custom_formatter!("%d", this);
            write!(f, "{}-{}-{}", year, month, day)
        }
    }
    // 24-hour time (%H:%M:%S)
    "%T" => {
        |this, f| {
            let hour = custom_formatter!("%H", this);
            let minute = custom_formatter!("%M", this);
            let second = custom_formatter!("%S", this);
            write!(f, "{}:{}:{}", hour, minute, second)
        }
    }
});

let date_time = DateTime {
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
// match (&(date_time), &("The date time is")) {
//     (arg0, arg1) => {
//         ::std::println!(
//             "{0}: {1}-{2}-{3} {4}:{5}:{6}.{7}",
//             arg1,
//             ::custom_format::custom_formatter!("%Y", arg0),
//             ::custom_format::custom_formatter!("%m", arg0),
//             ::custom_format::custom_formatter!("%d", arg0),
//             ::custom_format::custom_formatter!("%H", arg0),
//             ::custom_format::custom_formatter!("%M", arg0),
//             ::custom_format::custom_formatter!("%S", arg0),
//             ::custom_format::custom_formatter!("%9N", arg0)
//         )
//     }
// };
//
// Output: "The date time is: 1836-05-18 23:45:54.123456789"
//
cfmt::println!(
    "{1}: {0 :%Y}-{0 :%m}-{0 :%d} {0 :%H}:{0 :%M}:{0 :%S}.{0 :%9N}",
    date_time,
    "The date time is"
);

// Compile-time error since "%h" is not a valid format specifier
// cfmt::println!("{0 :%h}", date_time);
```

</details>

## Example with the `runtime` feature

<details>
<summary>Code</summary>

```rust
use custom_format::runtime::{self as cfmt, CustomFormat, CustomFormatter};

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

impl CustomFormat for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter, spec: &str) -> fmt::Result {
        match spec {
            // Year with pad for at least 4 digits
            "%Y" => write!(f, "{:04}", self.year),
            // Year % 100 (00..99)
            "%y" => write!(f, "{:02}", (self.year % 100).abs()),
            // Month of the year, zero-padded (01..12)
            "%m" => write!(f, "{:02}", self.month),
            // Day of the month, zero-padded (01..31)
            "%d" => write!(f, "{:02}", self.month_day),
            // Hour of the day, 24-hour clock, zero-padded (00..23)
            "%H" => write!(f, "{:02}", self.hour),
            // Minute of the hour (00..59)
            "%M" => write!(f, "{:02}", self.minute),
            // Second of the minute (00..60)
            "%S" => write!(f, "{:02}", self.second),
            // Nanosecond (9 digits)
            "%9N" => write!(f, "{:09}", self.nanoseconds),
            // Date (%m/%d/%y)
            "%D" => {
                let month = CustomFormatter::new("%m", self);
                let day = CustomFormatter::new("%d", self);
                let year = CustomFormatter::new("%y", self);
                write!(f, "{}/{}/{}", month, day, year)
            }
            // The ISO 8601 date format (%Y-%m-%d)
            "%F" => {
                let year = CustomFormatter::new("%Y", self);
                let month = CustomFormatter::new("%m", self);
                let day = CustomFormatter::new("%d", self);
                write!(f, "{}-{}-{}", year, month, day)
            }
            // 24-hour time (%H:%M:%S)
            "%T" => {
                let hour = CustomFormatter::new("%H", self);
                let minute = CustomFormatter::new("%M", self);
                let second = CustomFormatter::new("%S", self);
                write!(f, "{}:{}:{}", hour, minute, second)
            }
            // Invalid format specifier
            _ => Err(fmt::Error),
        }
    }
}

let date_time = DateTime {
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
// match (&(date_time), &("The date time is")) {
//     (arg0, arg1) => {
//         ::std::println!(
//             "{0}: {1}-{2}-{3} {4}:{5}:{6}.{7}",
//             arg1,
//             ::custom_format::CustomFormatter::new(arg0, "%Y"),
//             ::custom_format::CustomFormatter::new(arg0, "%m"),
//             ::custom_format::CustomFormatter::new(arg0, "%d"),
//             ::custom_format::CustomFormatter::new(arg0, "%H"),
//             ::custom_format::CustomFormatter::new(arg0, "%M"),
//             ::custom_format::CustomFormatter::new(arg0, "%S"),
//             ::custom_format::CustomFormatter::new(arg0, "%9N")
//         )
//     }
// }
//
// Output: "The date time is: 1836-05-18 23:45:54.123456789"
//
cfmt::println!(
    "{1}: {0 :%Y}-{0 :%m}-{0 :%d} {0 :%H}:{0 :%M}:{0 :%S}.{0 :%9N}",
    date_time,
    "The date time is"
);

// Panic at runtime since "%h" is not a valid format specifier
// cfmt::println!("{0 :%h}", date_time);
```

</details>

## License

This project is licensed under either of

- [Apache License, Version 2.0](https://github.com/x-hgg-x/custom-format/blob/master/LICENSE-Apache)
- [MIT license](https://github.com/x-hgg-x/custom-format/blob/master/LICENSE-MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
