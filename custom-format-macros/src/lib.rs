#[cfg(any(feature = "runtime", feature = "compile-time"))]
mod fmt;

#[cfg(any(feature = "runtime", feature = "compile-time"))]
use proc_macro::TokenStream;

/// Wrapper function for converting [`proc_macro::TokenStream`] to `proc_macro2::TokenStream` for tests
#[cfg(any(feature = "runtime", feature = "compile-time"))]
#[allow(clippy::useless_conversion)]
fn fmt(input: TokenStream, skip_first: bool, root_macro: &str, compile_time: bool) -> TokenStream {
    fmt::fmt(input.into(), skip_first, root_macro, compile_time).parse().unwrap()
}

//
// Macros for runtime format specifier checking
//

/// Constructs parameters for the other string-formatting macros
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_format_args(input: TokenStream) -> TokenStream {
    fmt(input, false, "::core::format_args!", false)
}

/// Creates a `String` using interpolation of runtime expressions
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_format(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::format!", false)
}

/// Prints to the standard output
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_print(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::print!", false)
}

/// Prints to the standard output, with a newline
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_println(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::println!", false)
}

/// Prints to the standard error
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_eprint(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::eprint!", false)
}

/// Prints to the standard error, with a newline
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_eprintln(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::eprintln!", false)
}

/// Writes formatted data into a buffer
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_write(input: TokenStream) -> TokenStream {
    fmt(input, true, "::core::write!", false)
}

/// Write formatted data into a buffer, with a newline appended
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_writeln(input: TokenStream) -> TokenStream {
    fmt(input, true, "::core::writeln!", false)
}

/// Panics the current thread
#[proc_macro]
#[cfg(feature = "runtime")]
pub fn runtime_panic(input: TokenStream) -> TokenStream {
    fmt(input, false, "::core::panic!", false)
}

//
// Macros for compile-time format specifier checking
//

/// Constructs parameters for the other string-formatting macros
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_format_args(input: TokenStream) -> TokenStream {
    fmt(input, false, "::core::format_args!", true)
}

/// Creates a `String` using interpolation of runtime expressions
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_format(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::format!", true)
}

/// Prints to the standard output
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_print(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::print!", true)
}

/// Prints to the standard output, with a newline
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_println(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::println!", true)
}

/// Prints to the standard error
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_eprint(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::eprint!", true)
}

/// Prints to the standard error, with a newline
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_eprintln(input: TokenStream) -> TokenStream {
    fmt(input, false, "::std::eprintln!", true)
}

/// Writes formatted data into a buffer
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_write(input: TokenStream) -> TokenStream {
    fmt(input, true, "::core::write!", true)
}

/// Write formatted data into a buffer, with a newline appended
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_writeln(input: TokenStream) -> TokenStream {
    fmt(input, true, "::core::writeln!", true)
}

/// Panics the current thread
#[proc_macro]
#[cfg(feature = "compile-time")]
pub fn compile_time_panic(input: TokenStream) -> TokenStream {
    fmt(input, false, "::core::panic!", true)
}
