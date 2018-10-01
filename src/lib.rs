//! Session support for Finchers

// master
#![doc(html_root_url = "https://finchers-rs.github.io/finchers-session")]
// released
//#![doc(html_root_url = "https://docs.rs/finchers-session/0.1.0")]
#![warn(
    missing_docs,
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    unused,
)]
//#![warn(rust_2018_compatibility)]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "strict", doc(test(attr(deny(warnings)))))]

extern crate cookie;
#[macro_use]
extern crate failure;
extern crate finchers;
#[cfg_attr(feature = "redis", macro_use)]
extern crate futures;
extern crate time;
extern crate uuid;

#[cfg(feature = "redis")]
extern crate redis;

pub mod backend;

pub use self::imp::{session, Session, SessionEndpoint};

mod imp {
    use finchers::endpoint;
    use finchers::endpoint::{ApplyContext, ApplyResult, Endpoint};
    use finchers::error::Error;

    use futures::{Future, IntoFuture, Poll};

    use backend::{Backend, RawSession};

    /// Create an endpoint which extracts a session manager from the request.
    pub fn session<B>(backend: B) -> SessionEndpoint<B>
    where
        B: Backend,
    {
        SessionEndpoint { backend }
    }

    #[allow(missing_docs)]
    #[derive(Debug, Copy, Clone)]
    pub struct SessionEndpoint<B: Backend> {
        backend: B,
    }

    impl<'a, B> Endpoint<'a> for SessionEndpoint<B>
    where
        B: Backend + 'a,
    {
        type Output = (Session<B::Session>,);
        type Future = ReadSessionFuture<B::ReadFuture>;

        fn apply(&'a self, cx: &mut ApplyContext<'_>) -> ApplyResult<Self::Future> {
            Ok(ReadSessionFuture {
                future: self.backend.read(cx.input()),
            })
        }
    }

    #[derive(Debug)]
    pub struct ReadSessionFuture<F> {
        future: F,
    }

    impl<F> Future for ReadSessionFuture<F>
    where
        F: Future,
        F::Item: RawSession,
        F::Error: Into<Error>,
    {
        type Item = (Session<F::Item>,);
        type Error = Error;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            self.future
                .poll()
                .map_err(Into::into)
                .map(|x| x.map(|raw| (Session { raw },)))
        }
    }

    /// A struct which manages the session value per request.
    #[derive(Debug)]
    #[must_use = "The value must be convert into a Future to finish the session handling."]
    pub struct Session<S: RawSession> {
        raw: S,
    }

    impl<S> Session<S>
    where
        S: RawSession,
    {
        /// Get the session value if available.
        pub fn get(&self) -> Option<&str> {
            self.raw.get()
        }

        /// Set the session value.
        pub fn set(&mut self, value: impl Into<String>) {
            self.raw.set(value.into());
        }

        /// Annotates to remove session value to the backend.
        pub fn remove(&mut self) {
            self.raw.remove();
        }

        #[allow(missing_docs)]
        pub fn with<R>(
            mut self,
            f: impl FnOnce(&mut Self) -> R,
        ) -> impl Future<Item = R::Item, Error = Error>
        where
            R: IntoFuture<Error = Error>,
        {
            f(&mut self)
                .into_future()
                .and_then(move |item| self.into_future().map(move |()| item))
        }
    }

    impl<S> IntoFuture for Session<S>
    where
        S: RawSession,
    {
        type Item = ();
        type Error = Error;
        type Future = WriteSessionFuture<S::WriteFuture>;

        fn into_future(self) -> Self::Future {
            WriteSessionFuture {
                future: endpoint::with_get_cx(|input| self.raw.write(input)),
            }
        }
    }

    #[derive(Debug)]
    #[must_use = "futures do not anything unless polled."]
    pub struct WriteSessionFuture<F> {
        future: F,
    }

    impl<F> Future for WriteSessionFuture<F>
    where
        F: Future<Item = ()>,
        F::Error: Into<Error>,
    {
        type Item = ();
        type Error = Error;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            self.future.poll().map_err(Into::into)
        }
    }
}
