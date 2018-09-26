pub mod cookie;
pub mod in_memory;
pub mod redis;

use finchers::error::Error;
use finchers::input::Input;
use futures::Future;

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
