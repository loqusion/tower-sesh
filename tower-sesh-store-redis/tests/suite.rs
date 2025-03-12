#![cfg(feature = "test-util")]

use std::{env, sync::LazyLock, time::Duration};

use redis::aio::ConnectionManagerConfig;
use serde::{de::DeserializeOwned, Serialize};
use tower_sesh_core::SessionStore;
use tower_sesh_store_redis::RedisStore;

static REDIS_URL: LazyLock<&'static str> = LazyLock::new(|| {
    env::var("REDIS_URL")
        .expect("`REDIS_URL` environment variable must be set")
        .leak()
});

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
}

tower_sesh_test::test_suite!(store().await);
