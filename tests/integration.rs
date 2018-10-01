extern crate finchers;
extern crate finchers_session;
extern crate futures;

use finchers::error::Error;
use finchers::input::Input;
use finchers::local;
use finchers::prelude::*;
use finchers_session::backend::{Backend, RawSession};
use finchers_session::Session;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
enum Op {
    Read,
    Get,
    Set(String),
    Remove,
    Write,
}

#[derive(Default)]
struct CallChain {
    chain: RefCell<Vec<Op>>,
}

impl CallChain {
    fn register(&self, op: Op) {
        self.chain.borrow_mut().push(op);
    }

    fn result(&self) -> Vec<Op> {
        self.chain.borrow().clone()
    }
}

#[derive(Default)]
struct MockBackend {
    call_chain: Rc<CallChain>,
}

impl Backend for MockBackend {
    type Session = MockSession;
    type ReadFuture = futures::future::FutureResult<Self::Session, Error>;

    fn read(&self, _: &mut Input) -> Self::ReadFuture {
        self.call_chain.register(Op::Read);
        futures::future::ok(MockSession {
            call_chain: self.call_chain.clone(),
        })
    }
}

struct MockSession {
    call_chain: Rc<CallChain>,
}

impl RawSession for MockSession {
    type WriteFuture = futures::future::FutureResult<(), Error>;

    fn get(&self) -> Option<&str> {
        self.call_chain.register(Op::Get);
        None
    }

    fn set(&mut self, value: String) {
        self.call_chain.register(Op::Set(value));
    }

    fn remove(&mut self) {
        self.call_chain.register(Op::Remove);
    }

    fn write(self, _: &mut Input) -> Self::WriteFuture {
        self.call_chain.register(Op::Write);
        futures::future::ok(())
    }
}

#[test]
fn test_session_with() {
    let backend = Rc::new(MockBackend::default());

    let session = finchers_session::session(backend.clone());
    let endpoint = session.and_then(|session: Session<MockSession>| {
        session.with(|session| {
            session.get();
            session.set("foo");
            session.remove();
            Ok("done")
        })
    });

    let response = local::get("/")
        .header("host", "localhost:3000")
        .respond(&endpoint);
    assert!(!response.headers().contains_key("set-cookie"));

    assert_eq!(
        backend.call_chain.result(),
        vec![
            Op::Read,
            Op::Get,
            Op::Set("foo".into()),
            Op::Remove,
            Op::Write,
        ]
    );
}
