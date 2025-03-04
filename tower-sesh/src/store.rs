use std::{fmt, marker::PhantomData};

use async_trait::async_trait;
#[cfg(feature = "memory-store")]
use dashmap::DashMap;
use tower_sesh_core::{
    store::{Error, SessionStoreImpl},
    time::now,
    Record, SessionKey, SessionStore, Ttl,
};

type Result<T, E = Error> = std::result::Result<T, E>;

const MAX_ITERATIONS: usize = 8;

#[cfg(feature = "memory-store")]
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
impl<T> fmt::Debug for MemoryStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MemoryStore { .. }")
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
        let record = Record::new(data.clone(), ttl);

        // Collision resolution
        // (This is statistically improbable for a sufficiently large session key)
        for _ in 0..MAX_ITERATIONS {
            let session_key = rand::random::<SessionKey>();
            match self.map.entry(session_key.clone()) {
                dashmap::Entry::Vacant(entry) => {
                    entry.insert(record);
                    return Ok(session_key);
                }
                dashmap::Entry::Occupied(_) => continue,
            }
        }

        Err(Error::max_iterations_reached())
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        let record = self
            .map
            .get(session_key)
            .as_deref()
            .cloned()
            .filter(|record| record.ttl >= now());
        Ok(record)
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

impl<T, Cache: SessionStore<T>, Store: SessionStore<T>> fmt::Debug for CachingStore<T, Cache, Store>
where
    Cache: fmt::Debug,
    Store: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachingStore")
            .field("cache", &self.cache)
            .field("store", &self.store)
            .finish()
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
