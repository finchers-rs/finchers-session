//! The implementation of session backend used in-memory database.

extern crate cookie;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use finchers;
use finchers::error::Error;
use finchers::input::Input;

use self::cookie::Cookie;
use futures::future;
use uuid::Uuid;

use backend::{Backend, RawSession};

/// Create a session backend which uses in-memory database.
pub fn in_memory() -> InMemoryBackend {
    InMemoryBackend::default()
}

#[derive(Debug, Default)]
struct Storage {
    inner: RwLock<HashMap<Uuid, String>>,
}

impl Storage {
    fn get(&self, session_id: &Uuid) -> Result<Option<String>, Error> {
        let inner = self.inner.read().map_err(|e| format_err!("{}", e))?;
        Ok(inner.get(&session_id).cloned())
    }

    fn set(&self, session_id: Uuid, value: String) -> Result<(), Error> {
        let mut inner = self.inner.write().map_err(|e| format_err!("{}", e))?;
        inner.insert(session_id, value);
        Ok(())
    }

    fn remove(&self, session_id: &Uuid) -> Result<(), Error> {
        let mut inner = self.inner.write().map_err(|e| format_err!("{}", e))?;
        inner.remove(&session_id);
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryBackend {
    inner: Arc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    storage: Storage,
}

impl InMemoryBackend {
    fn read_value(&self, input: &mut Input) -> Result<(Option<String>, Option<Uuid>), Error> {
        match input.cookies()?.get("session-id") {
            Some(cookie) => {
                let session_id: Uuid = cookie
                    .value()
                    .parse()
                    .map_err(finchers::error::bad_request)?;
                let value = self.inner.storage.get(&session_id)?;
                Ok((value, Some(session_id)))
            }
            None => Ok((None, None)),
        }
    }

    fn write_value(&self, input: &mut Input, session_id: Uuid, value: String) -> Result<(), Error> {
        self.inner.storage.set(session_id.clone(), value)?;
        input
            .cookies()?
            .add(Cookie::new("session-id", session_id.to_string()));
        Ok(())
    }

    fn remove_value(&self, input: &mut Input, session_id: Uuid) -> Result<(), Error> {
        self.inner.storage.remove(&session_id)?;
        input.cookies()?.remove(Cookie::named("session-id"));
        Ok(())
    }
}

impl Backend for InMemoryBackend {
    type Session = InMemorySession;
    type ReadFuture = future::FutureResult<Self::Session, Error>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        future::result(
            self.read_value(input)
                .map(|(value, session_id)| InMemorySession {
                    backend: self.clone(),
                    value,
                    session_id,
                }),
        )
    }
}

#[derive(Debug)]
pub struct InMemorySession {
    backend: InMemoryBackend,
    session_id: Option<Uuid>,
    value: Option<String>,
}

impl InMemorySession {
    fn write_impl(self, input: &mut Input) -> Result<(), Error> {
        match self.value {
            Some(value) => {
                let session_id = self.session_id.unwrap_or_else(Uuid::new_v4);
                self.backend.write_value(input, session_id, value)
            }
            None => match self.session_id {
                Some(session_id) => self.backend.remove_value(input, session_id),
                None => Ok(()),
            },
        }
    }
}

impl RawSession for InMemorySession {
    type WriteFuture = future::FutureResult<(), Error>;

    fn get(&self) -> Option<&str> {
        self.value.as_ref().map(|s| s.as_ref())
    }

    fn set(&mut self, value: String) {
        self.value = Some(value);
    }

    fn remove(&mut self) {
        self.value = None;
    }

    fn write(self, input: &mut Input) -> Self::WriteFuture {
        future::result(self.write_impl(input))
    }
}
