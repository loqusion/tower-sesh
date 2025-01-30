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
#[async_trait]
pub trait SessionStore<Data>: 'static + Send + Sync {
    /// Create a new session with the provided `session_state`.
    ///
    /// Returns the [`SessionKey`] for the newly created session.
    ///
    /// [`SessionKey`]: crate::session::SessionKey
    async fn create(&self, record: &Record<Data>) -> Result<SessionKey>;

    /// Load the session state associated with a session key.
    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<Data>>>;

    /// Update an existing session associated with `session_key` with the
    /// provided `record`.
    ///
    /// If such a session does not exist, it will be created.
    async fn update(&self, session_key: &SessionKey, record: &Record<Data>) -> Result<()>;

    /// Delete the session associated with `session_key`.
    ///
    /// If no such session exists, this is a no-op.
    async fn delete(&self, session_key: &SessionKey) -> Result<()>;
}

#[derive(Clone)]
pub struct MemoryStore<Data>(Arc<parking_lot::Mutex<HashMap<SessionKey, Record<Data>>>>);

impl<Data> Default for MemoryStore<Data> {
    fn default() -> Self {
        let store = HashMap::new();
        MemoryStore(Arc::new(parking_lot::Mutex::new(store)))
    }
}

impl<Data> MemoryStore<Data> {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl<Data> SessionStore<Data> for MemoryStore<Data>
where
    Data: 'static + Send + Sync,
{
    async fn create(&self, record: &Record<Data>) -> Result<SessionKey> {
        let mut store_guard = self.0.lock();
        todo!()
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<Data>>> {
        todo!()
    }

    async fn update(&self, session_key: &SessionKey, record: &Record<Data>) -> Result<()> {
        todo!()
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        self.0.lock().remove(session_key);
        Ok(())
    }
}

pub struct CachingStore<Data, Cache: SessionStore<Data>, Store: SessionStore<Data>> {
    cache: Cache,
    store: Store,
    _marker: PhantomData<fn() -> Data>,
}

impl<Data, Cache: SessionStore<Data>, Store: SessionStore<Data>> CachingStore<Data, Cache, Store> {
    pub fn from_cache_and_store(cache: Cache, store: Store) -> Self {
        Self {
            cache,
            store,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<Data, Cache: SessionStore<Data>, Store: SessionStore<Data>> SessionStore<Data>
    for CachingStore<Data, Cache, Store>
where
    Data: 'static + Send + Sync,
{
    async fn create(&self, record: &Record<Data>) -> Result<SessionKey> {
        let session_key = SessionKey::generate();

        let store_fut = self.store.update(&session_key, record);
        let cache_fut = self.cache.update(&session_key, record);

        futures::try_join!(store_fut, cache_fut)?;

        Ok(session_key)
    }

    async fn load(&self, session_key: &SessionKey) -> Result<Option<Record<Data>>> {
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

    async fn update(&self, session_key: &SessionKey, record: &Record<Data>) -> Result<()> {
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
pub struct Record<Data> {
    data: Data,
    expiry: OffsetDateTime,
}

impl<Data> Record<Data> {
    pub fn unix_timestamp(&self) -> i64 {
        self.expiry.unix_timestamp()
    }
}
