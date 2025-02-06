use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use async_trait::async_trait;
use parking_lot::{Mutex, MutexGuard};
use tower_sesh_core::{store::Ttl, Record, SessionKey};

/// Extractor to read and mutate session data.
///
/// # Session Migration
///
/// TODO
pub struct Session<T>(Arc<Mutex<Inner<T>>>);

/// A RAII mutex guard holding a lock to a mutex contained in `Session<T>`. The
/// data `T` can be accessed through this guard via its [`Deref`] and
/// [`DerefMut`] implementations.
///
/// The lock is automatically released whenever the guard is dropped.
//
// # Invariants
//
// 1. When constructing `SessionGuard`, the `data` contained within
//    `SessionInner` must contain a `Some` variant. This invariant must be met
//    while the mutex lock is held.
// 2. After the previous invariant is met, and until the `SessionGuard` is
//    dropped, the lock must never be released and `data` must never be replaced
//    with `None`.
pub struct SessionGuard<'a, T>(MutexGuard<'a, Inner<T>>);

/// A RAII mutex guard holding a lock to a mutex contained in `Session<T>`. The
/// data `Option<T>` can be accessed through this guard via its [`Deref`] and
/// [`DerefMut`] implementations.
///
/// The lock is automatically released whenever the guard is dropped.
pub struct OptionSessionGuard<'a, T>(MutexGuard<'a, Inner<T>>);

struct Inner<T> {
    session_key: Option<SessionKey>,
    data: Option<T>,
    expires_at: Option<Ttl>,
    status: Status,
}

/// # State transitions
///
/// Unchanged -> Changed | Renewed | Purged
/// Renewed -> Changed | Purged
/// Changed -> Purged
/// Purged
enum Status {
    Unchanged,
    Renewed,
    Changed,
    Purged,
}
use Status::*;

impl<T> Inner<T> {
    fn changed(&mut self) {
        if !matches!(self.status, Purged) {
            self.status = Changed;
        }
    }
}

impl<T> Session<T> {
    fn new(session_key: SessionKey, record: Record<T>) -> Session<T> {
        let inner = Inner {
            session_key: Some(session_key),
            data: Some(record.data),
            expires_at: Some(record.ttl),
            status: Unchanged,
        };
        Session(Arc::new(Mutex::new(inner)))
    }

    fn empty() -> Session<T> {
        let inner = Inner {
            session_key: None,
            data: None,
            expires_at: None,
            status: Unchanged,
        };
        Session(Arc::new(Mutex::new(inner)))
    }

    fn ignored(session_key: SessionKey) -> Session<T> {
        let inner = Inner {
            session_key: Some(session_key),
            data: None,
            expires_at: None,
            status: Unchanged,
        };
        Session(Arc::new(Mutex::new(inner)))
    }

    #[must_use]
    pub fn get(&self) -> OptionSessionGuard<'_, T> {
        let lock = self.0.lock();

        OptionSessionGuard::new(lock)
    }

    pub fn insert(&self, value: T) -> SessionGuard<'_, T> {
        let mut lock = self.0.lock();

        lock.data = Some(value);
        lock.changed();

        // SAFETY: a `None` variant for `data` would have been replaced by a
        // `Some` variant in the code above.
        unsafe { SessionGuard::new(lock) }
    }

    pub fn get_or_insert(&self, value: T) -> SessionGuard<'_, T> {
        let mut lock = self.0.lock();

        if lock.data.is_none() {
            lock.data = Some(value);
            lock.changed();
        }

        // SAFETY: a `None` variant for `data` would have been replaced by a
        // `Some` variant in the code above.
        unsafe { SessionGuard::new(lock) }
    }

    pub fn get_or_insert_with<F>(&self, f: F) -> SessionGuard<'_, T>
    where
        F: FnOnce() -> T,
    {
        let mut lock = self.0.lock();

        if lock.data.is_none() {
            lock.data = Some(f());
            lock.changed();
        }

        // SAFETY: a `None` variant for `data` would have been replaced by a
        // `Some` variant in the code above.
        unsafe { SessionGuard::new(lock) }
    }

    #[inline]
    pub fn get_or_insert_default(&self) -> SessionGuard<'_, T>
    where
        T: Default,
    {
        self.get_or_insert_with(T::default)
    }
}

impl<T> Clone for Session<T> {
    fn clone(&self) -> Self {
        Session(Arc::clone(&self.0))
    }
}

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "Failed to load session"]
    /// Rejection for [`Session`] if an unrecoverable error occurred when
    /// loading the session.
    pub struct SessionRejection;
}

#[cfg(feature = "axum")]
#[async_trait]
impl<S, T> axum::extract::FromRequestParts<S> for Session<T>
where
    T: 'static + Send + Sync,
{
    type Rejection = SessionRejection;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        match lazy::get_or_init(&mut parts.extensions).await {
            Ok(Some(session)) => Ok(session),
            Ok(None) => Err(SessionRejection),
            // Panic because this indicates a bug in the program rather than an
            // expected failure.
            Err(_) => panic!(
                "Missing request extension. `SessionLayer` must be called \
                before the `Session` extractor is run. Also, check that the \
                generic type for `Session<T>` is correct."
            ),
        }
    }
}

impl<'a, T> SessionGuard<'a, T> {
    /// # Safety
    ///
    /// The caller of this method must ensure that `owned_guard.data` is a
    /// `Some` variant.
    unsafe fn new(guard: MutexGuard<'a, Inner<T>>) -> Self {
        debug_assert!(guard.data.is_some());
        SessionGuard(guard)
    }
}

