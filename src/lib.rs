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
//!
//!
//! ## Additional features
//!
//! By default, the `syn` crate is used for parsing arguments of the procedural macros.
//!
//! To improve compilation time, a fast parsing mode is available when disabling default features.
//! It doesn't parse the full expression of the arguments, but instead only separate the arguments by the `,` token when not in a delimited group.
//!
//! This works for 99% of cases but cannot parse correctly expressions containing commas inside turbofishs or closures without delimiters:
//!
//! ```rust
//! // Compilation error due to incorrect parsing
//! cfmt::println!("{ :?}", |a, b| a);
//! cfmt::println!("{:?}", HashMap::<u32, u32>::new());
//! ```
//!
//! The workaround is simply to add an additional delimited group, or to define a new variable:
//!
//! ```rust
//! // No compilation error
//!
//! cfmt::println!("{ :?}", (|a, b| a)); // if a custom specifier is defined
//! cfmt::println!("{:?}", (HashMap::<u32, u32>::new()));
//! cfmt::println!("{:?}", { HashMap::<u32, u32>::new() });
//!
//! let map = HashMap::<u32, u32>::new();
//! cfmt::println!("{map:?}");
//! cfmt::println!("{map:?}", map = map);
//! ```

#[cfg(feature = "compile-time")]
pub mod compile_time;

#[cfg(feature = "runtime")]
pub mod runtime;
