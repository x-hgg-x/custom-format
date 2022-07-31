#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! This crate extends the standard formatting syntax with custom format specifiers, by providing custom formatting macros.
//!
//! It uses ` :` (a space and a colon) as a separator before the format specifier, which is not a syntax currently accepted and allows supporting standard specifiers in addition to custom specifiers.
//! It also supports [format args capture](https://blog.rust-lang.org/2022/01/13/Rust-1.58.0.html#captured-identifiers-in-format-strings)
//! even on older versions of Rust, since it manually adds the named parameter if missing.
//!
//! This library comes in two flavors, corresponding to the following features:
//!
//! - `compile-time` (*enabled by default*)
//!
//!     The set of possible custom format specifiers is defined at compilation, so invalid specifiers can be checked at compile-time.
//!     This allows the library to have the same performance as when using the standard library formatting traits.
//!     See the [`compile_time::CustomFormat`](crate::compile_time::CustomFormat) trait.
//!
//! - `runtime` (*enabled by default*)
//!
//!     The formatting method dynamically checks the format specifier at runtime for each invocation.
//!     This is a slower version, but has a lower MSRV for greater compatibility.
//!     See the [`runtime::CustomFormat`](crate::runtime::CustomFormat) trait.

#[cfg(feature = "compile-time")]
pub mod compile_time;

#[cfg(feature = "runtime")]
pub mod runtime;

#[doc(hidden)]
pub use custom_format_macros;

#[doc(hidden)]
#[macro_export]
macro_rules! parse_args {
    ([$($macro:tt)*], [$($first_arg:expr)?], [$($result:expr),*], $id:ident = $expr:expr, $($arg:tt)*) => {{
        $crate::parse_args!([$($macro)*], [$($first_arg)?], [$($result,)* ($id) = $expr], $($arg)*)
    }};
    ([$($macro:tt)*], [$($first_arg:expr)?], [$($result:expr),*], $expr:expr, $($arg:tt)*) => {{
        $crate::parse_args!([$($macro)*], [$($first_arg)?], [$($result,)* $expr], $($arg)*)
    }};
    ([$($macro:tt)*], [$($first_arg:expr)?], [$($result:expr),*], $(,)?) => {{
        $crate::custom_format_macros::fmt!([$($macro)*], [$($first_arg)?], [$($result),*])
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! fmt_inner {
    ([$($macro:tt)*], [$($first_arg:expr)?], ) => {{
        compile_error!("requires at least a format string argument")
    }};
    ([$($macro:tt)*], [$($first_arg:expr)?], $fmt:literal) => {{
        $crate::custom_format_macros::fmt!([$($macro)*], [$($first_arg)?], [$fmt])
    }};
    ([$($macro:tt)*], [$($first_arg:expr)?], $fmt:literal, $($arg:tt)*) => {{
        $crate::parse_args!([$($macro)*], [$($first_arg)?], [$fmt], $($arg)*,)
    }};
}

/// Creates a `String` using interpolation of runtime expressions
#[macro_export]
macro_rules! format {
    ($($arg:tt)*) => {{
        $crate::fmt_inner!([::std::format!], [], $($arg)*)
    }};
}

/// Prints to the standard output
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::fmt_inner!([::std::print!], [], $($arg)*)
    }};
}

/// Prints to the standard output, with a newline
#[macro_export]
macro_rules! println {
    () => {{
        ::std::println!()
    }};
    ($($arg:tt)*) => {{
        $crate::fmt_inner!([::std::println!], [], $($arg)*)
    }};
}

/// Prints to the standard error
#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => {{
        $crate::fmt_inner!([::std::eprint!], [], $($arg)*)
    }};
}

/// Prints to the standard error, with a newline
#[macro_export]
macro_rules! eprintln {
    () => {{
        ::std::eprintln!()
    }};
    ($($arg:tt)*) => {{
        $crate::fmt_inner!([::std::eprintln!], [], $($arg)*)
    }};
}

/// Writes formatted data into a buffer
#[macro_export]
macro_rules! write {
    ($dst:expr, $($arg:tt)*) => {{
        $crate::fmt_inner!([::core::write!], [$dst], $($arg)*)
    }};
}

/// Write formatted data into a buffer, with a newline appended
#[macro_export]
macro_rules! writeln {
    ($dst:expr) => {{
        ::core::writeln!($dst)
    }};
    ($dst:expr, $($arg:tt)*) => {{
        $crate::fmt_inner!([::core::writeln!], [$dst], $($arg)*)
    }};
}

/// Panics the current thread
#[macro_export]
macro_rules! panic {
    () => {{
        ::core::panic!()
    }};
    ($($arg:tt)*) => {{
        $crate::fmt_inner!([::core::panic!], [], $($arg)*)
    }};
}