impl<T> Deref for SessionGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: `SessionGuard` holds the lock, so `data` can never be set
        // to `None`.
        unsafe { self.0.data.as_ref().unwrap_unchecked() }
    }
}

impl<T> DerefMut for SessionGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.changed();

        // SAFETY: `SessionGuard` holds the lock, so `data` can never be set
        // to `None`.
        unsafe { self.0.data.as_mut().unwrap_unchecked() }
    }
}

impl<'a, T> OptionSessionGuard<'a, T> {
    fn new(owned_guard: MutexGuard<'a, Inner<T>>) -> Self {
        OptionSessionGuard(owned_guard)
    }
}

impl<T> Deref for OptionSessionGuard<'_, T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}

impl<T> DerefMut for OptionSessionGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.changed();

        &mut self.0.data
    }
}

pub(crate) mod lazy {
    use std::{error::Error as StdError, fmt, sync::Arc};

    use async_once_cell::OnceCell;
    use cookie::Cookie;
    use http::Extensions;
    use tower_sesh_core::{store::ErrorKind, SessionKey, SessionStore};

    use crate::middleware::SessionConfig;

    use super::Session;

    pub(crate) fn insert<T>(
        cookie: Option<Cookie<'static>>,
        store: &Arc<impl SessionStore<T>>,
        extensions: &mut Extensions,
        session_config: SessionConfig,
    ) where
        T: 'static + Send,
    {
        debug_assert!(
            extensions.get::<LazySession<T>>().is_none(),
            "`session::lazy::insert` was called more than once!"
        );

        let lazy_session = match cookie {
            Some(cookie) => LazySession::new(cookie, Arc::clone(store), session_config),
            None => LazySession::empty(),
        };
        extensions.insert::<LazySession<T>>(lazy_session);
    }

    pub(super) async fn get_or_init<T>(
        extensions: &mut Extensions,
    ) -> Result<Option<Session<T>>, Error>
    where
        T: 'static + Send,
    {
        match extensions.get::<LazySession<T>>() {
            Some(lazy_session) => Ok(lazy_session.get_or_init().await.cloned()),
            None => Err(Error),
        }
    }

    pub(crate) fn take<T>(extensions: &mut Extensions) -> Result<Option<Session<T>>, Error>
    where
        T: 'static + Send,
    {
        match extensions.remove::<LazySession<T>>() {
            Some(lazy_session) => Ok(lazy_session.get().cloned()),
            None => Err(Error),
        }
    }

    enum LazySession<T> {
        Empty(Arc<OnceCell<Session<T>>>),
        Init {
            cookie: Cookie<'static>,
            store: Arc<dyn SessionStore<T> + 'static>,
            session: Arc<OnceCell<Option<Session<T>>>>,
            config: SessionConfig,
        },
    }

    impl<T> Clone for LazySession<T> {
        fn clone(&self) -> Self {
            match self {
                LazySession::Empty(session) => LazySession::Empty(Arc::clone(session)),
                LazySession::Init {
                    cookie,
                    store,
                    session,
                    config,
                } => LazySession::Init {
                    cookie: cookie.clone(),
                    store: Arc::clone(store),
                    session: Arc::clone(session),
                    config: config.clone(),
                },
            }
        }
    }

    impl<T> LazySession<T>
    where
        T: 'static,
    {
        fn new(
            cookie: Cookie<'static>,
            store: Arc<impl SessionStore<T>>,
            config: SessionConfig,
        ) -> LazySession<T> {
            LazySession::Init {
                cookie,
                store,
                session: Arc::new(OnceCell::new()),
                config,
            }
        }

        fn empty() -> LazySession<T> {
            LazySession::Empty(Arc::new(OnceCell::new()))
        }

        async fn get_or_init(&self) -> Option<&Session<T>> {
            match self {
                LazySession::Empty(session) => {
                    Some(session.get_or_init(async { Session::empty() }).await)
                }
                LazySession::Init {
                    cookie,
                    store,
                    session,
                    config,
                } => session
                    .get_or_init(init_session(cookie, store.as_ref(), config))
                    .await
                    .as_ref(),
            }
        }

        fn get(&self) -> Option<&Session<T>> {
            match self {
                LazySession::Empty(session) => session.get(),
                LazySession::Init { session, .. } => session.get().and_then(Option::as_ref),
            }
        }
    }

    async fn init_session<T>(
        cookie: &Cookie<'static>,
        store: &dyn SessionStore<T>,
        config: &SessionConfig,
    ) -> Option<Session<T>>
    where
        T: 'static,
    {
        let session_key = match SessionKey::decode(cookie.value()) {
            Ok(session_key) => session_key,
            Err(_) => return Some(Session::empty()),
        };

        match store.load(&session_key).await {
            Ok(Some(record)) => Some(Session::new(session_key, record)),
            Ok(None) => Some(Session::empty()),
            Err(err) => {
                match err.kind() {
                    ErrorKind::Serde(_) if config.ignore_invalid_sessions => {
                        Some(Session::ignored(session_key))
                    }
                    _ => {
                        // TODO: Better error reporting
                        error!(%err);
                        None
                    }
                }
            }
        }
    }

    pub(crate) struct Error;

    impl StdError for Error {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            None
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("missing request extension")
        }
    }

    impl fmt::Debug for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Error({:?})", self.to_string())
        }
    }
}
