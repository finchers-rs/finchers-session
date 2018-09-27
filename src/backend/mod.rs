#![allow(missing_docs)]

//! The definition of backends.

pub mod cookie;
pub mod in_memory;
#[cfg(feature = "redis")]
pub mod redis;

use finchers::error::Error;
use finchers::input::Input;
use futures::Future;

use std::rc::Rc;
use std::sync::Arc;

#[allow(missing_docs)]
pub trait RawSession {
    type WriteError: Into<Error>;
    type WriteFuture: Future<Item = (), Error = Self::WriteError>;

    fn get(&self) -> Option<&str>;
    fn set(&mut self, value: String);
    fn remove(&mut self);
    fn write(self, input: &mut Input) -> Self::WriteFuture;
}

#[allow(missing_docs)]
pub trait SessionBackend {
    type Session: RawSession;
    type ReadError: Into<Error>;
    type ReadFuture: Future<Item = Self::Session, Error = Self::ReadError>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture;
}

impl<T: SessionBackend> SessionBackend for Box<T> {
    type Session = T::Session;
    type ReadError = T::ReadError;
    type ReadFuture = T::ReadFuture;

    #[inline]
    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        (**self).read(input)
    }
}

impl<T: SessionBackend> SessionBackend for Rc<T> {
    type Session = T::Session;
    type ReadError = T::ReadError;
    type ReadFuture = T::ReadFuture;

    #[inline]
    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        (**self).read(input)
    }
}

impl<T: SessionBackend> SessionBackend for Arc<T> {
    type Session = T::Session;
    type ReadError = T::ReadError;
    type ReadFuture = T::ReadFuture;

    #[inline]
    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        (**self).read(input)
    }
}

/// Create a session backend which uses the specified Redis client.
#[cfg(feature = "redis")]
pub fn redis(client: ::redis::Client) -> self::redis::RedisSessionBackend {
    self::redis::RedisSessionBackend::new(client)
}
