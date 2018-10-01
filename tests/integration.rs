extern crate finchers;
extern crate finchers_session;

use finchers::local;
use finchers::prelude::*;
use finchers_session::backend::cookie;

type Session = finchers_session::Session<cookie::CookieSession>;

#[test]
fn test_no_op() {
    let backend = cookie::plain();
    let session = finchers_session::session(backend);

    let endpoint = session.and_then(|session: Session| session.with(|_session| Ok("done")));

    let response = local::get("/")
        .header("host", "localhost:3000")
        .respond(&endpoint);

    assert!(!response.headers().contains_key("set-cookie"));
}
