use finchers;
use finchers::error::Error;
use finchers::input::Input;

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use cookie::Cookie;
use futures::future;
use futures::{Future, Poll};
use redis;
use redis::async::Connection;
use redis::Client;
use serde_json;
use uuid::Uuid;

use super::{RawSession, SessionBackend};

#[derive(Debug)]
struct RedisSessionConfig {
    key_prefix: String,
    cookie_name: String,
    timeout: Option<Duration>,
}

impl RedisSessionConfig {
    fn key_name(&self, id: &Uuid) -> String {
        format!("{}:{}", self.key_prefix, id)
    }

    fn get_session_id(&self, input: &mut Input) -> Result<Option<Uuid>, Error> {
        if let Some(cookie) = input.cookies()?.get(&self.cookie_name) {
            let session_id: Uuid = cookie
                .value()
                .parse()
                .map_err(finchers::error::bad_request)?;
            return Ok(Some(session_id));
        }
        Ok(None)
    }
}

/// The instance of `SessionBackend` which uses Redis.
#[derive(Debug, Clone)]
pub struct RedisSessionBackend {
    client: Client,
    config: Arc<RedisSessionConfig>,
}

impl RedisSessionBackend {
    /// Create a new `RedisSessionBackend` from the specified Redis client.
    pub fn new(client: Client) -> RedisSessionBackend {
        RedisSessionBackend {
            client,
            config: Arc::new(RedisSessionConfig {
                key_prefix: "finchers-esssion".into(),
                cookie_name: "finchers-session-id".into(),
                timeout: None,
            }),
        }
    }

    fn config_mut(&mut self) -> &mut RedisSessionConfig {
        Arc::get_mut(&mut self.config).expect("The instance has already shared.")
    }

    /// Set the prefix string used in the key name when stores the session value
    /// to Redis.
    ///
    /// The default value is "finchers-session"
    pub fn key_prefix(mut self, prefix: impl AsRef<str>) -> RedisSessionBackend {
        self.config_mut().key_prefix = prefix.as_ref().into();
        self
    }

    /// Set the name of Cookie entry which stores the session id.
    ///
    /// The default value is "finchers-session-id"
    pub fn cookie_name(mut self, name: impl AsRef<str>) -> RedisSessionBackend {
        self.config_mut().cookie_name = name.as_ref().into();
        self
    }

    /// Set the timeout of session value.
    pub fn timeout(mut self, timeout: Duration) -> RedisSessionBackend {
        self.config_mut().timeout = Some(timeout);
        self
    }
}

impl SessionBackend for RedisSessionBackend {
    type Session = RedisSession;
    type ReadError = Error;
    type ReadFuture =
        Box<dyn Future<Item = Self::Session, Error = Self::ReadError> + Send + 'static>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        let session_id = match self.config.get_session_id(input) {
            Ok(id) => id,
            Err(err) => return Box::new(future::err(err)),
        };

        let config = self.config.clone();
        let read_future = self
            .client
            .get_async_connection()
            .map_err(finchers::error::fail)
            .and_then(move |conn| {
                if let Some(id) = session_id {
                    let future = redis::cmd("GET")
                        .arg(config.key_name(&id))
                        .query_async::<_, Option<String>>(conn)
                        .map_err(finchers::error::fail)
                        .and_then(move |(conn, value)| {
                            if let Some(value) = value {
                                let values =
                                    serde_json::from_str(&value).map_err(finchers::error::fail)?;
                                Ok(RedisSession {
                                    conn,
                                    session_id: Some(id),
                                    values,
                                    mode: WriteMode::Unmodified,
                                    config,
                                })
                            } else {
                                Ok(RedisSession {
                                    conn,
                                    session_id: None,
                                    values: BTreeMap::new(),
                                    mode: WriteMode::Unmodified,
                                    config,
                                })
                            }
                        });
                    Box::new(future) as Box<dyn Future<Item = RedisSession, Error = Error> + Send>
                } else {
                    let future = future::ok(RedisSession {
                        conn,
                        session_id: None,
                        values: BTreeMap::new(),
                        mode: WriteMode::Unmodified,
                        config,
                    });
                    Box::new(future) as Box<dyn Future<Item = RedisSession, Error = Error> + Send>
                }
            });

        Box::new(read_future)
    }
}

