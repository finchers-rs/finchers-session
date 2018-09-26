extern crate cookie as cookie;

use std::collections::HashMap;
use std::sync::Arc;

use finchers::error::Error;
use finchers::input::Input;

use self::cookie::{Cookie, CookieJar, Key};
use futures::future;
use serde_json;
use std::fmt;

use super::{RawSession, SessionBackend};

#[derive(Debug)]
enum Security {
    Signed,
    Private,
}

struct Inner {
    key: Key,
    security: Security,
    name: String,
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inner")
            .field("security", &self.security)
            .field("name", &self.name)
            .finish()
    }
}

impl Inner {
    fn read_values(&self, input: &mut Input) -> Result<HashMap<String, String>, Error> {
        let cookies = input.cookies()?;
        let cookie_opt = match self.security {
            Security::Signed => cookies.signed(&self.key).get(&self.name),
            Security::Private => cookies.private(&self.key).get(&self.name),
        };
        match cookie_opt {
            Some(cookie) => {
                let values =
                    serde_json::from_str(cookie.value()).map_err(::finchers::error::bad_request)?;
                Ok(values)
            }
            None => Ok(HashMap::new()),
        }
    }

    fn write_value(&self, value: String, jar: &mut CookieJar) {
        let cookie = Cookie::new(self.name.clone(), value);
        match self.security {
            Security::Signed => jar.signed(&self.key).add(cookie),
            Security::Private => jar.private(&self.key).add(cookie),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CookieSessionBackend {
    inner: Arc<Inner>,
}

impl CookieSessionBackend {
    fn new(key: Key, security: Security) -> CookieSessionBackend {
        CookieSessionBackend {
            inner: Arc::new(Inner {
                key,
                security,
                name: "finchers-session".into(),
            }),
        }
    }

    pub fn signed(key: impl AsRef<[u8]>) -> CookieSessionBackend {
        CookieSessionBackend::new(Key::from_master(key.as_ref()), Security::Signed)
    }

    pub fn private(key: impl AsRef<[u8]>) -> CookieSessionBackend {
        CookieSessionBackend::new(Key::from_master(key.as_ref()), Security::Private)
    }
}

impl SessionBackend for CookieSessionBackend {
    type Session = CookieSession;
    type ReadError = Error;
    type ReadFuture = future::FutureResult<Self::Session, Self::ReadError>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        future::result(self.inner.read_values(input).map(|values| CookieSession {
            inner: self.inner.clone(),
            values,
            modified: false,
        }))
    }
}

#[derive(Debug)]
pub struct CookieSession {
    inner: Arc<Inner>,
    values: HashMap<String, String>,
    modified: bool,
}

impl CookieSession {
    fn write_impl(self, input: &mut Input) -> Result<(), Error> {
        if self.modified {
            let value = serde_json::to_string(&self.values).map_err(::finchers::error::fail)?;
            let jar = input.cookies()?;
            self.inner.write_value(value, jar);
        }
        Ok(())
    }
}

impl RawSession for CookieSession {
    type WriteError = Error;
    type WriteFuture = future::FutureResult<(), Self::WriteError>;

    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    fn set(&mut self, key: &str, value: String) {
        self.values.insert(key.to_owned(), value);
        self.modified = true;
    }

    fn remove(&mut self, key: &str) {
        self.values.remove(key);
        self.modified = true;
    }

    fn clear(&mut self) {
        self.values.clear();
        self.modified = true;
    }

    fn write(self, input: &mut Input) -> Self::WriteFuture {
        future::result(self.write_impl(input))
    }
}
