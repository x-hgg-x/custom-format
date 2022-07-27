#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! This crate provides procedural macros used for the `custom-format` crate.

mod fmt;

use proc_macro::TokenStream;

/// Wrapper function for converting [`proc_macro::TokenStream`] to `proc_macro2::TokenStream` for tests
#[allow(clippy::useless_conversion)]
fn fmt(input: TokenStream, skip_first: bool, root_macro: &str) -> TokenStream {
    fmt::fmt(input.into(), skip_first, root_macro).parse().unwrap()
}

/// Constructs parameters for the other string-formatting macros
#[proc_macro]
pub fn format_args(input: TokenStream) -> TokenStream {
    fmt(input, false, "::core::format_args!")
}

/// Creates a `String` using interpolation of runtime expressions
#[proc_macro]
pub fn format(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::format!")
}

/// Prints to the standard output
#[proc_macro]
pub fn print(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::print!")
}

/// Prints to the standard output, with a newline
#[proc_macro]
pub fn println(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::println!")
}

/// Prints to the standard error
#[proc_macro]
pub fn eprint(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::eprint!")
}

/// Prints to the standard error, with a newline
#[proc_macro]
pub fn eprintln(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::eprintln!")
}

/// Writes formatted data into a buffer
#[proc_macro]
pub fn write(input: TokenStream) -> TokenStream {
    fmt(input, true, "::core::write!")
}

/// Write formatted data into a buffer, with a newline appended
#[proc_macro]
pub fn writeln(input: TokenStream) -> TokenStream {
    fmt(input, true, "::core::writeln!")
}

/// Panics the current thread
#[proc_macro]
pub fn panic(input: TokenStream) -> TokenStream {
    fmt(input, false, "::core::panic!")
}
