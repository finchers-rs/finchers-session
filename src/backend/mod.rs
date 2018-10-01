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
pub trait Backend {
    type Session: RawSession;
    type ReadFuture: Future<Item = Self::Session, Error = Error>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture;
}

#[allow(missing_docs)]
pub trait RawSession {
    type WriteFuture: Future<Item = (), Error = Error>;

    fn get(&self) -> Option<&str>;
    fn set(&mut self, value: String);
    fn remove(&mut self);
    fn write(self, input: &mut Input) -> Self::WriteFuture;
}

impl<T: Backend> Backend for Box<T> {
    type Session = T::Session;
    type ReadFuture = T::ReadFuture;

    #[inline]
    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        (**self).read(input)
    }
}

impl<T: Backend> Backend for Rc<T> {
    type Session = T::Session;
    type ReadFuture = T::ReadFuture;

    #[inline]
    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        (**self).read(input)
    }
}

impl<T: Backend> Backend for Arc<T> {
    type Session = T::Session;
    type ReadFuture = T::ReadFuture;

    #[inline]
    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        (**self).read(input)
    }
}

/// Create a session backend which uses in-memory database.
pub fn in_memory() -> self::in_memory::InMemoryBackend {
    self::in_memory::InMemoryBackend::default()
}

/// Create a session backend which uses the specified Redis client.
#[cfg(feature = "redis")]
pub fn redis(client: ::redis::Client) -> self::redis::RedisBackend {
    self::redis::RedisBackend::new(client)
}
