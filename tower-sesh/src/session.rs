use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use parking_lot::{Mutex, MutexGuard};
use tower_sesh_core::{time::now, Record, SessionKey, SessionStore, Ttl};

/// Extractor to read and mutate session data.
///
/// # Session migration
///
/// TODO
///
/// # Logging rejections
///
/// To see the logs, enable the `tracing` feature for `tower-sesh` (enabled by
/// default) and the `tower_sesh::rejection=trace` tracing target, for example
/// with `RUST_LOG=info,tower_sesh::rejection=trace cargo run`.
pub struct Session<T> {
    inner: Arc<Mutex<Inner<T>>>,
}

pub(crate) struct Inner<T> {
    session_key: Option<SessionKey>,
    data: Option<T>,
    expires_at: Option<Ttl>,
    status: Status,
}

/// The status of a session.
///
/// Valid state transitions are as follows:
///
/// `Unchanged` -> `Renewed` | `Changed` | `Purged` | `Taken`
/// `Renewed` -> `Changed` | `Purged` | `Taken`
/// `Changed` -> `Purged` | `Taken`
/// `Purged` -> `Taken`
///
/// State transitions should be performed with the methods [`Inner::renewed`],
/// [`Inner::changed`], [`Inner::purged`], and [`Inner::take`] instead of
/// directly assigning to `status`.
///
/// `Taken` means the session `Inner` fields have been `mem::replace`d; using
/// any of the fields after a session is `Taken` is a bug.
#[derive(Clone, Copy, Debug)]
enum Status {
    /// `Session` is unchanged, so no sync action is required.
    Unchanged,

    /// `Session` expiry should be renewed.
    Renewed,

    /// `Session` data and expiry should be synced.
    Changed,

    /// `Session` should be removed from the session store.
    Purged,

    /// `Session` was taken, so any further use is invalid.
    Taken,
}
use Status::*;

/// Which action was performed by `Session::sync`.
pub(crate) enum SyncAction {
    /// The session was created, updated, or renewed with the session key.
    Set(SessionKey),

    /// The session was removed.
    Remove,

    /// The session was unmodified. No action was performed.
    None,
}

