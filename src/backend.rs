//! The definition of backends.

use finchers::error::Error;
use finchers::input::Input;
use futures::Future;

use std::rc::Rc;
use std::sync::Arc;

// not a public API.
#[doc(hidden)]
pub trait Backend {
    type Session: RawSession;
    type ReadFuture: Future<Item = Self::Session, Error = Error>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture;
}

// not a public API.
#[doc(hidden)]
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
