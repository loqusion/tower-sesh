//! The `SessionStore` trait and related items.
//!
//! # Implementing `SessionStore`
//!
//! `SessionStore` is sealed with the `SessionStoreImpl` trait. To implement
//! `SessionStore`, implement `SessionStoreImpl` too:
//!
//! ```
//! use async_trait::async_trait;
//! use tower_sesh_core::{store::SessionStoreImpl, SessionStore};
//! # use tower_sesh_core::{store::{Error, Record}, SessionKey, Ttl};
//!
//! struct StoreImpl<T> {
//!     /* ... */
//! # _marker: std::marker::PhantomData<fn() -> T>,
//! }
//!
//! impl<T> SessionStore<T> for StoreImpl<T>
//! # where T: 'static,
//! {}
//!
//! #[async_trait]
//! impl<T> SessionStoreImpl<T> for StoreImpl<T>
//! # where T: 'static,
//! {
//!     /* ... */
//! # async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey, Error> { unimplemented!() }
//! # async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>, Error> { unimplemented!() }
//! # async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<(), Error> { unimplemented!() }
//! # async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<(), Error> { unimplemented!() }
//! # async fn delete(&self, session_key: &SessionKey) -> Result<(), Error> { unimplemented!() }
//! }
//! ```

use std::{error::Error as StdError, fmt};

use async_trait::async_trait;

use crate::{time::Ttl, SessionKey};

type Result<T, E = Error> = std::result::Result<T, E>;

// TODO: MUST mention that the data format used by a session store must be
// self-describing, i.e. it implements `Deserializer::deserialize_any`. (This
// is because `Value`'s `Deserialize::deserialize` delegates to
// `Deserializer::deserialize_any`.)

/// Backing storage for session data.
///
/// This trait is sealed and intended to be opaque. The details of this trait
/// are open to change across non-major version bumps; as such, depending on
/// them may cause breakage.
pub trait SessionStore<T>: 'static + Send + Sync + SessionStoreImpl<T> {}

/// The contents of this trait are meant to be kept private and __not__
/// part of `SessionStore`'s public API. The details will change over time.
#[doc(hidden)]
#[async_trait]
pub trait SessionStoreImpl<T>: 'static + Send + Sync {
    async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey>;

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>>;

    async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()>;

    async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()>;

    async fn delete(&self, session_key: &SessionKey) -> Result<()>;
}

/// A struct containing a session's data and expiration time.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Record<T> {
    pub data: T,
    pub ttl: Ttl,
}

impl<T> Record<T> {
    #[inline]
    pub fn new(data: T, ttl: Ttl) -> Record<T> {
        Record { data, ttl }
    }

    #[inline]
    pub fn unix_timestamp(&self) -> i64 {
        self.ttl.unix_timestamp()
    }
}

/// An error returned by [`SessionStore`] methods.
pub struct Error {
    kind: ErrorKind,
}

/// Represents all the ways a [`SessionStore`] method can fail.
#[non_exhaustive]
pub enum ErrorKind {
    /// Catchall error message
    Message(Box<str>),

    /// Error occurred while interacting with the underlying storage mechanism.
    Store(Box<dyn StdError + Send + Sync>),

    /// Error occurred while serializing/deserializing.
    Serde(Box<dyn StdError + Send + Sync>),
}

impl Error {
    #[inline]
    fn new(kind: ErrorKind) -> Error {
        Error { kind }
    }

    /// Creates a new error from an error emitted by the underlying storage
    /// mechanism.
    #[cold]
    #[must_use]
    pub fn store(err: impl Into<Box<dyn StdError + Send + Sync + 'static>>) -> Error {
        Error::new(ErrorKind::Store(err.into()))
    }

    /// Creates a new error from an error emitted when serializing/deserializing
    /// data.
    #[cold]
    #[must_use]
    pub fn serde(err: impl Into<Box<dyn StdError + Send + Sync + 'static>>) -> Error {
        Error::new(ErrorKind::Serde(err.into()))
    }

