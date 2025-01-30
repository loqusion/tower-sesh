use std::{
    env,
    sync::atomic::{self, AtomicU64},
    time::Duration,
};

use redis::aio::ConnectionManagerConfig;
use tower_sesh_core::{SessionKey, SessionStore};
use tower_sesh_store_redis::RedisStore;

/// A session key that is safe to use in tests without fear of collisions.
///
/// Collisions can cause tests to be flaky, since two tests using the same
/// session key can interact with each other in unexpected ways. For
/// instance, one test can delete the session state of another test and
/// cause a test assertion to fail.
///
/// Actually, a CSPRNG is suitable for this purpose, as collisions for
/// values in the range 1..2^128 are _exceedingly_ rare. Still, the
/// probability of collision is non-zero.
pub fn test_key() -> SessionKey {
    static KEY_STATE: AtomicU64 = AtomicU64::new(1);
    let v = KEY_STATE.fetch_add(1, atomic::Ordering::SeqCst) as u128;
    SessionKey::try_from(v).unwrap()
}

async fn store<T>() -> RedisStore<T> {
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
