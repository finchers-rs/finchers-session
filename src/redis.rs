extern crate cookie;
extern crate redis;
extern crate uuid;

use self::cookie::Cookie;
use self::redis::async::Connection;
use self::redis::Client;
use self::uuid::Uuid;

use finchers;
use finchers::error::Error;
use finchers::input::Input;

use futures::future;
use futures::Future;
use std::collections::BTreeMap;

use super::{RawSession, SessionBackend};

#[derive(Debug)]
pub struct RedisSessionBackend {
    client: Client,
}

impl RedisSessionBackend {
    pub fn new(client: Client) -> RedisSessionBackend {
        RedisSessionBackend { client }
    }

    pub fn get_session_id(&self, input: &mut Input) -> Result<Option<Uuid>, Error> {
        if let Some(cookie) = input.cookies()?.get("session-id") {
            let session_id: Uuid = cookie
                .value()
                .parse()
                .map_err(finchers::error::bad_request)?;
            return Ok(Some(session_id));
        }
        Ok(None)
    }
}

impl SessionBackend for RedisSessionBackend {
    type Session = RedisSession;
    type ReadError = Error;
    type ReadFuture =
        Box<dyn Future<Item = Self::Session, Error = Self::ReadError> + Send + 'static>;

    fn read(&self, input: &mut Input) -> Self::ReadFuture {
        let session_id = match self.get_session_id(input) {
            Ok(id) => id,
            Err(err) => return Box::new(future::err(err)),
        };

        let read_future = self
            .client
            .get_async_connection()
            .map_err(finchers::error::fail)
            .and_then(move |conn| {
                if let Some(id) = session_id {
                    let future = redis::cmd("GET")
                        .arg(id.to_string())
                        .query_async(conn)
                        .map_err(finchers::error::fail)
                        .map(move |(conn, values)| RedisSession {
                            conn,
                            session_id: Some(id),
                            values,
                            modified: false,
                        });
                    Box::new(future) as Box<dyn Future<Item = RedisSession, Error = Error> + Send>
                } else {
                    let future = future::ok(RedisSession {
                        conn,
                        session_id,
                        values: BTreeMap::new(),
                        modified: false,
                    });
                    Box::new(future) as Box<dyn Future<Item = RedisSession, Error = Error> + Send>
                }
            });

        Box::new(read_future)
    }
}

#[allow(missing_debug_implementations)]
pub struct RedisSession {
    conn: Connection,
    session_id: Option<Uuid>,
    values: BTreeMap<String, String>,
    modified: bool,
}

impl RawSession for RedisSession {
    type WriteError = Error;
    type WriteFuture = Box<dyn Future<Item = (), Error = Self::WriteError> + Send + 'static>;

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
        let session_id = self.session_id.unwrap_or_else(Uuid::new_v4);
        let write_future = redis::cmd("SET")
            .arg(session_id.to_string())
            .arg(self.values)
            .query_async(self.conn)
            .map_err(finchers::error::fail)
            .and_then(|(conn, ())| Ok(()));
        Box::new(write_future)
    }
}
