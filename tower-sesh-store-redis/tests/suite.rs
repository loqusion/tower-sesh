#![cfg(feature = "test-util")]

use std::{
    env,
    sync::{
        atomic::{AtomicU64, Ordering::SeqCst},
        LazyLock,
    },
    time::Duration,
};

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use redis::aio::ConnectionManagerConfig;
use serde::{de::DeserializeOwned, Serialize};
use tower_sesh_core::SessionStore;
use tower_sesh_store_redis::RedisStore;

static REDIS_URL: LazyLock<&'static str> = LazyLock::new(|| {
    env::var("REDIS_URL")
        .expect("`REDIS_URL` environment variable must be set")
        .leak()
});

static SEED: AtomicU64 = AtomicU64::new(0);

async fn store<T>() -> impl SessionStore<T>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    RedisStore::with_config(
        REDIS_URL.clone(),
        ConnectionManagerConfig::new()
            .set_connection_timeout(Duration::from_secs(5))
            .set_number_of_retries(1),
    )
    .await
    .expect("failed to connect to redis")
    .rng(ChaCha20Rng::seed_from_u64(SEED.fetch_add(1, SeqCst)))
}

tower_sesh_test::test_suite!(store().await);
