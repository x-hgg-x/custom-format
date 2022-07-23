use core::fmt;

/// Trait for custom formatting
pub trait CustomFormat<const SPEC: u128> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

/// Wrapper for custom formatting via its [`Display`](core::fmt::Display) trait
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

/// Helper macro for constructing a new [`CustomFormatter`] value from a format specifier
#[macro_export]
macro_rules! custom_formatter {
    ($spec:literal, $value:expr) => {{
        $crate::compile_time::CustomFormatter::<_, { $crate::compile_time::spec($spec) }>::new($value)
    }};
}

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

pub use custom_format_macros::compile_time_eprint as eprint;
pub use custom_format_macros::compile_time_eprintln as eprintln;
pub use custom_format_macros::compile_time_format as format;
pub use custom_format_macros::compile_time_format_args as format_args;
pub use custom_format_macros::compile_time_panic as panic;
pub use custom_format_macros::compile_time_print as print;
pub use custom_format_macros::compile_time_println as println;
pub use custom_format_macros::compile_time_write as write;
pub use custom_format_macros::compile_time_writeln as writeln;
