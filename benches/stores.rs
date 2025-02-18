#![allow(clippy::disallowed_types)]

use std::{sync::Mutex, time::Duration};

use divan::black_box;
use serde::{Deserialize, Serialize};
use tower_sesh::store::MemoryStore;
use tower_sesh_core::{
    store::{SessionStoreImpl, Ttl},
    SessionKey,
};
#[cfg(feature = "store-redis")]
use tower_sesh_store_redis::RedisStore;

use build_single_rt as build_rt;

const THREADS: &[usize] = &[0, 1, 2, 4, 8, 16];

#[cfg(feature = "store-redis")]
static REDIS_URL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    std::env::var("REDIS_URL").unwrap_or_else(|err| {
        panic!("`REDIS_URL` environment variable must be set to a valid Redis URL: {err}")
    })
});

const NUM_KEYS_ERROR_MESSAGE: &str = "\
    `NUM_KEYS` is not large enough to cover all iterations\n\
    lower the iteration count with `sample_count` or `sample_size`, or increase `NUM_KEYS`\
";

fn main() {
    divan::main();
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct Simple {
    id: String,
}

impl Simple {
    fn sample() -> Simple {
        Simple {
            id: "hello, world!".to_owned(),
        }
    }
}

fn ttl_sample() -> Ttl {
    Ttl::now_utc() + Duration::from_secs(10)
}

#[divan::bench(threads = THREADS)]
fn control(bencher: divan::Bencher) {
    let rt = build_rt();

    bencher.bench(|| {
        rt.block_on(async {});
    });
}

#[divan::bench_group(threads = THREADS)]
mod create {
    use super::*;

    #[divan::bench(name = "MemoryStore")]
    fn memory_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = MemoryStore::<Simple>::new();
        let data = Simple::sample();
        let ttl = ttl_sample();

        bencher.bench(|| {
            rt.block_on(async {
                store
                    .create(black_box(&data), black_box(ttl))
                    .await
                    .unwrap();
            });
        });
    }

    #[cfg(feature = "store-redis")]
    #[divan::bench(name = "RedisStore")]
    fn redis_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = rt.block_on(build_redis_store());
        let data = Simple::sample();
        let ttl = ttl_sample();

        bencher.bench(|| {
            rt.block_on(async {
                store
                    .create(black_box(&data), black_box(ttl))
                    .await
                    .unwrap();
            });
        });
    }
}

#[divan::bench_group(threads = THREADS)]
mod load {
    use super::*;

    const NUM_KEYS: usize = 1000;

    #[divan::bench(name = "MemoryStore")]
    fn memory_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = MemoryStore::<Simple>::new();

        let keys = rt.block_on(populate_store(&store, Simple::sample, ttl_sample, NUM_KEYS));
        let keys_iter = MutexIter::new(keys.into_iter());

        bencher
            .with_inputs(|| keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE))
            .bench_values(|key| {
                rt.block_on(async {
                    let rec = store.load(&key).await.unwrap();
                    black_box(rec);
                });
            });
    }

    #[cfg(feature = "store-redis")]
    #[divan::bench(name = "RedisStore")]
    fn redis_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = rt.block_on(build_redis_store());

        let keys = rt.block_on(populate_store(&store, Simple::sample, ttl_sample, NUM_KEYS));
        let keys_iter = MutexIter::new(keys.into_iter());

        bencher
            .with_inputs(|| keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE))
            .bench_values(|key| {
                rt.block_on(async {
                    let rec = store.load(&key).await.unwrap();
                    black_box(rec);
                });
            });
    }
}

#[divan::bench_group(threads = THREADS)]
mod update {
    use super::*;

    const NUM_KEYS: usize = 1000;

    #[divan::bench(name = "MemoryStore")]
    fn memory_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = MemoryStore::<Simple>::new();

        let keys = rt.block_on(populate_store(&store, Simple::sample, ttl_sample, NUM_KEYS));
        let keys_iter = MutexIter::new(keys.into_iter());