    /// Creates a new error from a string containing a custom error message.
    #[cold]
    #[must_use]
    pub fn message(msg: impl Into<Box<str>>) -> Error {
        Error::new(ErrorKind::Message(msg.into()))
    }

    /// Returns the corresponding `ErrorKind` for this error.
    #[inline]
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = f.debug_struct("store::Error");

        use ErrorKind::*;
        match self.kind() {
            Message(msg) => {
                builder.field("message", msg);
            }
            Store(err) => {
                builder.field("kind", &"Store");
                builder.field("source", err);
            }
            Serde(err) => {
                builder.field("kind", &"Serde");
                builder.field("source", err);
            }
        }

        builder.finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ErrorKind::*;
        match self.kind() {
            Message(msg) => f.write_str(msg),
            Store(_) => f.write_str("session store error"),
            Serde(_) => f.write_str("session serialization error"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use ErrorKind::*;
        match self.kind() {
            Message(_) => None,
            Store(err) => Some(err.as_ref()),
            Serde(err) => Some(err.as_ref()),
        }
    }
}

#[cfg(test)]
mod test {
    use std::iter;

    use serde::Deserialize;

    use super::*;

    trait ErrorExt {
        fn display_chain(&self) -> DisplayChain<'_>;
    }

    impl<E> ErrorExt for E
    where
        E: StdError + 'static,
    {
        fn display_chain(&self) -> DisplayChain<'_> {
            DisplayChain { inner: self }
        }
    }

    struct DisplayChain<'a> {
        inner: &'a (dyn StdError + 'static),
    }

    impl fmt::Display for DisplayChain<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.inner)?;

            for error in iter::successors(Some(self.inner), |err| (*err).source()).skip(1) {
                write!(f, ": {}", error)?;
            }

            Ok(())
        }
    }

    #[test]
    fn test_store_dyn_compatible() {
        use std::sync::Arc;

        const _: fn() = || {
            let _dyn_store: Arc<dyn SessionStore<()>> = todo!();
        };
    }

    #[test]
    fn test_error_constraints() {
        fn require_traits<T: Send + Sync + 'static>() {}

        require_traits::<Error>();
    }

    fn error_store() -> Error {
        let err = "Reconnecting failed: Connection refused (os error 111)";
        Error::store(err)
    }

    fn error_serde() -> Error {
        #[derive(Debug, Deserialize)]
        struct Data {
            #[allow(dead_code)]
            hello: String,
        }

        let err = serde_json::from_str::<Data>(r#"{"hello": "world}"#).unwrap_err();
        Error::serde(err)
    }

    fn error_msg() -> Error {
        Error::message("max iterations reached when handling session key collisions")
    }

    #[test]
    fn test_error_display() {
        insta::assert_snapshot!(error_store(), @"session store error");
        insta::assert_snapshot!(
            error_store().display_chain(),
            @"session store error: Reconnecting failed: Connection refused (os error 111)"
        );
        insta::assert_snapshot!(error_serde(), @"session serialization error");
        insta::assert_snapshot!(
            error_serde().display_chain(),
            @"session serialization error: EOF while parsing a string at line 1 column 17"
        );
        insta::assert_snapshot!(
            error_msg(),
            @"max iterations reached when handling session key collisions"
        );
    }

    #[test]
    fn test_error_debug() {
        insta::assert_debug_snapshot!( error_store(), @r#"
        store::Error {
            kind: "Store",
            source: "Reconnecting failed: Connection refused (os error 111)",
        }
        "#
        );
        insta::assert_debug_snapshot!(error_serde(), @r#"
        store::Error {
            kind: "Serde",
            source: Error("EOF while parsing a string", line: 1, column: 17),
        }
        "#
        );
        insta::assert_debug_snapshot!(error_msg(), @r#"
        store::Error {
            message: "max iterations reached when handling session key collisions",
        }
        "#);
    }
}
