#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! This crate provides procedural macros used for the `custom-format` crate.

mod fmt;

use proc_macro::TokenStream;

/// Parse custom format specifiers in format string and write output tokens.
///
/// This is an internal unstable macro and should not be used directly.
#[proc_macro]
#[allow(clippy::useless_conversion)]
pub fn fmt(input: TokenStream) -> TokenStream {
    fmt::fmt(input.into()).into()
}