impl<T> Session<T> {
    /// # Examples
    ///
    /// ```
    /// use axum::{extract::Path, response::IntoResponse};
    /// use tower_sesh::Session;
    /// # use std::time::Duration;
    /// # use axum::{body::Body, routing::get, Router};
    /// # use http::{header, Request, StatusCode};
    /// # use tower::ServiceExt;
    /// # use tower_sesh::{store::MemoryStore, SessionLayer};
    /// # use tower_sesh_core::{store::SessionStoreImpl, SessionKey, Ttl};
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct SessionData {
    /// #     user_id: u64,
    /// # }
    /// #
    /// async fn sensitive(
    ///     Path(user_id): Path<u64>,
    ///     session: Session<SessionData>,
    /// ) -> impl IntoResponse {
    ///     if session
    ///         .get()
    ///         .as_ref()
    ///         .is_some_and(|s| s.user_id == user_id)
    ///     {
    ///         // authorized
    ///         # StatusCode::OK
    ///     } else {
    ///         // unauthorized
    ///         # StatusCode::FORBIDDEN
    ///     }
    /// }
    /// #
    /// # tokio_test::block_on(async {
    /// #
    /// # let session_key = SessionKey::try_from(1)?;
    /// # let store = MemoryStore::<SessionData>::new();
    /// # store.update(
    /// #     &session_key,
    /// #     &SessionData { user_id: 1234 },
    /// #     Ttl::now_local().unwrap() + Duration::from_secs(10 * 60),
    /// # ).await?;
    /// # let session_layer = SessionLayer::plain(store.into())
    /// #     .cookie_name("id");
    /// # let app = Router::new()
    /// #     .route("/user/{user_id}/sensitive", get(sensitive))
    /// #     .layer(session_layer);
    /// #
    /// # let req = Request::builder()
    /// #     .uri("/user/1234/sensitive")
    /// #     .header(
    /// #         header::COOKIE,
    /// #         format!("id={}", session_key.encode()),
    /// #     )
    /// #     .body(Body::empty())?;
    /// # let res = app.clone().oneshot(req).await?;
    /// # assert!(res.status().is_success());
    /// #
    /// # let req = Request::builder()
    /// #     .uri("/user/1337/sensitive")
    /// #     .header(
    /// #         header::COOKIE,
    /// #         format!("id={}", session_key.encode()),
    /// #     )
    /// #     .body(Body::empty())?;
    /// # let res = app.clone().oneshot(req).await?;
    /// # assert!(!res.status().is_success());
    /// #
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// # }).unwrap();
    /// ```
    ///
    /// ```
    /// use axum::response::IntoResponse;
    /// use tower_sesh::Session;
    /// # use std::time::Duration;
    /// # use axum::{body::Body, routing::patch, Router};
    /// # use http::{header, Method, Request, StatusCode};
    /// # use tower::ServiceExt;
    /// # use tower_sesh::{store::MemoryStore, SessionLayer};
    /// # use tower_sesh_core::{store::SessionStoreImpl, SessionKey, Ttl};
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct SessionData {
    /// #     theme: Theme,
    /// # }
    /// #
    /// # #[derive(Clone, Copy, Debug)]
    /// # enum Theme {
    /// #     Light,
    /// #     Dark,
    /// # }
    /// #
    /// async fn change_theme(session: Session<SessionData>) -> impl IntoResponse {
    ///     if let Some(data) = session.get().as_mut() {
    ///         data.theme = Theme::Light;
    ///         # StatusCode::OK
    ///     }
    ///     # else {
    ///         # StatusCode::FORBIDDEN
    ///     # }
    /// }
    /// #
    /// # tokio_test::block_on(async {
    /// #
    /// # let session_key = SessionKey::try_from(1)?;
    /// # let store = MemoryStore::<SessionData>::new();
    /// # store.update(
    /// #     &session_key,
    /// #     &SessionData { theme: Theme::Dark },
    /// #     Ttl::now_local().unwrap() + Duration::from_secs(10 * 60),
    /// # ).await?;
    /// # let session_layer = SessionLayer::plain(store.into())
    /// #     .cookie_name("id");
    /// # let app = Router::new()
    /// #     .route("/user/set-theme", patch(change_theme))
    /// #     .layer(session_layer);
    /// #
    /// # let req = Request::builder()
    /// #     .uri("/user/set-theme")
    /// #     .method(Method::PATCH)
    /// #     .header(
    /// #         header::COOKIE,
    /// #         format!("id={}", session_key.encode()),
    /// #     )
    /// #     .body(Body::empty())?;
    /// # let res = app.clone().oneshot(req).await?;
    /// # assert!(res.status().is_success());
    /// #
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// # }).unwrap();
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self) -> OptionSessionGuard<'_, T> {
        let guard = self.lock();

        OptionSessionGuard::new(guard)
    }

    pub fn insert(&self, value: T) -> SessionGuard<'_, T> {
        let mut guard = self.lock();

        guard.data = Some(value);
        guard.changed();

        // SAFETY: a `None` variant for `data` would have been replaced by a
        // `Some` variant in the code above.
        unsafe { SessionGuard::new(guard) }
    }

    #[inline]
    pub fn get_or_insert(&self, value: T) -> SessionGuard<'_, T> {
        let mut guard = self.lock();

        if guard.data.is_none() {
            guard.data = Some(value);
            guard.changed();
        }

        // SAFETY: a `None` variant for `data` would have been replaced by a
        // `Some` variant in the code above.
        unsafe { SessionGuard::new(guard) }
    }

    #[inline]
    pub fn get_or_insert_with<F>(&self, f: F) -> SessionGuard<'_, T>
    where
        F: FnOnce() -> T,
    {
        let mut guard = self.lock();

        if guard.data.is_none() {
            guard.data = Some(f());
            guard.changed();
        }

        // SAFETY: a `None` variant for `data` would have been replaced by a
        // `Some` variant in the code above.
        unsafe { SessionGuard::new(guard) }
    }

    #[inline]
    pub fn get_or_insert_default(&self) -> SessionGuard<'_, T>
    where
        T: Default,
    {
        self.get_or_insert_with(T::default)
    }

    #[inline]
    pub fn renew(&self) {
        self.lock().renewed();
    }

    #[inline]
    pub fn purge(&self) {
        self.lock().purged();
    }

    #[inline]
    fn lock(&self) -> MutexGuard<'_, Inner<T>> {
        let guard = self.inner.lock();

        #[cfg(feature = "tracing")]
        if guard.is_taken() {
            error!("called `Session` method after it was synchronized to store");
        }

        guard
    }
}

