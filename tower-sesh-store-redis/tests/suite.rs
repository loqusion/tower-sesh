#![cfg(feature = "test-util")]

use std::{env, sync::LazyLock, time::Duration};

use redis::aio::ConnectionManagerConfig;
use serde::{de::DeserializeOwned, Serialize};
use tower_sesh_core::{store::SessionStoreRng, SessionStore};
use tower_sesh_store_redis::RedisStore;

static REDIS_URL: LazyLock<&'static str> = LazyLock::new(|| {
    env::var("REDIS_URL")
        .expect("`REDIS_URL` environment variable must be set")
        .leak()
});

async fn store<T, Rng>() -> impl SessionStore<T> + SessionStoreRng<Rng>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
    Rng: rand::CryptoRng + Send + 'static,
{
    RedisStore::with_config(
        REDIS_URL.clone(),
        ConnectionManagerConfig::new()
            .set_connection_timeout(Duration::from_secs(5))
            .set_number_of_retries(1),
    )
    .await
    .expect("failed to connect to redis")
}

#[cfg(not(tower_sesh_test_caching_store))]
tower_sesh_test::test_suite!(store().await);
#[cfg(tower_sesh_test_caching_store)]
tower_sesh_test::test_suite!(tower_sesh::store::CachingStore::from_cache_and_store(
    tower_sesh::store::MemoryStore::new(),
    store().await
));
