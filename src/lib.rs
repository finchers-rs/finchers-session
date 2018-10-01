//! Session support for Finchers

// master
#![doc(html_root_url = "https://finchers-rs.github.io/finchers-session")]
// released
//#![doc(html_root_url = "https://docs.rs/finchers-session/0.1.0")]
#![warn(
    missing_docs,
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    unused,
)]
//#![warn(rust_2018_compatibility)]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "strict", doc(test(attr(deny(warnings)))))]

#[macro_use]
extern crate failure;
extern crate finchers;
#[cfg_attr(feature = "redis", macro_use)]
extern crate futures;
extern crate time;
extern crate uuid;

mod session;
#[cfg(test)]
mod tests;
mod util;

pub mod cookie;
pub mod in_memory;
#[cfg(feature = "redis")]
pub mod redis;

pub use self::session::Session;