impl<T> Session<T> {
    /// Similar to [`Option::take`], the fields are taken out of the [`Inner`]
    /// struct and returned, leaving a "taken" state in its place.
    #[inline]
    #[must_use]
    pub(crate) fn take(&self) -> Inner<T> {
        self.inner.lock().take()
    }
}

impl<T> Session<T> {
    #[inline]
    fn new(session_key: SessionKey, record: Record<T>) -> Session<T> {
        let inner = Inner {
            session_key: Some(session_key),
            data: Some(record.data),
            expires_at: Some(record.ttl),
            status: Unchanged,
        };
        Session::from_inner(inner)
    }

    #[inline]
    fn empty() -> Session<T> {
        let inner = Inner {
            session_key: None,
            data: None,
            expires_at: None,
            status: Unchanged,
        };
        Session::from_inner(inner)
    }

    fn corrupted(session_key: SessionKey) -> Session<T> {
        let inner = Inner {
            session_key: Some(session_key),
            data: None,
            expires_at: None,
            status: Unchanged,
        };
        Session::from_inner(inner)
    }

    #[inline]
    fn from_inner(inner: Inner<T>) -> Session<T> {
        Session {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl<T> Clone for Session<T> {
    #[inline]
    fn clone(&self) -> Self {
        Session {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> fmt::Debug for Session<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner.try_lock() {
            Some(guard) => f
                .debug_struct("Session")
                .field("data", &guard.data)
                .field("expires_at", &guard.expires_at)
                .field("status", &guard.status)
                .finish(),
            None => f.write_str("Session(<locked>)"),
        }
    }
}

impl<T> Inner<T> {
    #[inline]
    fn renewed(&mut self) {
        if matches!(self.status, Unchanged) {
            self.status = Renewed;
        }
    }

    #[inline]
    fn changed(&mut self) {
        if matches!(self.status, Unchanged | Renewed) {
            self.status = Changed;
        }
    }

    #[inline]
    fn purged(&mut self) {
        if matches!(self.status, Unchanged | Renewed | Changed) {
            self.status = Purged;
        }
    }

    #[inline]
    fn is_taken(&self) -> bool {
        matches!(self.status, Taken)
    }

    /// Similar to [`Option::take`], the fields are taken out of the struct and
    /// returned, leaving a "taken" state in its place.
    #[inline]
    #[must_use]
    fn take(&mut self) -> Inner<T> {
        std::mem::replace(
            self,
            Inner {
                session_key: None,
                data: None,
                expires_at: None,
                status: Taken,
            },
        )
    }

    /// Sync this session to the passed session store, if it needs syncing.
    ///
    /// This method should be called on the return value of [`Session::take`].
    /// We need to `take` the data, since borrowing it from `Session` requires
    /// holding a mutex lock across an await point. (Using the `Session` after
    /// this function is called would be a bug, in any case.)
    ///
    /// # Panics
    ///
    /// If this function is called when `status` is [`Status::Taken`], it will
    /// panic.
    pub(crate) async fn sync(
        self,
        store: &impl SessionStore<T>,
    ) -> Result<SyncAction, tower_sesh_core::store::Error> {
        // FIXME: Determine proper `ttl`.
        let ttl = now() + Duration::from_secs(10 * 60 * 60);

        match (self.status, self.session_key, self.data) {
            (Renewed, Some(session_key), _) => {
                store.update_ttl(&session_key, ttl).await?;
                Ok(SyncAction::Set(session_key))
            }
            (Changed, Some(session_key), Some(data)) => {
                store.update(&session_key, &data, ttl).await?;
                Ok(SyncAction::Set(session_key))
            }
            (Changed, None, Some(data)) => {
                let session_key = store.create(&data, ttl).await?;
                Ok(SyncAction::Set(session_key))
            }
            (Changed, Some(session_key), None) | (Purged, Some(session_key), _) => {
                store.delete(&session_key).await?;
                Ok(SyncAction::Remove)
            }
            (Unchanged, _, _) | (Renewed, None, _) | (Changed, None, None) | (Purged, None, _) => {
                Ok(SyncAction::None)
            }
            (Taken, _, _) => {
                unreachable!("`sync` called in `Taken` state. This is a bug.")
            }
        }
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
impl<S, T> axum::extract::FromRequestParts<S> for Session<T>
where
    T: Send + Sync + 'static,
    S: Sync,
{
    type Rejection = SessionRejection;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        match lazy::get_or_init(&parts.extensions).await {
            Ok(Some(session)) => Ok(session.clone()),
            Ok(None) => Err(SessionRejection),
            // Panic because this indicates a bug in the program rather than an
            // expected failure.
            Err(_) => panic!(
                "Extractor `{}` failed because of a missing request extension. \n\
                `SessionLayer` must be called before the `Session` extractor, \
                and the store implementing `SessionStore<T>`'s type parameter \
                `T` must be `{}`.",
                std::any::type_name::<Self>(),
                std::any::type_name::<T>()
            ),
        }
    }
}

/// A RAII mutex guard holding a lock to a mutex contained in `Session<T>`. The
/// data `T` can be accessed through this guard via its [`Deref`] and
/// [`DerefMut`] implementations.
///
/// The lock is automatically released whenever the guard is dropped.
///
/// This structure is created by methods defined on [`Session`], such as
/// [`insert`].
///
/// [`insert`]: Session::insert
//
// # Invariants
//
// 1. When constructing `SessionGuard`, the `data` contained within
//    `SessionInner` must contain a `Some` variant. This invariant must be met
//    while the mutex lock is held.
// 2. After the previous invariant is met, and until the `SessionGuard` is
//    dropped, the lock must never be released and `data` must never be replaced
//    with `None`.
pub struct SessionGuard<'a, T: 'a>(MutexGuard<'a, Inner<T>>);

impl<'a, T: 'a> SessionGuard<'a, T> {
    /// # Safety
    ///
    /// The caller of this method must uphold `SessionGuard`'s safety
    /// invariants. See the comment above [`SessionGuard`].
    #[inline]
    #[track_caller]
    unsafe fn new(guard: MutexGuard<'a, Inner<T>>) -> Self {
        // This is ensured by the calling contexts.
        debug_assert!(guard.data.is_some());

        SessionGuard(guard)
    }
}

impl<'a, T: 'a> Deref for SessionGuard<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: The safety invariants of `SessionGuard` must be upheld when
        // `SessionGuard` is constructed.
        unsafe { self.0.data.as_ref().unwrap_unchecked() }
    }
}

impl<'a, T: 'a> DerefMut for SessionGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.changed();

        // SAFETY: The safety invariants of `SessionGuard` must be upheld when
        // `SessionGuard` is constructed.
        unsafe { self.0.data.as_mut().unwrap_unchecked() }
    }
}

impl<T> fmt::Debug for SessionGuard<'_, T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Display for SessionGuard<'_, T>
where
    T: fmt::Display,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

/// A RAII mutex guard holding a lock to a mutex contained in `Session<T>`. The
/// data `Option<T>` can be accessed through this guard via its [`Deref`] and
/// [`DerefMut`] implementations.
///
/// The lock is automatically released whenever the guard is dropped.
///
/// This structure is created by the [`get`] method on [`Session`].
///
/// [`get`]: Session::get
pub struct OptionSessionGuard<'a, T: 'a>(MutexGuard<'a, Inner<T>>);

impl<'a, T: 'a> OptionSessionGuard<'a, T> {
    #[inline]
    fn new(guard: MutexGuard<'a, Inner<T>>) -> Self {
        OptionSessionGuard(guard)
    }
}

impl<'a, T: 'a> Deref for OptionSessionGuard<'a, T> {
    type Target = Option<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}

