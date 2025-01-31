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
            .ok_or(SessionRejection)
    }
}

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "Missing request extension"]
    /// Rejection for [`Session`] if an expected request extension
    /// was not found.
    pub struct SessionRejection;
}

pub(crate) mod lazy {
    use std::sync::Arc;

    use async_once_cell::OnceCell;
    use cookie::Cookie;
    use http::Extensions;
    use tower_sesh_core::{SessionKey, SessionStore};

    use super::Session;

    pub(crate) fn insert<T>(
        cookie: Cookie<'static>,
        store: Arc<dyn SessionStore<T>>,
        extensions: &mut Extensions,
    ) where
        T: 'static + Send,
    {
        let lazy_session = LazySession::new(cookie, store);
        extensions.insert::<LazySession<T>>(lazy_session);
    }

    pub(super) async fn get_or_init<T>(extensions: &mut Extensions) -> Option<Session<T>>
    where
        T: 'static + Send,
    {
        let session = match extensions.get::<LazySession<T>>() {
            Some(lazy_session) => lazy_session.get_or_init().await.cloned()?,
            None => Session::empty(),
        };

        Some(session)
    }

    struct LazySession<T> {
        cookie: Cookie<'static>,
        store: Arc<dyn SessionStore<T> + 'static>,
        session: Arc<OnceCell<Option<Session<T>>>>,
    }

    impl<T> Clone for LazySession<T> {
        fn clone(&self) -> Self {
            LazySession {
                cookie: self.cookie.clone(),
                store: Arc::clone(&self.store),
                session: Arc::clone(&self.session),
            }
        }
    }

    impl<T> LazySession<T>
    where
        T: 'static,
    {
        fn new(cookie: Cookie<'static>, store: Arc<dyn SessionStore<T>>) -> LazySession<T> {
            LazySession {
                cookie,
                store,
                session: Arc::new(OnceCell::new()),
            }
        }

        async fn get_or_init(&self) -> Option<&Session<T>> {
            let init = async {
                let session_key = match SessionKey::decode(self.cookie.value()) {
                    Ok(session_key) => session_key,
                    Err(_) => return Some(Session::empty()),
                };

                match self.store.load(&session_key).await {
                    Ok(Some(data)) => Some(Session::new(session_key, todo!())),
                    Ok(None) => Some(Session::empty()),
                    // TODO: We may want to ignore some types of errors here.
                    Err(err) => {
                        error!(%err);
                        None
                    }
                }
            };

            self.session.get_or_init(init).await.as_ref()
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
