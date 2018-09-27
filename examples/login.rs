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

use finchers::input::query::Serde;
use finchers::prelude::*;
use finchers_session::backend::in_memory::InMemorySessionBackend;
use finchers_session::{session, Session};

use futures::prelude::*;
use http::{Response, StatusCode};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
struct Login {
    username: String,
}

fn main() {
    pretty_env_logger::init();

    // Uses in memory database backend:
    let backend = InMemorySessionBackend::default();

    // Uses redis backend:
    // let client = redis::Client::open("redis://127.0.0.1").unwrap();
    // let backend = finchers_session::backend::redis(client);

    let session = Arc::new(session(backend));

    let greet = path!(@get /)
        .and(session.clone())
        .and_then(|session: Session<_>| {
            let response = match session.get::<Login>() {
                Ok(Some(login)) => html(format!(
                    "Hello, {}! <br />\n\
                     <form method=\"post\" action=\"/logout\">\n\
                     <input type=\"submit\" value=\"Log out\" />\n\
                     </form>\
                     ",
                    login.username
                )),
                _ => Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header("content-type", "text/html; charset=utf-8")
                    .body("<a href=\"/login\">Log in</a>".into())
                    .unwrap(),
            };
            session.finish().map(|_| response)
        });

    let login = path!(@get /"login"/)
        .and(session.clone())
        .and_then(|session: Session<_>| {
            let response = match session.get::<Login>() {
                Ok(Some(_login)) => redirect_to("/").map(|_| ""),
                _ => html(
                    "login form\n\
                     <form method=\"post\">\n\
                     <input type=\"text\" name=\"username\">\n\
                     <input type=\"submit\">\n\
                     </form>",
                ),
            };
            session.finish().map(|_| response)
        });

    let login_post = {
        #[derive(Debug, Deserialize)]
        struct Form {
            username: String,
        }

        path!(@post /"login"/)
            .and(session.clone())
            .and(endpoints::body::urlencoded().map(Serde::into_inner))
            .and_then(|mut session: Session<_>, form: Form| {
                session
                    .set(Login {
                        username: form.username,
                    }).into_future()
                    .and_then(move |()| session.finish().map(|_| redirect_to("/")))
            })
    };

    let logout =
        path!(@post /"logout"/)
            .and(session.clone())
            .and_then(|mut session: Session<_>| {
                session.remove();
                session.finish().map(|_| redirect_to("/"))
            });

    let endpoint = endpoint::EndpointObj::new(routes![greet, login, login_post, logout,]);

    info!("Listening on http://127.0.0.1:4000");
    finchers::launch(endpoint).start("127.0.0.1:4000");
}

fn redirect_to(location: &str) -> Response<()> {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("location", location)
        .body(())
        .unwrap()
}

fn html<T>(body: T) -> Response<T> {
    Response::builder()
        .header("content-type", "text/html; charset=utf-8")
        .body(body)
        .unwrap()
}
