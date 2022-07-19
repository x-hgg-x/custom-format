# custom-format

[![version](https://img.shields.io/crates/v/custom-format?color=blue&style=flat-square)](https://crates.io/crates/custom-format)
[![Documentation](https://docs.rs/custom-format/badge.svg)](https://docs.rs/custom-format)

This crate extends the standard formatting syntax with custom format specifiers, by providing custom formatting macros.

It uses ` :` (a space and a colon) as a separator before the format specifier, which is not a syntax currently accepted and allows supporting standard specifiers in addition to custom specifiers.

## Example

```rust
use custom_format::{CustomFormat, CustomFormatter};

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
            // %Y  - Year with pad for at least 4 digits
            "%Y" => write!(f, "{:04}", self.year),
            // %y  - Year % 100 (00..99)
            "%y" => write!(f, "{:02}", (self.year % 100).abs()),
            // %m  - Month of the year, zero-padded (01..12)
            "%m" => write!(f, "{:02}", self.month),
            // %d  - Day of the month, zero-padded (01..31)
            "%d" => write!(f, "{:02}", self.month_day),
            // %H  - Hour of the day, 24-hour clock, zero-padded (00..23)
            "%H" => write!(f, "{:02}", self.hour),
            // %M  - Minute of the hour (00..59)
            "%M" => write!(f, "{:02}", self.minute),
            // %S  - Second of the minute (00..60)
            "%S" => write!(f, "{:02}", self.second),
            // %9N - Nanosecond (9 digits)
            "%9N" => write!(f, "{:09}", self.nanoseconds),
            // %D  - Date (%m/%d/%y)
            "%D" => write!(f, "{}/{}/{}", CustomFormatter::new(self, "%m"), CustomFormatter::new(self, "%d"), CustomFormatter::new(self, "%y")),
            // %F  - The ISO 8601 date format (%Y-%m-%d)
            "%F" => write!(f, "{}-{}-{}", CustomFormatter::new(self, "%Y"), CustomFormatter::new(self, "%m"), CustomFormatter::new(self, "%d")),
            // %T  - 24-hour time (%H:%M:%S)
            "%T" => write!(f, "{}:{}:{}", CustomFormatter::new(self, "%H"), CustomFormatter::new(self, "%M"), CustomFormatter::new(self, "%S")),
            // Incorrect format
            _ => Err(fmt::Error),
        }
    }
}

fn main() {
    let date_time = DateTime { year: 1836, month: 5, month_day: 18, hour: 23, minute: 45, second: 54, nanoseconds: 123456789 };

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
    custom_format::println!("{1}: {0 :%Y}-{0 :%m}-{0 :%d} {0 :%H}:{0 :%M}:{0 :%S}.{0 :%9N}", date_time, "The date time is");
}
```
