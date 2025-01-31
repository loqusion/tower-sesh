use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use parking_lot::Mutex;
use tower_sesh_core::{store::Error, Record, SessionKey};

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

#[async_trait]
impl<T> SessionStore<T> for MemoryStore<T>
where
    T: 'static + Send + Sync + Clone,
{
    async fn create(&self, record: &Record<T>) -> Result<SessionKey> {
        let session_key = SessionKey::generate();
        self.update(&session_key, record).await?;
        Ok(session_key)
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        let store_guard = self.0.lock();
        Ok(store_guard.get(session_key).cloned())
    }

    async fn update(&self, session_key: &SessionKey, record: &Record<T>) -> Result<()> {
        self.0.lock().insert(session_key.clone(), record.clone());
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

#[async_trait]
impl<T, Cache: SessionStore<T>, Store: SessionStore<T>> SessionStore<T>
    for CachingStore<T, Cache, Store>
where
    T: 'static + Send + Sync,
{
    // FIXME: This has correctness issues.
    async fn create(&self, record: &Record<T>) -> Result<SessionKey> {
        let session_key = SessionKey::generate();

        let store_fut = self.store.update(&session_key, record);
        let cache_fut = self.cache.update(&session_key, record);

        futures::try_join!(store_fut, cache_fut)?;

        Ok(session_key)
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        match self.cache.load(session_key).await {
            Ok(Some(record)) => Ok(Some(record)),
            Ok(None) | Err(_) => {
                let record = self.store.load(session_key).await?;

                if let Some(record) = record.as_ref() {
                    let _ = self.cache.update(session_key, record).await;
                }

                Ok(record)
            }
        }
    }

    async fn update(&self, session_key: &SessionKey, record: &Record<T>) -> Result<()> {
        #![allow(clippy::clone_on_copy)]

        let store_fut = self.store.update(session_key, record);
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