#[derive(Debug)]
enum WriteMode {
    Unmodified,
    Modified,
    Cleared,
}

#[allow(missing_docs)]
pub struct RedisSession {
    conn: Connection,
    config: Arc<RedisSessionConfig>,
    session_id: Option<Uuid>,
    values: BTreeMap<String, String>,
    mode: WriteMode,
}

impl fmt::Debug for RedisSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisSession")
            .field("config", &self.config)
            .field("session_id", &self.session_id)
            .field("values", &self.values)
            .field("mode", &self.mode)
            .finish()
    }
}

impl RawSession for RedisSession {
    type WriteError = Error;
    type WriteFuture = WriteFuture;

    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    fn set(&mut self, key: &str, value: String) {
        self.values.insert(key.to_owned(), value);
        self.mode = WriteMode::Modified;
    }

    fn remove(&mut self, key: &str) {
        self.values.remove(key);
        self.mode = WriteMode::Modified;
    }

    fn clear(&mut self) {
        self.values.clear();
        self.mode = WriteMode::Cleared;
    }

    fn write(self, input: &mut Input) -> Self::WriteFuture {
        let Self {
            conn,
            config,
            session_id,
            values,
            mode,
        } = self;

        let session_id = session_id.unwrap_or_else(Uuid::new_v4);

        match input.cookies() {
            Ok(jar) => {
                let cookie = Cookie::new(config.cookie_name.to_string(), session_id.to_string());
                jar.add(cookie)
            }
            Err(err) => return WriteFuture::failed(err),
        }

        let redis_key = config.key_name(&session_id);

        let value = match serde_json::to_string(&values) {
            Ok(value) => value,
            Err(err) => return WriteFuture::failed(finchers::error::fail(err)),
        };

        match mode {
            WriteMode::Modified => {
                if let Some(timeout) = config.timeout {
                    WriteFuture::cmd(
                        conn,
                        redis::cmd("SETEX")
                            .arg(redis_key)
                            .arg(timeout.as_secs())
                            .arg(value),
                    )
                } else {
                    WriteFuture::cmd(conn, redis::cmd("SET").arg(redis_key).arg(value))
                }
            }
            WriteMode::Cleared => WriteFuture::cmd(conn, redis::cmd("DEL").arg(redis_key)),
            _ => WriteFuture::no_op(),
        }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct WriteFuture {
    state: WriteFutureState,
}

enum WriteFutureState {
    Noop,
    Failed(Option<Error>),
    Cmd(redis::RedisFuture<(Connection, ())>),
}

impl WriteFuture {
    fn no_op() -> WriteFuture {
        WriteFuture {
            state: WriteFutureState::Noop,
        }
    }

    fn failed(err: Error) -> WriteFuture {
        WriteFuture {
            state: WriteFutureState::Failed(Some(err)),
        }
    }

    fn cmd(conn: Connection, cmd: &redis::Cmd) -> WriteFuture {
        WriteFuture {
            state: WriteFutureState::Cmd(cmd.query_async::<_, ()>(conn)),
        }
    }
}

impl Future for WriteFuture {
    type Item = ();
    type Error = Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use self::WriteFutureState::*;
        match self.state {
            Noop => Ok(().into()),
            Failed(ref mut err) => Err(err.take().expect("The future has already polled.")),
            Cmd(ref mut cmd) => cmd
                .poll()
                .map(|x| x.map(|(_conn, ())| ()))
                .map_err(finchers::error::fail),
        }
    }
}
