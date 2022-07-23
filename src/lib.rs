#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! This crate extends the standard formatting syntax with custom format specifiers, by providing custom formatting macros.
//!
//! It uses ` :` (a space and a colon) as a separator before the format specifier, which is not a syntax currently accepted and allows supporting standard specifiers in addition to custom specifiers.
//!
//! This library comes in two flavors, corresponding to the following features:
//!
//! - `compile-time`
//!
//!     The set of possible custom format specifiers is defined at compilation, so invalid specifiers can be checked at compile-time.
//!     This allows the library to have the same performance as when using the standard library formatting traits.
//!     See the [`compile_time::CustomFormat`](crate::compile_time::CustomFormat) trait.
//!
//! - `runtime`
//!
//!     The formatting method dynamically checks the format specifier at runtime for each invocation.
//!     This is a slower version, but has a lower MSRV for greater compatibility.
//!     See the [`runtime::CustomFormat`](crate::runtime::CustomFormat) trait.

#[cfg(feature = "compile-time")]
pub mod compile_time;

#[cfg(feature = "runtime")]
pub mod runtime;
