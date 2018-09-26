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
extern crate futures;
extern crate serde;
extern crate serde_json;
extern crate uuid;

use finchers::endpoint::{Context, Endpoint, EndpointResult};
use finchers::error::Error;
use finchers::input::Input;

use futures::{Future, Poll};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod cookie;
pub mod in_memory;
pub mod redis;

pub trait RawSession {
    type WriteError: Into<Error>;
    type WriteFuture: Future<Item = (), Error = Self::WriteError>;

    fn get(&self, key: &str) -> Option<&str>;
    fn set(&mut self, key: &str, value: String);
    fn remove(&mut self, key: &str);
    fn clear(&mut self);

    fn write(self, input: &mut Input) -> Self::WriteFuture;
}

pub trait SessionBackend {
    type Session: RawSession;
    type ReadError: Into<Error>;
    type ReadFuture: Future<Item = Self::Session, Error = Self::ReadError>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture;
}

// ====

pub fn session<S: SessionBackend>(backend: S) -> SessionEndpoint<S> {
    SessionEndpoint { backend }
}

#[derive(Debug, Copy, Clone)]
pub struct SessionEndpoint<S> {
    backend: S,
}

impl<'a, S> Endpoint<'a> for SessionEndpoint<S>
where
    S: SessionBackend + 'a,
{
    type Output = (Session<S::Session>,);
    type Future = SessionFuture<S::ReadFuture>;

    fn apply(&'a self, cx: &mut Context<'_>) -> EndpointResult<Self::Future> {
        Ok(SessionFuture {
            future: self.backend.read(cx.input()),
        })
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct SessionFuture<F> {
    future: F,
}

impl<F> Future for SessionFuture<F>
where
    F: Future,
    F::Item: RawSession,
    F::Error: Into<Error>,
{
    type Item = (Session<F::Item>,);
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.future
            .poll()
            .map_err(Into::into)
            .map(|x| x.map(|raw| (Session { raw },)))
    }
}

#[must_use]
#[derive(Debug)]
pub struct Session<S: RawSession> {
    raw: S,
}

impl<S: RawSession> Session<S> {
    pub fn get<T>(&self, key: &str) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned,
    {
        if let Some(value) = self.raw.get(key) {
            serde_json::from_str(&value)
                .map(Some)
                .map_err(finchers::error::bad_request)
        } else {
            Ok(None)
        }
    }

    pub fn set<T>(&mut self, key: &str, value: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let value = serde_json::to_string(&value).map_err(finchers::error::fail)?;
        self.raw.set(key, value);
        Ok(())
    }

    pub fn remove(&mut self, key: &str) {
        self.raw.remove(key);
    }

    pub fn clear(&mut self) {
        self.raw.clear();
    }

    pub fn finish(self) -> impl Future<Item = (), Error = Error> {
        finchers::input::with_get_cx(|input| self.raw.write(input)).map_err(Into::into)
    }
}
