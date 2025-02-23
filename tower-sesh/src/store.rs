use std::marker::PhantomData;

use async_trait::async_trait;
#[cfg(feature = "memory-store")]
use dashmap::DashMap;
use tower_sesh_core::{
    store::{Error, SessionStoreImpl, Ttl},
    Record, SessionKey, SessionStore,
};

type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(feature = "memory-store")]
#[derive(Clone)]
pub struct MemoryStore<T> {
    map: DashMap<SessionKey, Record<T>>,
}

#[cfg(feature = "memory-store")]
impl<T> Default for MemoryStore<T> {
    fn default() -> Self {
        MemoryStore {
            map: DashMap::new(),
        }
    }
}

#[cfg(feature = "memory-store")]
impl<T> MemoryStore<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(feature = "memory-store")]
impl<T> SessionStore<T> for MemoryStore<T> where T: 'static + Send + Sync + Clone {}

#[cfg(feature = "memory-store")]
#[async_trait]
impl<T> SessionStoreImpl<T> for MemoryStore<T>
where
    T: 'static + Send + Sync + Clone,
{
    async fn create(&self, data: &T, ttl: Ttl) -> Result<SessionKey> {
        let session_key = rand::random();
        self.update(&session_key, data, ttl).await?;
        Ok(session_key)
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        Ok(self.map.get(session_key).as_deref().cloned())
    }

    async fn update(&self, session_key: &SessionKey, data: &T, ttl: Ttl) -> Result<()> {
        let record = Record::new(data.clone(), ttl);
        self.map.insert(session_key.clone(), record);
        Ok(())
    }

    async fn update_ttl(&self, session_key: &SessionKey, ttl: Ttl) -> Result<()> {
        if let Some(mut record) = self.map.get_mut(session_key) {
            record.ttl = ttl;
        }
        Ok(())
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        self.map.remove(session_key);
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
