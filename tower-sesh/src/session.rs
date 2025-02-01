use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use tower_sesh_core::SessionKey;

pub struct Session<T>(Arc<Mutex<SessionInner<T>>>);

impl<T> Clone for Session<T> {
    fn clone(&self) -> Self {
        Session(Arc::clone(&self.0))
    }
}

struct SessionInner<T> {
    session_id: Option<SessionKey>,
    data: Option<T>,
    status: SessionStatus,
}

enum SessionStatus {
    Unchanged,
    Renewed,
    Changed,
    Purged,
}
use SessionStatus::*;

impl<T> Session<T> {
    fn new(session_key: SessionKey, data: T) -> Session<T> {
        let inner = SessionInner {
            session_id: Some(session_key),
            data: Some(data),
            status: Unchanged,
        };
        Session(Arc::new(Mutex::new(inner)))
    }

    fn empty() -> Session<T> {
        let inner = SessionInner {
            session_id: None,
            data: None,
            status: Unchanged,
        };
        Session(Arc::new(Mutex::new(inner)))
    }
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
        lazy::get_or_init(&mut parts.extensions)
            .await
            .unwrap_or_else(|_| {
                // Panic because this indicates a bug in the program rather
                // than an expected failure.
                panic!(
                    "Missing request extension. `SessionManagerLayer` must be \
                    called before the `Session` extractor is run. Also, check \
                    that the generic type for `Session<T>` is correct."
                )
            })
            .ok_or(SessionRejection)
    }
}

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "Failed to load session"]
    /// Rejection for [`Session`] if an unrecoverable error occurred when
    /// loading the session.
    pub struct SessionRejection;
}

pub(crate) mod lazy {
    use std::{error::Error as StdError, fmt, sync::Arc};

    use async_once_cell::OnceCell;
    use cookie::Cookie;
    use http::Extensions;
    use tower_sesh_core::{SessionKey, SessionStore};

    use super::Session;

    pub(crate) fn insert<T>(
        cookie: Option<Cookie<'static>>,
        store: &Arc<impl SessionStore<T>>,
        extensions: &mut Extensions,
    ) where
        T: 'static + Send,
    {
        debug_assert!(
            extensions.get::<LazySession<T>>().is_none(),
            "`session::lazy::insert` was called more than once!"
        );

        let lazy_session = match cookie {
            Some(cookie) => LazySession::new(cookie, Arc::clone(store)),
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
                } => LazySession::Init {
                    cookie: cookie.clone(),
                    store: Arc::clone(store),
                    session: Arc::clone(session),
                },
            }
        }
    }

    impl<T> LazySession<T>
    where
        T: 'static,
    {
        fn new(cookie: Cookie<'static>, store: Arc<impl SessionStore<T>>) -> LazySession<T> {
            LazySession::Init {
                cookie,
                store,
                session: Arc::new(OnceCell::new()),
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
                } => session
                    .get_or_init(init_session(cookie, store.as_ref()))
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
    ) -> Option<Session<T>>
    where
        T: 'static,
    {
        let session_key = match SessionKey::decode(cookie.value()) {
            Ok(session_key) => session_key,
            Err(_) => return Some(Session::empty()),
        };

        match store.load(&session_key).await {
            Ok(Some(data)) => Some(Session::new(session_key, todo!())),
            Ok(None) => Some(Session::empty()),
            // TODO: We may want to ignore some types of errors here and
            // simply return an empty session.
            Err(err) => {
                // TODO: Better error reporting
                error!(%err);
                None
            }
        }
    }

    pub(crate) struct Error;

    impl StdError for Error {
        fn cause(&self) -> Option<&dyn StdError> {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn traits() {
        fn assert_send<T: Send>() {}
        assert_send::<Session<()>>();
    }
}
