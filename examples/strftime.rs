fn main() {
    #[cfg(all(feature = "compile-time", feature = "runtime"))]
    {
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

        let date_time = DateTime { year: 1836, month: 5, month_day: 18, hour: 23, minute: 45, second: 54, nanoseconds: 123456789 };

        // Expands to:
        //
        // match (&(date_time), &("DateTime")) {
        //     (arg0, arg1) => {
        //         ::std::println!(
        //             "The {0:?} is: {1}-{2}-{3} {4}:{5}:{6}.{7}",
        //             arg1,
        //             ::custom_format::custom_formatter!("%Y", arg0),
        //             ::custom_format::custom_formatter!("%m", arg0),
        //             ::custom_format::custom_formatter!("%d", arg0),
        //             ::custom_format::custom_formatter!("%H", arg0),
        //             ::custom_format::custom_formatter!("%M", arg0),
        //             ::custom_format::custom_formatter!("%S", arg0),
        //             ::custom_format::runtime::CustomFormatter::new("%6N", arg0)
        //         )
        //     }
        // }
        //
        // Output: `The "DateTime" is: 1836-05-18 23:45:54.123456`
        //
        // The custom format specifier is interpreted as a compile-time specifier by default, or as a runtime specifier if it is inside "<>".
        cfmt::println!("The {ty:?} is: {0 :%Y}-{0 :%m}-{0 :%d} {0 :%H}:{0 :%M}:{0 :%S}.{0 :<%6N>}", date_time, ty = "DateTime");

        // Compile-time error since "%h" is not a valid format specifier
        // cfmt::println!("{0 :%h}", date_time);

        // Panic at runtime since "%h" is not a valid format specifier
        // cfmt::println!("{0 :<%h>}", date_time);
    }
}
