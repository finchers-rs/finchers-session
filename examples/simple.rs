#[macro_use]
extern crate finchers;
extern crate finchers_session;
extern crate futures;
extern crate http;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate serde;

use finchers::error::Error;
use finchers::prelude::*;
use finchers_session::{session, Session};

use futures::prelude::*;
use http::Response;

#[derive(Debug, Deserialize, Serialize, Default)]
struct SessionValue(String);

fn main() {
    pretty_env_logger::init();

    // Uses in memory database backend:
    let backend = finchers_session::backend::in_memory();

    // Uses cookie backend:
    // let master_key = "this-is-a-very-very-secret-master-key";
    // let backend = finchers_session::backend::cookie::signed(master_key);

    // Uses redis backend:
    // let client = redis::Client::open("redis://127.0.0.1").unwrap();
    // let backend = finchers_session::backend::redis(client);

    // Create an endpoint which extracts a session manager from request.
    let session = session::<SessionValue, _>(backend);

    let endpoint = path!(@get /)
        .and(session)
        .and_then(
            |mut session: Session<SessionValue, _>| -> Result<_, Error> {
                let mut session_value = session.get()?.unwrap_or_default();
                let response = Response::builder()
                    .header("content-type", "text/html; charset=utf-8")
                    .body(format!(
                        "Reload this page to add an 'a': {}\n\n\
                         Clear cookies to reset.",
                        session_value.0
                    )).expect("should be a valid response");

                session_value.0 += "a";
                session.set(session_value)?;

                Ok((response, session))
            },
        ).and_then(|(response, session): (_, Session<_, _>)| {
            session.into_future().map(|_| response)
        });

    info!("Listening on http://127.0.0.1:4000");
    finchers::launch(endpoint).start("127.0.0.1:4000");
}
