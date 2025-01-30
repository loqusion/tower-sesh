use std::sync::Arc;

use async_trait::async_trait;
use http::Extensions;
use parking_lot::Mutex;
use tower_sesh_core::SessionKey;

pub struct Session<T>(Arc<Mutex<SessionInner<T>>>);

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

impl<T> Session<T>
where
    T: 'static + Send + Sync,
{
    pub(crate) fn extract(extensions: &mut Extensions) -> Option<Self> {
        extensions
            .get::<Arc<Mutex<SessionInner<T>>>>()
            .cloned()
            .map(Session)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetError {}
#[derive(Debug, thiserror::Error)]
pub enum InsertError {}
#[derive(Debug, thiserror::Error)]
pub enum RemoveError {}

impl<T> Clone for Session<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
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
        Session::extract(&mut parts.extensions).ok_or(SessionRejection)
    }
}

define_rejection! {
    #[status = INTERNAL_SERVER_ERROR]
    #[body = "Missing request extension"]
    /// Rejection for [`Session`] if an expected request extension
    /// was not found.
    pub struct SessionRejection;
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
