use finchers::error::Error;
use finchers::input::Input;

use cookie::{Cookie, CookieJar, Key};
use futures::future;
use std::fmt;
use std::sync::Arc;

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
    fn read_value(&self, input: &mut Input) -> Result<Option<String>, Error> {
        let cookies = input.cookies()?;
        let cookie_opt = match self.security {
            Security::Signed => cookies.signed(&self.key).get(&self.name),
            Security::Private => cookies.private(&self.key).get(&self.name),
        };
        match cookie_opt {
            Some(cookie) => Ok(Some(cookie.value().to_string())),
            None => Ok(None),
        }
    }

    fn write_value(&self, value: String, jar: &mut CookieJar) {
        let cookie = Cookie::new(self.name.clone(), value);
        match self.security {
            Security::Signed => jar.signed(&self.key).add(cookie),
            Security::Private => jar.private(&self.key).add(cookie),
        }
    }

    fn remove_value(&self, jar: &mut CookieJar) {
        let cookie = Cookie::named(self.name.clone());
        match self.security {
            Security::Signed => jar.signed(&self.key).remove(cookie),
            Security::Private => jar.private(&self.key).remove(cookie),
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
        future::result(self.inner.read_value(input).map(|value| CookieSession {
            inner: self.inner.clone(),
            value,
        }))
    }
}

#[derive(Debug)]
pub struct CookieSession {
    inner: Arc<Inner>,
    value: Option<String>,
}

impl CookieSession {
    fn write_impl(self, input: &mut Input) -> Result<(), Error> {
        let jar = input.cookies()?;
        if let Some(value) = self.value {
            self.inner.write_value(value, jar);
        } else {
            self.inner.remove_value(jar);
        }
        Ok(())
    }
}

impl RawSession for CookieSession {
    type WriteError = Error;
    type WriteFuture = future::FutureResult<(), Self::WriteError>;

    fn get(&self) -> Option<&str> {
        self.value.as_ref().map(|s| s.as_str())
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
