#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

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
//!   The set of possible custom format specifiers is defined at compilation, so invalid specifiers can be checked at compile-time.
//!   This allows the library to have the same performance as when using the standard library formatting traits.
//!   See the [`compile_time::CustomFormat`] trait.
//!
//! - `runtime` (*enabled by default*)
//!
//!   The formatting method dynamically checks the format specifier at runtime for each invocation.
//!   This is a slower version, but has a lower MSRV for greater compatibility.
//!   See the [`runtime::CustomFormat`] trait.

#[cfg(feature = "compile-time")]
#[cfg_attr(docsrs, doc(cfg(feature = "compile-time")))]
pub mod compile_time;

#[cfg(feature = "runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "runtime")))]
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
        $crate::custom_format_macros::fmt!($crate, [$($macro)*], [$($first_arg)?], [$($result),*])
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! fmt_inner {
    ([$($macro:tt)*], [$($first_arg:expr)?], ) => {{
        compile_error!("requires at least a format string argument")
    }};
    ([$($macro:tt)*], [$($first_arg:expr)?], $fmt:literal) => {{
        $crate::custom_format_macros::fmt!($crate, [$($macro)*], [$($first_arg)?], [$fmt])
    }};
    ([$($macro:tt)*], [$($first_arg:expr)?], $fmt:literal, $($arg:tt)*) => {{
        $crate::parse_args!([$($macro)*], [$($first_arg)?], [$fmt], $($arg)*,)
    }};
}

/// Constructs parameters for the other string-formatting macros.
///
/// ## Important note
///
/// The other macros in this crate use an inner `match` to avoid reevaluating the input arguments several times.
///
/// For example, the following `println!` call:
///
/// ```rust
/// use custom_format as cfmt;
/// use core::fmt;
///
/// #[derive(Debug)]
/// struct Hex(u8);
///
/// impl cfmt::runtime::CustomFormat for Hex {
///     fn fmt(&self, f: &mut fmt::Formatter, _: &str) -> fmt::Result {
///         write!(f, "{:#02x}", self.0)
///     }
/// }
///
/// fn call() -> Hex {
///     Hex(42)
/// }
///
/// cfmt::println!("{0:?}, {res :<x>}", res = call());
/// ```
///
/// is expanded to:
///
/// ```rust
/// # use custom_format as cfmt;
/// # use core::fmt;
/// # #[derive(Debug)]
/// # struct Hex(u8);
/// # impl cfmt::runtime::CustomFormat for Hex {
/// #     fn fmt(&self, f: &mut fmt::Formatter, _: &str) -> fmt::Result {
/// #         write!(f, "{:#02x}", self.0)
/// #     }
/// # }
/// # fn call() -> Hex { Hex(42) }
/// match (&(call())) {
///     (arg0) => ::std::println!("{0:?}, {1}", arg0, cfmt::runtime::CustomFormatter::new("x", arg0)),
/// }
/// ```
///
/// This method doesn't work with the `format_args!` macro, since it returns a value of type [`core::fmt::Arguments`]
/// which borrows the temporary values of the `match`. Since these temporary values are dropped before returning,
/// the return value cannot be used at all if the format string contains format specifiers.
///
/// For this reason, the `format_args!` macro is expanded in another way. The following call:
///
/// ```rust
/// # use custom_format as cfmt;
/// # use core::fmt;
/// # #[derive(Debug)]
/// # struct Hex(u8);
/// # impl cfmt::runtime::CustomFormat for Hex {
/// #     fn fmt(&self, f: &mut fmt::Formatter, _: &str) -> fmt::Result {
/// #         write!(f, "{:#02x}", self.0)
/// #     }
/// # }
/// # fn call() -> Hex { Hex(42) }
/// println!("{}", cfmt::format_args!("{0:?}, {res :<x>}", res = call()));
/// ```
///
/// must be expanded to:
///
/// ```rust
/// # use custom_format as cfmt;
/// # use core::fmt;
/// # #[derive(Debug)]
/// # struct Hex(u8);
/// # impl cfmt::runtime::CustomFormat for Hex {
/// #     fn fmt(&self, f: &mut fmt::Formatter, _: &str) -> fmt::Result {
/// #         write!(f, "{:#02x}", self.0)
/// #     }
/// # }
/// # fn call() -> Hex { Hex(42) }
/// println!("{}", ::core::format_args!("{0:?}, {1}", &(call()), cfmt::runtime::CustomFormatter::new("x", &(call()))));
/// ```
///
/// which reevaluates the input arguments if they are used several times in the format string.
///
/// To avoid unnecessary reevaluations, we can store the expression result in a variable beforehand:
///
/// ```rust
/// # use custom_format as cfmt;
/// # use core::fmt;
/// # #[derive(Debug)]
/// # struct Hex(u8);
/// # impl cfmt::runtime::CustomFormat for Hex {
/// #     fn fmt(&self, f: &mut fmt::Formatter, _: &str) -> fmt::Result {
/// #         write!(f, "{:#02x}", self.0)
/// #     }
/// # }
/// # fn call() -> Hex { Hex(42) }
/// let res = call();
/// println!("{}", cfmt::format_args!("{res:?}, {res :<x>}"));
/// ```
///
/// is expanded to:
///
/// ```rust
/// # use custom_format as cfmt;
/// # use core::fmt;
/// # #[derive(Debug)]
/// # struct Hex(u8);
/// # impl cfmt::runtime::CustomFormat for Hex {
/// #     fn fmt(&self, f: &mut fmt::Formatter, _: &str) -> fmt::Result {
/// #         write!(f, "{:#02x}", self.0)
/// #     }
/// # }
/// # fn call() -> Hex { Hex(42) }
/// # let res = call();
/// println!("{}", ::core::format_args!("{0:?}, {1}", &res, cfmt::runtime::CustomFormatter::new("x", &res)))
/// ```
#[macro_export]
macro_rules! format_args {
    ($($arg:tt)*) => {{
        $crate::fmt_inner!([::core::format_args!], [], $($arg)*)
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
