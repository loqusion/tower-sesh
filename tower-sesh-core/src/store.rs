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
//! # use tower_sesh_core::{store::{Record, Result}, SessionKey, Ttl};
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
//! # async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey> { unimplemented!() }
//! # async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> { unimplemented!() }
//! # async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()> { unimplemented!() }
//! # async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()> { unimplemented!() }
//! # async fn delete(&self, session_key: &SessionKey) -> Result<()> { unimplemented!() }
//! }
//! ```

use std::{error::Error as StdError, fmt};

use async_trait::async_trait;

use crate::{time::Ttl, SessionKey};

/// A specialized `Result` type for store operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

// TODO: MUST mention that the data format used by a session store must be
// self-describing, i.e. it implements `Deserializer::deserialize_any`. (This
// is because `Value`'s `Deserialize::deserialize` delegates to
// `Deserializer::deserialize_any`.)

/// Storage mechanism used to store, retrieve, and mutate session data.
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
    /// Creates a session, returning the session key that may be used to
    /// subsequently retrieve the session.
    ///
    /// Implementors should randomly generate a session key and perform
    /// collision resolution (even though collisions are statistically
    /// improbable).
    async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey>;

    /// Returns a record containing the data and expiry corresponding to the
    /// session identified by the provided session key.
    ///
    /// If there is no session identified by the given session key, `Ok(None)`
    /// is returned.
    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>>;

    /// Updates the session identified by the provided session key.
    ///
    /// If no session identified by the session key exists, or if it has
    /// expired, it should be created.
    async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()>;

    /// Updates the expiry of the session identified by the provided session
    /// key.
    ///
    /// If no session identified by the session key exists, or if it has
    /// expired, this should be a no-op with an `Ok` result.
    async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()>;

    /// Deletes the session identified by the provided session key.
    ///
    /// If no session identified by the session key exists, this should be a
    /// no-op with an `Ok` result.
    async fn delete(&self, session_key: &SessionKey) -> Result<()>;
}

/// A trait allowing a session store to override its source of randomness, for
/// use in testing.
///
/// Every [`SessionStore`] implementation requires a source of randomness to
/// generate session keys, but it should also use the passed RNG for _any_
/// randomness it requires.
///
/// To meet [`SessionStore`]'s `Send` and `Sync` requirements while allowing
/// interior mutability, an implementor can use a locking data structure such
/// as [`std::sync::Mutex`] (or [`parking_lot::Mutex`]). However, this results
/// in performance degradation, so use conditional compilation to ensure it is
/// not used outside of testing.
///
/// [`parking_lot::Mutex`]: https://docs.rs/parking_lot/latest/parking_lot/type.Mutex.html
///
/// # Example
///
/// ```rust
/// use std::sync::Mutex;
/// use rand::{rngs::ThreadRng, CryptoRng, Rng, SeedableRng};
/// use rand_chacha::ChaCha20Rng;
/// use tower_sesh_core::{store::SessionStoreRng};
///
/// pub struct Store {
///     #[cfg(feature = "test-util")]
/// #   _unused: (),
///     rng: Option<Box<Mutex<dyn CryptoRng + Send + 'static>>>,
///     // ...
/// }
/// #
/// # impl Store {
/// #   fn new() -> Self {
/// #       Self {
/// #           #[cfg(feature = "test-util")]
/// #           _unused: (),
/// #           rng: None
/// #       }
/// #   }
/// # }
///
/// #[cfg(feature = "test-util")]
/// # {}
/// impl<Rng> SessionStoreRng<Rng> for Store
/// where
///     Rng: CryptoRng + Send + 'static,
/// {
///     fn rng(&mut self, rng: Rng) {
///         self.rng = Some(Box::new(Mutex::new(rng)));
///     }
/// }
///
/// impl Store {
///     #[cfg(feature = "test-util")]
///     # fn _unused() { unimplemented!() }
///     fn random<U>(&self) -> U
///     where
///         rand::distr::StandardUniform: rand::distr::Distribution<U>,
///     {
///         // Slower, for testing only
///         if let Some(rng) = &self.rng {
///             rng.lock().unwrap().random()
///         } else {
///             ThreadRng::default().random()
///         }
///     }
/// # }
///
/// # struct Hidden;
/// # impl Hidden {
///     #[cfg(not(feature = "test-util"))]
///     # fn _unused() { unimplemented!() }
///     fn random<U>(&self) -> U
///     where
///         rand::distr::StandardUniform: rand::distr::Distribution<U>,
///     {
///         // Faster (no branching or locking)
///         ThreadRng::default().random()
///     }
/// }
///
/// let mut store = Store::new();
/// store.rng(ChaCha20Rng::seed_from_u64(9));
/// ```
pub trait SessionStoreRng<Rng: rand::CryptoRng + Send + 'static> {
    /// Overrides the PRNG used by the session store to randomly generate
    /// session keys.
    ///
    /// This is only suitable for tests, as it results in performance
    /// degradation.
    fn rng(&mut self, rng: Rng);
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
}

/// An error returned by [`SessionStore`] methods.
pub struct Error {
    kind: ErrorKind,
}

/// Represents all the ways a [`SessionStore`] method can fail.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Catchall error message.
    Message(Box<str>),

    /// Error occurred from the underlying storage mechanism.
    Store(Box<dyn StdError + Send + Sync>),

    /// Error occurred from serializing/deserializing.
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

    /// Error returned when session key collision resolution reaches max
    /// iterations.
    #[cold]
    pub fn max_iterations_reached() -> Error {
        Error::message("max iterations reached when handling session key collisions")
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
    use serde::Deserialize;

    use crate::util::Report;

    use super::*;

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
    #[cfg_attr(miri, ignore = "incompatible with miri")]
    fn test_error_display() {
        insta::assert_snapshot!(error_store(), @"session store error");
        insta::assert_snapshot!(
            Report::new(error_store()),
            @"session store error: Reconnecting failed: Connection refused (os error 111)"
        );
        insta::assert_snapshot!(error_serde(), @"session serialization error");
        insta::assert_snapshot!(
            Report::new(error_serde()),
            @"session serialization error: EOF while parsing a string at line 1 column 17"
        );
        insta::assert_snapshot!(
            error_msg(),
            @"max iterations reached when handling session key collisions"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "incompatible with miri")]
    fn test_error_debug() {
        insta::assert_debug_snapshot!(error_store(), @r#"
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
