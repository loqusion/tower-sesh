#![cfg(feature = "test-util")]

use std::{
    env,
    sync::{
        atomic::{AtomicU64, Ordering::SeqCst},
        LazyLock,
    },
};

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use tower_sesh_store_redis::RedisStore;

static REDIS_URL: LazyLock<&'static str> = LazyLock::new(|| {
    env::var("REDIS_URL")
        .expect("`REDIS_URL` environment variable must be set")
        .leak()
});

static SEED: AtomicU64 = AtomicU64::new(0);

tower_sesh_test::test_suite!(RedisStore::open(REDIS_URL.clone())
    .await
    .unwrap()
    .rng(ChaCha20Rng::seed_from_u64(SEED.fetch_add(1, SeqCst))));
