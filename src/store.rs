use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::session::{Id, Record};

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait SessionStore: 'static + Send + Sync {
    async fn create(&self, record: &mut Record) -> Result<()>;

    async fn save(&self, record: &Record) -> Result<()>;

    async fn load(&self, id: &Id) -> Result<Option<Record>>;

    async fn delete(&self, id: &Id) -> Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryStore(Arc<parking_lot::Mutex<HashMap<Id, Record>>>);

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn create(&self, record: &mut Record) -> Result<()> {
        let mut store_guard = self.0.lock();
        todo!();
    }

    async fn save(&self, record: &Record) -> Result<()> {
        todo!();
        Ok(())
    }

    async fn load(&self, session_id: &Id) -> Result<Option<Record>> {
        todo!()
    }

    async fn delete(&self, id: &Id) -> Result<()> {
        self.0.lock().remove(id);
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
    async fn create(&self, record: &mut Record) -> Result<()> {
        self.store.create(record).await?;
        self.cache.create(record).await?;
        Ok(())
    }

    async fn save(&self, record: &Record) -> Result<()> {
        let store_save_fut = self.store.save(record);
        let cache_save_fut = self.cache.save(record);

        futures::try_join!(store_save_fut, cache_save_fut)?;

        Ok(())
    }

    async fn load(&self, id: &Id) -> Result<Option<Record>> {
        match self.cache.load(id).await {
            Ok(Some(record)) => Ok(Some(record)),
            Ok(None) => {
                let record = self.store.load(id).await?;

                if let Some(record) = record.as_ref() {
                    self.cache.save(record).await?;
                }

                Ok(record)
            }
            Err(err) => Err(err),
        }
    }

    async fn delete(&self, id: &Id) -> Result<()> {
        let store_delete_fut = self.store.delete(id);
        let cache_delete_fut = self.cache.delete(id);

        futures::try_join!(store_delete_fut, cache_delete_fut)?;

        Ok(())
    }
}
