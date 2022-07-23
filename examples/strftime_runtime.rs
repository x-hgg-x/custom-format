fn main() {
    #[cfg(feature = "runtime")]
    {
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

        let date_time = DateTime { year: 1836, month: 5, month_day: 18, hour: 23, minute: 45, second: 54, nanoseconds: 123456789 };

        // Expands to:
        //
        // match (&(date_time), &("The date time is")) {
        //     (arg0, arg1) => {
        //         ::std::println!(
        //             "{0}: {1}-{2}-{3} {4}:{5}:{6}.{7}",
        //             arg1,
        //             ::custom_format::runtime::CustomFormatter::new("%Y", arg0),
        //             ::custom_format::runtime::CustomFormatter::new("%m", arg0),
        //             ::custom_format::runtime::CustomFormatter::new("%d", arg0),
        //             ::custom_format::runtime::CustomFormatter::new("%H", arg0),
        //             ::custom_format::runtime::CustomFormatter::new("%M", arg0),
        //             ::custom_format::runtime::CustomFormatter::new("%S", arg0),
        //             ::custom_format::runtime::CustomFormatter::new("%9N", arg0)
        //         )
        //     }
        // }
        //
        // Output: "The date time is: 1836-05-18 23:45:54.123456789"
        //
        cfmt::println!("{prefix}: {0 :%Y}-{0 :%m}-{0 :%d} {0 :%H}:{0 :%M}:{0 :%S}.{0 :%9N}", date_time, prefix = "The date time is");

        // Panic at runtime since "%h" is not a valid format specifier
        // cfmt::println!("{0 :%h}", date_time);
    }
}
