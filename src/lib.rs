//! Session support for Finchers.
//!
//! Supported backends:
//!
//! * Cookie
//! * In-memory database
//! * Redis (requires the feature flag `feature = "redis"`)
//!
//! # Feature Flags
//!
//! * `redis` - enable Redis backend (default: off)
//! * `secure` - enable signing and encryption support for Cookie values
//!              (default: on. it adds the crate `ring` to dependencies).

#![doc(html_root_url = "https://finchers-rs.github.io/docs/finchers-session/v0.1.0")]
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

pub use self::session::{RawSession, Session};