impl<'a, T: 'a> DerefMut for OptionSessionGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.changed();

        &mut self.0.data
    }
}

impl<T> fmt::Debug for OptionSessionGuard<'_, T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

pub(crate) mod lazy {
    use std::{error::Error as StdError, fmt, sync::Arc};

    use async_once_cell::OnceCell;
    use cookie::Cookie;
    use futures_util::future;
    use http::Extensions;
    use tower_sesh_core::{store::ErrorKind, SessionKey, SessionStore};

    use super::Session;

    #[track_caller]
    pub(crate) fn insert<T>(
        extensions: &mut Extensions,
        cookie: Option<Cookie<'static>>,
        store: &Arc<impl SessionStore<T>>,
    ) -> LazySessionHandle<T>
    where
        T: 'static + Send,
    {
        debug_assert!(
            extensions.get::<LazySession<T>>().is_none(),
            "`tower_sesh::session::lazy::insert` was called more than once!"
        );

        let lazy_session = match cookie {
            Some(cookie) => LazySession::new(cookie, Arc::clone(store)),
            None => LazySession::empty(),
        };
        let handle = lazy_session.handle();
        extensions.insert::<LazySession<T>>(lazy_session);

        handle
    }

    pub(super) async fn get_or_init<T>(
        extensions: &Extensions,
    ) -> Result<Option<&Session<T>>, Error>
    where
        T: 'static + Send,
    {
        match extensions.get::<LazySession<T>>() {
            Some(lazy_session) => Ok(lazy_session.get_or_init().await),
            None => Err(Error),
        }
    }

