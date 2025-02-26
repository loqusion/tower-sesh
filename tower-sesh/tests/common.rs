use std::marker::PhantomData;

use axum::async_trait;
use tower_sesh_core::{
    store::{self, SessionStoreImpl},
    Record, SessionKey, SessionStore, Ttl,
};

pub struct ErrStore<T = ()> {
    error_fn: Box<dyn Fn() -> store::Error + Send + Sync + 'static>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ErrStore<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() -> store::Error + Send + Sync + 'static,
    {
        ErrStore {
            error_fn: Box::new(f),
            _marker: PhantomData,
        }
    }
}

impl<T> SessionStore<T> for ErrStore<T> where T: Send + Sync + 'static {}
#[async_trait]
impl<T> SessionStoreImpl<T> for ErrStore<T>
where
    T: Send + Sync + 'static,
{
    async fn create(&self, _data: &T, _ttl: Ttl) -> Result<SessionKey, store::Error> {
        Err((self.error_fn)())
    }

    async fn load(&self, _session_key: &SessionKey) -> Result<Option<Record<T>>, store::Error> {
        Err((self.error_fn)())
    }

    async fn update(
        &self,
        _session_key: &SessionKey,
        _data: &T,
        _ttl: Ttl,
    ) -> Result<(), store::Error> {
        Err((self.error_fn)())
    }

    async fn update_ttl(&self, _session_key: &SessionKey, _ttl: Ttl) -> Result<(), store::Error> {
        Err((self.error_fn)())
    }

    async fn delete(&self, _session_key: &SessionKey) -> Result<(), store::Error> {
        Err((self.error_fn)())
    }
}
