use std::{env, sync::LazyLock};

use tower_sesh_store_redis::RedisStore;

static REDIS_URL: LazyLock<&'static str> = LazyLock::new(|| {
    env::var("REDIS_URL")
        .expect("`REDIS_URL` environment variable must be set")
        .leak()
});

tower_sesh_test::test_suite!(RedisStore::open(REDIS_URL.clone()).await.unwrap());
