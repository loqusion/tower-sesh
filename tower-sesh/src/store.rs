use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use parking_lot::Mutex;
use tower_sesh_core::{
    store::{Error, SessionStoreImpl, Ttl},
    Record, SessionKey,
};

pub use tower_sesh_core::SessionStore;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct MemoryStore<T>(Arc<Mutex<HashMap<SessionKey, Record<T>>>>);

impl<T> Default for MemoryStore<T> {
    fn default() -> Self {
        let store = HashMap::new();
        MemoryStore(Arc::new(Mutex::new(store)))
    }
}

impl<T> MemoryStore<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> SessionStore<T> for MemoryStore<T> where T: 'static + Send + Sync + Clone {}

#[async_trait]
impl<T> SessionStoreImpl<T> for MemoryStore<T>
where
    T: 'static + Send + Sync + Clone,
{
    async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey> {
        let session_key = SessionKey::generate();
        self.update(&session_key, data, ttl).await?;
        Ok(session_key)
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        let store_guard = self.0.lock();
        Ok(store_guard.get(session_key).cloned())
    }

    async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()> {
        let record = Record::new(data.clone(), ttl);
        self.0.lock().insert(session_key.clone(), record);
        Ok(())
    }

    async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()> {
        if let Some(record) = self.0.lock().get_mut(session_key) {
            record.ttl = ttl;
        }
        Ok(())
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        self.0.lock().remove(session_key);
        Ok(())
    }
}

pub struct CachingStore<T, Cache: SessionStore<T>, Store: SessionStore<T>> {
    cache: Cache,
    store: Store,
    _marker: PhantomData<fn() -> T>,
}

impl<T, Cache: SessionStore<T>, Store: SessionStore<T>> CachingStore<T, Cache, Store> {
    pub fn from_cache_and_store(cache: Cache, store: Store) -> Self {
        Self {
            cache,
            store,
            _marker: PhantomData,
        }
    }
}

impl<T, Cache: SessionStore<T>, Store: SessionStore<T>> SessionStore<T>
    for CachingStore<T, Cache, Store>
where
    T: 'static + Send + Sync,
{
}

#[async_trait]
impl<T, Cache: SessionStore<T>, Store: SessionStore<T>> SessionStoreImpl<T>
    for CachingStore<T, Cache, Store>
where
    T: 'static + Send + Sync,
{
    async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey> {
        let session_key = self.store.create(data, ttl).await?;
        self.cache.update(&session_key, data, ttl).await?;

        Ok(session_key)
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        match self.cache.load(session_key).await {
            Ok(Some(record)) => Ok(Some(record)),
            Ok(None) | Err(_) => {
                let record = self.store.load(session_key).await?;

                if let Some(record) = record.as_ref() {
                    let _ = self
                        .cache
                        .update(session_key, &record.data, record.ttl)
                        .await;
                }

                Ok(record)
            }
        }
    }

    async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()> {
        let store_fut = self.store.update(session_key, data, ttl);
        let cache_fut = self.cache.update(session_key, data, ttl);

        futures::try_join!(store_fut, cache_fut)?;

        Ok(())
    }

    async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()> {
        let store_fut = self.store.update_ttl(session_key, ttl);
        let cache_fut = self.cache.update_ttl(session_key, ttl);

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
