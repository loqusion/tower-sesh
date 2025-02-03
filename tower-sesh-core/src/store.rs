use std::{error::Error as StdError, fmt};

use async_trait::async_trait;
use time::OffsetDateTime;

use crate::SessionKey;

type Result<T, E = Error> = std::result::Result<T, E>;

// TODO: MUST mention that the data format used by a session store must be
// self-describing, i.e. it implements `Deserializer::deserialize_any`. (This
// is because `Value`'s `Deserialize::deserialize` delegates to
// `Deserializer::deserialize_any`.)
//
// TODO: `Record` should be removed because you can't construct a `Record` without
// transferring ownership or cloning.
//
// TODO: Method signatures need a rework.

/// Backing storage for session data.
///
/// # Implementing `SessionStore`
///
/// Implementing this trait is highly discouraged, as its API is unstable and
/// may break implementors (for instance, by [adding a trait item]). As such,
/// this trait is sealed with a public (but hidden from documentation) `Sealed`
/// trait defined in the `tower-sesh-core` crate.
///
/// [adding a trait item]: https://doc.rust-lang.org/cargo/reference/semver.html#trait-new-item-no-default
///
/// To implement `SessionStore` anyway, implement the hidden `Sealed` trait too:
///
/// ```
/// use async_trait::async_trait;
/// use tower_sesh_core::SessionStore;
/// # use tower_sesh_core::{store::{Error, Record}, SessionKey};
///
/// struct StoreImpl<T> {
///     /* ... */
/// # _marker: std::marker::PhantomData<fn() -> T>,
/// }
///
/// #[async_trait]
/// impl<T> SessionStore<T> for StoreImpl<T>
/// # where T: 'static,
/// {
///     /* ... */
/// # async fn create(&self, record: &Record<T>) -> Result<SessionKey, Error> { todo!() }
/// # async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>, Error> { todo!() }
/// # async fn update(&self, session_key: &SessionKey, record: &Record<T>) -> Result<(), Error> { todo!() }
/// # async fn delete(&self, session_key: &SessionKey) -> Result<(), Error> { todo!() }
/// }
/// impl<T> tower_sesh_core::__private::Sealed for StoreImpl<T> {} // Required!
/// ```
#[cfg_attr(not(docsrs), async_trait)]
pub trait SessionStore<T>: 'static + Send + Sync + crate::__private::Sealed {
    async fn create(&self, record: &Record<T>) -> Result<SessionKey>;

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>>;

    async fn update(&self, session_key: &SessionKey, record: &Record<T>) -> Result<()>;

    async fn delete(&self, session_key: &SessionKey) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct Record<T> {
    data: T,
    expiry: OffsetDateTime,
}

impl<T> Record<T> {
    pub fn unix_timestamp(&self) -> i64 {
        self.expiry.unix_timestamp()
    }
}

#[derive(Debug)]
pub enum Error {}

impl StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

#[cfg(all(not(docsrs), test))]
#[test]
fn dyn_compatible() {
    use std::sync::Arc;

    const _: fn() = || {
        let _dyn_store: Arc<dyn SessionStore<()>> = todo!();
    };
}
