use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use time::OffsetDateTime;

use crate::session::SessionKey;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

type Result<T, E = Error> = std::result::Result<T, E>;

// TODO: MUST mention that the data format used by a session store must be
// self-describing, i.e. it implements `Deserializer::deserialize_any`. (This
// is because `Value`'s `Deserialize::deserialize` delegates to
// `Deserializer::deserialize_any`.)
//
// TODO: `Record` should be removed because you can't construct a `Record` without
// transferring ownership or cloning.
#[async_trait]
pub trait SessionStore<T>: 'static + Send + Sync {
    /// Create a new session with the provided `session_state`.
    ///
    /// Returns the [`SessionKey`] for the newly created session.
    ///
    /// [`SessionKey`]: crate::session::SessionKey
    async fn create(&self, record: &Record<T>) -> Result<SessionKey>;

    /// Load the session state associated with a session key.
    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>>;

    /// Update an existing session associated with `session_key` with the
    /// provided `record`.
    ///
    /// If such a session does not exist, it will be created.
    async fn update(&self, session_key: &SessionKey, record: &Record<T>) -> Result<()>;

    /// Delete the session associated with `session_key`.
    ///
    /// If no such session exists, this is a no-op.
    async fn delete(&self, session_key: &SessionKey) -> Result<()>;
}

#[derive(Clone)]
pub struct MemoryStore<T>(Arc<parking_lot::Mutex<HashMap<SessionKey, Record<T>>>>);

impl<T> Default for MemoryStore<T> {
    fn default() -> Self {
        let store = HashMap::new();
        MemoryStore(Arc::new(parking_lot::Mutex::new(store)))
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
    T: 'static + Send + Sync,
{
    async fn create(&self, record: &Record<T>) -> Result<SessionKey> {
        let mut store_guard = self.0.lock();
        todo!()
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<T>>> {
        todo!()
    }

    async fn update(&self, session_key: &SessionKey, record: &Record<T>) -> Result<()> {
        todo!()
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

#[derive(Clone, Debug)]
pub struct Record<T> {
    data: T,
    expiry: OffsetDateTime,
}

impl<T> Record<T> {
    pub fn unix_timestamp(&self) -> i64 {
        self.expiry.unix_timestamp()
    }
}
