use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::{record::Record, session::SessionKey};

#[derive(Debug, thiserror::Error)]
pub enum Error {}

type Result<T, E = Error> = std::result::Result<T, E>;

#[async_trait]
pub trait SessionStore: 'static + Send + Sync {
    /// Create a new session with the provided `session_state`.
    ///
    /// Returns the [`SessionKey`] for the newly created session.
    ///
    /// [`SessionKey`]: crate::session::SessionKey
    async fn create(&self, record: Record) -> Result<SessionKey>;

    /// Load the session state associated with a session key.
    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record>>;

    /// Update an existing session associated with `session_key` with the
    /// provided `record`.
    ///
    /// If such a session does not exist, it will be created.
    async fn update(&self, session_key: &SessionKey, record: Record) -> Result<()>;

    /// Delete the session associated with `session_key`.
    ///
    /// If no such session exists, this is a no-op.
    async fn delete(&self, session_key: &SessionKey) -> Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryStore(Arc<parking_lot::Mutex<HashMap<SessionKey, Record>>>);

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn create(&self, record: Record) -> Result<SessionKey> {
        let mut store_guard = self.0.lock();
        todo!()
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record>> {
        todo!()
    }

    async fn update(&self, session_key: &SessionKey, record: Record) -> Result<()> {
        todo!()
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        self.0.lock().remove(session_key);
        Ok(())
    }
}

pub struct CachingStore<Cache: SessionStore, Store: SessionStore> {
    cache: Cache,
    store: Store,
}

impl<Cache: SessionStore, Store: SessionStore> CachingStore<Cache, Store> {
    pub fn from_cache_and_store(cache: Cache, store: Store) -> Self {
        Self { cache, store }
    }
}

#[async_trait]
impl<Cache: SessionStore, Store: SessionStore> SessionStore for CachingStore<Cache, Store> {
    async fn create(&self, record: Record) -> Result<SessionKey> {
        let session_key = SessionKey::generate();

        let store_fut = self.store.update(&session_key, record.clone());
        let cache_fut = self.cache.update(&session_key, record);

        futures::try_join!(store_fut, cache_fut)?;

        Ok(session_key)
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record>> {
        match self.cache.load(session_key).await {
            Ok(Some(record)) => Ok(Some(record)),
            Ok(None) | Err(_) => {
                let record = self.store.load(session_key).await?;

                if let Some(record) = record.as_ref() {
                    let _ = self.cache.update(session_key, record.clone()).await;
                }

                Ok(record)
            }
        }
    }

    async fn update(&self, session_key: &SessionKey, record: Record) -> Result<()> {
        #![allow(clippy::clone_on_copy)]

        let store_fut = self.store.update(session_key, record.clone());
        let cache_fut = self.cache.update(session_key, record);

        futures::try_join!(store_fut, cache_fut)?;

        Ok(())
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        let store_fut = self.store.delete(session_key);
        let cache_fut = self.cache.delete(session_key);

        futures::try_join!(store_fut, cache_fut)?;

        Ok(())
    }
}
