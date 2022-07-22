#![no_std]

use core::fmt;

/// Trait for custom formatting
pub trait CustomFormat {
    fn fmt(&self, f: &mut fmt::Formatter, spec: &str) -> fmt::Result;
}

/// Wrapper for custom formatting via its [`Display`](core::fmt::Display) trait
#[derive(Debug, Clone)]
pub struct CustomFormatter<'a, T> {
    /// Value to format
    value: &'a T,
    /// Format specifier
    spec: &'static str,
}

impl<'a, T> CustomFormatter<'a, T> {
    /// Construct a new [`CustomFormatter`] value
    pub fn new(value: &'a T, spec: &'static str) -> Self {
        Self { value, spec }
    }
}

impl<T: CustomFormat> fmt::Display for CustomFormatter<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        CustomFormat::fmt(self.value, f, self.spec)
    }
}

pub use custom_format_macros::*;
