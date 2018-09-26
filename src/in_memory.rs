extern crate cookie;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use finchers;
use finchers::error::Error;
use finchers::input::Input;

use self::cookie::Cookie;
use futures::future;
use uuid::Uuid;

use super::{RawSession, SessionBackend};

#[derive(Debug, Default)]
struct Inner {
    storage: RwLock<HashMap<Uuid, HashMap<String, String>>>,
}

impl Inner {
    fn read_values(
        &self,
        input: &mut Input,
    ) -> Result<(HashMap<String, String>, Option<Uuid>), Error> {
        if let Some(cookie) = input.cookies()?.get("session-id") {
            let session_id: Uuid = cookie
                .value()
                .parse()
                .map_err(finchers::error::bad_request)?;

            let inner = self.storage.read().map_err(|e| format_err!("{}", e))?;
            let values = inner.get(&session_id).cloned().unwrap_or_default();

            return Ok((values, Some(session_id)));
        }
        Ok((HashMap::new(), None))
    }

    fn write_values(
        &self,
        input: &mut Input,
        values: HashMap<String, String>,
        session_id: Option<Uuid>,
    ) -> Result<(), Error> {
        let session_id = session_id.unwrap_or_else(Uuid::new_v4);

        let mut inner = self.storage.write().map_err(|e| format_err!("{}", e))?;
        inner.insert(session_id.clone(), values);

        input
            .cookies()?
            .add(Cookie::new("session-id", session_id.to_string()));

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
                .read_values(input)
                .map(|(values, session_id)| InMemorySession {
                    inner: self.inner.clone(),
                    values,
                    session_id,
                }),
        )
    }
}

#[derive(Debug)]
pub struct InMemorySession {
    inner: Arc<Inner>,
    values: HashMap<String, String>,
    session_id: Option<Uuid>,
}

impl RawSession for InMemorySession {
    type WriteError = Error;
    type WriteFuture = future::FutureResult<(), Self::WriteError>;

    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    fn set(&mut self, key: &str, value: String) {
        self.values.insert(key.to_owned(), value);
    }

    fn remove(&mut self, key: &str) {
        self.values.remove(key);
    }

    fn clear(&mut self) {
        self.values.clear();
    }

    fn write(self, input: &mut Input) -> Self::WriteFuture {
        future::result(self.inner.write_values(input, self.values, self.session_id))
    }
}
