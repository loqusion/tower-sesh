use std::{env, time::Duration};

use redis::aio::ConnectionManagerConfig;
use tower_sesh::{
    test::{test_key, test_suite},
    SessionStore,
};
use tower_sesh_store_redis::RedisStore;

async fn store<Data>() -> RedisStore<Data> {
    let url =
        env::var("REDIS_URL").expect("REDIS_URL environment variable must be set to run tests");

    let config = ConnectionManagerConfig::new()
        .set_connection_timeout(Duration::from_secs(5))
        .set_number_of_retries(1);

    let client = redis::Client::open(url).expect("failed to connect to redis");
    RedisStore::with_connection_manager_config(client, config)
        .await
        .expect("failed to connect to redis")
}

#[tokio::test]
async fn smoke() {
    let _ = store::<()>().await;
}

#[tokio::test]
async fn loading_a_missing_session_returns_none() -> anyhow::Result<()> {
    let store = store::<()>().await;
    let session_key = test_key();

    let record = store.load(&session_key).await?;
    assert!(record.is_none(), "expected no record");

    Ok(())
}

test_suite!(store::<()>().await);