    enum LazySession<T> {
        Empty {
            session_cell: Arc<OnceCell<Session<T>>>,
        },
        Load {
            cookie: Cookie<'static>,
            store: Arc<dyn SessionStore<T> + 'static>,
            session_cell: Arc<OnceCell<Option<Session<T>>>>,
        },
    }

    pub(crate) enum LazySessionHandle<T> {
        Empty(Arc<OnceCell<Session<T>>>),
        Load(Arc<OnceCell<Option<Session<T>>>>),
    }

    impl<T> Clone for LazySession<T> {
        fn clone(&self) -> Self {
            match self {
                LazySession::Empty { session_cell } => LazySession::Empty {
                    session_cell: Arc::clone(session_cell),
                },
                LazySession::Load {
                    cookie,
                    store,
                    session_cell,
                } => LazySession::Load {
                    cookie: cookie.clone(),
                    store: Arc::clone(store),
                    session_cell: Arc::clone(session_cell),
                },
            }
        }
    }

    impl<T> LazySession<T>
    where
        T: 'static,
    {
        #[inline]
        fn new(cookie: Cookie<'static>, store: Arc<impl SessionStore<T>>) -> LazySession<T> {
            LazySession::Load {
                cookie,
                store,
                session_cell: Arc::new(OnceCell::new()),
            }
        }

        #[inline]
        fn empty() -> LazySession<T> {
            LazySession::Empty {
                session_cell: Arc::new(OnceCell::new()),
            }
        }

        async fn get_or_init(&self) -> Option<&Session<T>> {
            match self {
                LazySession::Empty { session_cell } => Some(
                    session_cell
                        .get_or_init(future::ready(Session::empty()))
                        .await,
                ),
                LazySession::Load {
                    cookie,
                    store,
                    session_cell,
                } => session_cell
                    .get_or_init(init_session(cookie, store.as_ref()))
                    .await
                    .as_ref(),
            }
        }

        fn handle(&self) -> LazySessionHandle<T> {
            match self {
                LazySession::Empty { session_cell } => {
                    LazySessionHandle::Empty(Arc::clone(session_cell))
                }
                LazySession::Load { session_cell, .. } => {
                    LazySessionHandle::Load(Arc::clone(session_cell))
                }
            }
        }
    }

    async fn init_session<T>(
        cookie: &Cookie<'static>,
        store: &dyn SessionStore<T>,
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
            Err(err) => match err.kind() {
                ErrorKind::Serde(_) => Some(Session::corrupted(session_key)),
                _ => {
                    error!(
                        err = %tower_sesh_core::util::Report::new(err),
                        "error loading session"
                    );
                    None
                }
            },
        }
    }

    impl<T> LazySessionHandle<T> {
        pub(crate) fn get(&self) -> Option<&Session<T>> {
            match self {
                LazySessionHandle::Empty(session_cell) => session_cell.get(),
                LazySessionHandle::Load(session_cell) => {
                    session_cell.get().and_then(Option::as_ref)
                }
            }
        }
    }

    pub(crate) struct Error;

    impl StdError for Error {}

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
