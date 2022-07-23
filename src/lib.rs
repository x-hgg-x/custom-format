#![no_std]

#[cfg(feature = "compile-time")]
pub mod compile_time;

#[cfg(feature = "runtime")]
pub mod runtime;
