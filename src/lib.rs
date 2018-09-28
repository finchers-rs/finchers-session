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

extern crate cookie;
#[macro_use]
extern crate failure;
extern crate finchers;
extern crate futures;
extern crate serde;
extern crate serde_json;
extern crate time;
extern crate uuid;

#[cfg(feature = "redis")]
extern crate redis;

pub mod backend;

// ====

use finchers::endpoint::{Context, Endpoint, EndpointResult};
use finchers::error::Error;

use futures::{Future, IntoFuture, Poll};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

use backend::{RawSession, SessionBackend};

/// Create an endpoint which extracts a session manager from the request.
pub fn session<T, S>(backend: S) -> SessionEndpoint<T, S>
where
    T: Serialize + DeserializeOwned,
    S: SessionBackend,
{
    SessionEndpoint {
        backend,
        _marker: PhantomData,
    }
}

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone)]
pub struct SessionEndpoint<T, S> {
    backend: S,
    _marker: PhantomData<fn() -> T>,
}

impl<'a, T, S> Endpoint<'a> for SessionEndpoint<T, S>
where
    T: Serialize + DeserializeOwned + 'a,
    S: SessionBackend + 'a,
{
    type Output = (Session<T, S::Session>,);
    type Future = ReadSessionFuture<T, S::ReadFuture>;

    fn apply(&'a self, cx: &mut Context<'_>) -> EndpointResult<Self::Future> {
        Ok(ReadSessionFuture {
            future: self.backend.read(cx.input()),
            _marker: PhantomData,
        })
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ReadSessionFuture<T, F> {
    future: F,
    _marker: PhantomData<fn() -> T>,
}

impl<T, F> Future for ReadSessionFuture<T, F>
where
    T: Serialize + DeserializeOwned,
    F: Future,
    F::Item: RawSession,
    F::Error: Into<Error>,
{
    type Item = (Session<T, F::Item>,);
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.future.poll().map_err(Into::into).map(|x| {
            x.map(|raw| {
                (Session {
                    raw,
                    _marker: PhantomData,
                },)
            })
        })
    }
}

/// A struct which manages the session value per request.
#[derive(Debug)]
pub struct Session<T, S> {
    raw: S,
    _marker: PhantomData<fn() -> T>,
}

impl<T, S> Session<T, S>
where
    T: Serialize + DeserializeOwned,
    S: RawSession,
{
    /// Get the session value if available.
    pub fn get(&self) -> Result<Option<T>, Error> {
        if let Some(value) = self.raw.get() {
            serde_json::from_str(&value)
                .map(Some)
                .map_err(finchers::error::bad_request)
        } else {
            Ok(None)
        }
    }

    /// Set the session value.
    pub fn set(&mut self, value: T) -> Result<(), Error> {
        let value = serde_json::to_string(&value).map_err(finchers::error::fail)?;
        self.raw.set(value);
        Ok(())
    }

    /// Annotates to remove session value to the backend.
    pub fn remove(&mut self) {
        self.raw.remove();
    }

    #[doc(hidden)]
    #[deprecated(note = "use `into_future()` instead.")]
    pub fn finish(self) -> impl Future<Item = (), Error = Error> {
        self.into_future()
    }
}

impl<T, S> IntoFuture for Session<T, S>
where
    T: Serialize + DeserializeOwned,
    S: RawSession,
{
    type Item = ();
    type Error = Error;
    type Future = WriteSessionFuture<S::WriteFuture>;

    fn into_future(self) -> Self::Future {
        WriteSessionFuture {
            future: finchers::input::with_get_cx(|input| self.raw.write(input)),
        }
    }
}

#[doc(hidden)]
#[derive(Debug)]
#[must_use = "futures do not anything unless polled."]
pub struct WriteSessionFuture<F> {
    future: F,
}

impl<F> Future for WriteSessionFuture<F>
where
    F: Future<Item = ()>,
    F::Error: Into<Error>,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.future.poll().map_err(Into::into)
    }
}
