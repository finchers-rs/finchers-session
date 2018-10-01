#[macro_use]
extern crate finchers;
extern crate finchers_session;
extern crate http;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate serde;
extern crate serde_json;

use finchers::prelude::*;

use http::Response;

type Session = finchers_session::Session<finchers_session::backend::in_memory::InMemorySession>;

#[derive(Debug, Deserialize, Serialize, Default)]
struct SessionValue {
    text: String,
}

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
    let session = finchers_session::session(backend);

    let endpoint = path!(@get /).and(session).and_then(|session: Session| {
        session.with(|session| {
            // Retrieve the value of session.
            //
            // Note that the session value are stored as a UTF-8 string,
            // which means that the user it is necessary for the user to
            // deserialize/serialize the session data.
            let mut session_value: SessionValue = {
                let s = session.get().unwrap_or(r#"{ "text": "" }"#);
                serde_json::from_str(s).map_err(|err| {
                    finchers::error::bad_request(format!(
                        "failed to parse session value (input = {:?}): {}",
                        s, err
                    ))
                })?
            };

            let response = Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .body(format!("{:?}", session_value))
                .expect("should be a valid response");

            session_value.text += "a";

            // Stores session data to the store.
            let s = serde_json::to_string(&session_value).map_err(finchers::error::fail)?;
            session.set(s);

            Ok(response)
        })
    });

    info!("Listening on http://127.0.0.1:4000");
    finchers::launch(endpoint).start("127.0.0.1:4000");
}
