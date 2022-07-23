//! Provides types associated to runtime formatting.

use core::fmt;

/// Trait for custom formatting with runtime format checking
pub trait CustomFormat {
    /// Formats the value using the given formatter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use custom_format::runtime::{self as cfmt, CustomFormat};
    ///
    /// use core::fmt;
    ///
    /// #[derive(Debug)]
    /// struct Hex(u8);
    ///
    /// impl CustomFormat for Hex {
    ///     fn fmt(&self, f: &mut fmt::Formatter, spec: &str) -> fmt::Result {
    ///         match spec {
    ///             "x" => write!(f, "{:#02x}", self.0),
    ///             "X" => write!(f, "{:#02X}", self.0),
    ///             _ => Err(fmt::Error),
    ///         }
    ///     }
    /// }
    ///
    /// assert_eq!(cfmt::format!("{0:X?}, {0 :x}, {0 :X}", Hex(0xAB)), "Hex(AB), 0xab, 0xAB");
    /// ```
    ///
    /// The following statement panics at runtime since `"z"` is not a valid format specifier:
    ///
    /// ```rust,should_panic
    /// # use custom_format::runtime::{self as cfmt, CustomFormat};
    /// # use core::fmt;
    /// # struct Hex(u8);
    /// # impl CustomFormat for Hex {
    /// #     fn fmt(&self, f: &mut fmt::Formatter, spec: &str) -> fmt::Result {
    /// #         match spec {
    /// #             "x" => write!(f, "{:#02x}", self.0),
    /// #             "X" => write!(f, "{:#02X}", self.0),
    /// #             _ => Err(fmt::Error),
    /// #         }
    /// #     }
    /// # }
    /// cfmt::println!("{ :z}", Hex(0));
    /// ```
    ///
    fn fmt(&self, f: &mut fmt::Formatter, spec: &str) -> fmt::Result;
}

/// Wrapper for custom formatting via its [`Display`](core::fmt::Display) trait
#[derive(Debug, Clone)]
pub struct CustomFormatter<'a, T> {
    /// Format specifier
    spec: &'static str,
    /// Value to format
    value: &'a T,
}

impl<'a, T> CustomFormatter<'a, T> {
    /// Construct a new [`CustomFormatter`] value
    pub fn new(spec: &'static str, value: &'a T) -> Self {
        Self { spec, value }
    }
}

impl<T: CustomFormat> fmt::Display for CustomFormatter<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        CustomFormat::fmt(self.value, f, self.spec)
    }
}

pub use custom_format_macros::runtime_eprint as eprint;
pub use custom_format_macros::runtime_eprintln as eprintln;
pub use custom_format_macros::runtime_format as format;
pub use custom_format_macros::runtime_format_args as format_args;
pub use custom_format_macros::runtime_panic as panic;
pub use custom_format_macros::runtime_print as print;
pub use custom_format_macros::runtime_println as println;
pub use custom_format_macros::runtime_write as write;
pub use custom_format_macros::runtime_writeln as writeln;
