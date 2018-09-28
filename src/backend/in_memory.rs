use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use finchers;
use finchers::error::Error;
use finchers::input::Input;

use cookie::Cookie;
use futures::future;
use uuid::Uuid;

use super::{RawSession, SessionBackend};

#[derive(Debug, Default)]
struct Inner {
    storage: RwLock<HashMap<Uuid, String>>,
}

impl Inner {
    fn read_value(&self, input: &mut Input) -> Result<(Option<String>, Option<Uuid>), Error> {
        match input.cookies()?.get("session-id") {
            Some(cookie) => {
                let session_id: Uuid = cookie
                    .value()
                    .parse()
                    .map_err(finchers::error::bad_request)?;

                let inner = self.storage.read().map_err(|e| format_err!("{}", e))?;
                let value = inner.get(&session_id).cloned();

                Ok((value, Some(session_id)))
            }
            None => Ok((None, None)),
        }
    }

    fn write_value(
        &self,
        input: &mut Input,
        session_id: Option<Uuid>,
        value: String,
    ) -> Result<(), Error> {
        let session_id = session_id.unwrap_or_else(Uuid::new_v4);

        let mut inner = self.storage.write().map_err(|e| format_err!("{}", e))?;
        inner.insert(session_id.clone(), value);

        input
            .cookies()?
            .add(Cookie::new("session-id", session_id.to_string()));

        Ok(())
    }

    fn remove_value(&self, input: &mut Input, session_id: Option<Uuid>) -> Result<(), Error> {
        if let Some(session_id) = session_id {
            let mut inner = self.storage.write().map_err(|e| format_err!("{}", e))?;
            inner.remove(&session_id);

            input.cookies()?.remove(Cookie::named("session-id"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemorySessionBackend {
    inner: Arc<Inner>,
}

impl SessionBackend for InMemorySessionBackend {
    type Session = InMemorySession;
    type ReadError = Error;
    type ReadFuture = future::FutureResult<Self::Session, Self::ReadError>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        future::result(
            self.inner
                .read_value(input)
                .map(|(value, session_id)| InMemorySession {
                    inner: self.inner.clone(),
                    value,
                    session_id,
                }),
        )
    }
}

#[derive(Debug)]
pub struct InMemorySession {
    inner: Arc<Inner>,
    session_id: Option<Uuid>,
    value: Option<String>,
}

impl InMemorySession {
    fn write_impl(self, input: &mut Input) -> Result<(), Error> {
        match self.value {
            Some(value) => self.inner.write_value(input, self.session_id, value),
            None => self.inner.remove_value(input, self.session_id),
        }
    }
}

impl RawSession for InMemorySession {
    type WriteError = Error;
    type WriteFuture = future::FutureResult<(), Self::WriteError>;

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
