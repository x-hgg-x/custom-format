fn main() {
    #[cfg(feature = "compile-time")]
    {
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

        let date_time = DateTime { year: 1836, month: 5, month_day: 18, hour: 23, minute: 45, second: 54, nanoseconds: 123456789 };

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
        // }
        //
        // Output: "The date time is: 1836-05-18 23:45:54.123456789"
        //
        cfmt::println!("{prefix}: {0 :%Y}-{0 :%m}-{0 :%d} {0 :%H}:{0 :%M}:{0 :%S}.{0 :%9N}", date_time, prefix = "The date time is");

        // Compile-time error since "%h" is not a valid format specifier
        // cfmt::println!("{0 :%h}", date_time);
    }
}