        bencher
            .with_inputs(|| {
                let key = keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE);
                let data = Simple::sample();
                let ttl = ttl_sample();
                (key, data, ttl)
            })
            .bench_values(|(key, data, ttl)| {
                rt.block_on(async {
                    store.update(&key, &data, ttl).await.unwrap();
                });
            });
    }

    #[cfg(feature = "store-redis")]
    #[divan::bench(name = "RedisStore")]
    fn redis_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = rt.block_on(build_redis_store());

        let keys = rt.block_on(populate_store(&store, Simple::sample, ttl_sample, NUM_KEYS));
        let keys_iter = MutexIter::new(keys.into_iter());

        bencher
            .with_inputs(|| {
                let key = keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE);
                let data = Simple::sample();
                let ttl = ttl_sample();
                (key, data, ttl)
            })
            .bench_values(|(key, data, ttl)| {
                rt.block_on(async {
                    store.update(&key, &data, ttl).await.unwrap();
                });
            });
    }
}

#[divan::bench_group(threads = THREADS)]
mod update_ttl {
    use super::*;

    const NUM_KEYS: usize = 2000;

    #[divan::bench(name = "MemoryStore")]
    fn memory_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = MemoryStore::<Simple>::new();

        let keys = rt.block_on(populate_store(&store, Simple::sample, ttl_sample, NUM_KEYS));
        let keys_iter = MutexIter::new(keys.into_iter());

        bencher
            .with_inputs(|| {
                let key = keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE);
                let ttl = ttl_sample();
                (key, ttl)
            })
            .bench_values(|(key, ttl)| {
                rt.block_on(async {
                    store.update_ttl(&key, ttl).await.unwrap();
                });
            });
    }

    #[cfg(feature = "store-redis")]
    #[divan::bench(name = "RedisStore")]
    fn redis_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = rt.block_on(build_redis_store());

        let keys = rt.block_on(populate_store(&store, Simple::sample, ttl_sample, NUM_KEYS));
        let keys_iter = MutexIter::new(keys.into_iter());

        bencher
            .with_inputs(|| {
                let key = keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE);
                let ttl = ttl_sample();
                (key, ttl)
            })
            .bench_values(|(key, ttl)| {
                rt.block_on(async {
                    store.update_ttl(&key, ttl).await.unwrap();
                });
            });
    }
}

#[divan::bench_group(threads = THREADS)]
mod delete {
    use super::*;

    const NUM_KEYS: usize = 2000;

    #[divan::bench(name = "MemoryStore")]
    fn memory_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = MemoryStore::<Simple>::new();

        let keys = rt.block_on(populate_store(&store, Simple::sample, ttl_sample, NUM_KEYS));
        let keys_iter = MutexIter::new(keys.into_iter());

        bencher
            .with_inputs(|| keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE))
            .bench_values(|key| {
                rt.block_on(async {
                    store.delete(&key).await.unwrap();
                });
            });
    }

    #[cfg(feature = "store-redis")]
    #[divan::bench(name = "RedisStore")]
    fn redis_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = rt.block_on(build_redis_store());

        let keys = rt.block_on(populate_store(&store, Simple::sample, ttl_sample, NUM_KEYS));
        let keys_iter = MutexIter::new(keys.into_iter());

        bencher
            .with_inputs(|| keys_iter.next().expect(NUM_KEYS_ERROR_MESSAGE))
            .bench_values(|key| {
                rt.block_on(async {
                    store.delete(&key).await.unwrap();
                });
            });
    }
}

#[allow(dead_code)]
fn build_single_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
}

#[allow(dead_code)]
fn build_multi_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
}

#[cfg(feature = "store-redis")]
async fn build_redis_store<T>() -> RedisStore<T> {
    RedisStore::open((*REDIS_URL).clone()).await.unwrap()
}

async fn populate_store<T, F1, F2>(
    store: &impl SessionStoreImpl<T>,
    data_fn: F1,
    ttl_fn: F2,
    n: usize,
) -> Vec<SessionKey>
where
    F1: Fn() -> T,
    F2: Fn() -> Ttl,
{
    let keys = (1..=n.try_into().unwrap())
        .map(SessionKey::try_from_u128)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for key in &keys {
        store.update(key, &data_fn(), ttl_fn()).await.unwrap();
    }

    keys
}

struct MutexIter<I> {
    iter: Mutex<I>,
}

impl<I, T> MutexIter<I>
where
    I: Iterator<Item = T>,
{
    fn new(iter: I) -> MutexIter<I> {
        let iter = Mutex::new(iter);
        MutexIter { iter }
    }

    #[track_caller]
    fn next(&self) -> Option<T> {
        self.iter.lock().unwrap().next()
    }
}
