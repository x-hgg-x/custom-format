//! Provides types associated to compile-time formatting.

use core::fmt;

/// Trait for custom formatting with compile-time format checking
pub trait CustomFormat<const SPEC: u128> {
    /// Formats the value using the given formatter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use custom_format as cfmt;
    /// use custom_format::compile_time::{spec, CustomFormat};
    ///
    /// use core::fmt;
    ///
    /// #[derive(Debug)]
    /// struct Hex(u8);
    ///
    /// impl CustomFormat<{ spec("x") }> for Hex {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(f, "{:#02x}", self.0)
    ///     }
    /// }
    ///
    /// impl CustomFormat<{ spec("X") }> for Hex {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(f, "{:#02X}", self.0)
    ///     }
    /// }
    ///
    /// assert_eq!(cfmt::format!("{0:X?}, {0 :x}, {0 :X}", Hex(0xAB)), "Hex(AB), 0xab, 0xAB");
    /// ```
    ///
    /// The following statement doesn't compile since `"z"` is not a valid format specifier:
    ///
    /// ```rust,compile_fail
    /// # use custom_format as cfmt;
    /// # use custom_format::compile_time::{spec, CustomFormat};
    /// # use core::fmt;
    /// # struct Hex(u8);
    /// # impl CustomFormat<{ cfmt::spec("x") }> for Hex {
    /// #     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    /// #         write!(f, "{:#02x}", self.0)
    /// #     }
    /// # }
    /// # impl CustomFormat<{ cfmt::spec("X") }> for Hex {
    /// #     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    /// #         write!(f, "{:#02X}", self.0)
    /// #     }
    /// # }
    /// cfmt::println!("{ :z}", Hex(0));
    /// ```
    ///
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

/// Wrapper for custom formatting via its [`Display`](core::fmt::Display) trait.
///
/// The format specifier is a const-generic parameter and is part of the type.
///
#[derive(Debug, Clone)]
pub struct CustomFormatter<'a, T, const SPEC: u128> {
    /// Value to format
    value: &'a T,
}

impl<'a, T, const SPEC: u128> CustomFormatter<'a, T, SPEC> {
    /// Construct a new [`CustomFormatter`] value
    pub fn new(value: &'a T) -> Self {
        Self { value }
    }
}

/// Helper macro for constructing a new [`compile_time::CustomFormatter`](CustomFormatter) value from a format specifier
#[macro_export]
macro_rules! custom_formatter {
    ($spec:literal, $value:expr) => {{
        $crate::compile_time::CustomFormatter::<_, { $crate::compile_time::spec($spec) }>::new($value)
    }};
}
pub use custom_formatter;

impl<T: CustomFormat<SPEC>, const SPEC: u128> fmt::Display for CustomFormatter<'_, T, SPEC> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        CustomFormat::fmt(self.value, f)
    }
}

/// Convert a format specifier to a [`u128`], used as a const-generic parameter
pub const fn spec(s: &str) -> u128 {
    let bytes = s.as_bytes();
    let len = s.len();

    if len > 16 {
        #[allow(unconditional_panic)]
        let _ = ["format specifier is limited to 16 bytes"][usize::MAX];
    }

    let mut result = [0u8; 16];

    let mut i = 0;
    while i < len {
        result[i] = bytes[i];
        i += 1;
    }

    u128::from_le_bytes(result)
}
