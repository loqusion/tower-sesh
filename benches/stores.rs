use std::time::Duration;

use divan::black_box;
use serde::{Deserialize, Serialize};
use tower_sesh::store::MemoryStore;
use tower_sesh_core::{
    store::{SessionStoreImpl, Ttl},
    SessionKey,
};

use build_single_rt as build_rt;

const THREADS: &[usize] = &[0, 1, 2, 4, 8, 16];

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
}

#[divan::bench_group(threads = THREADS)]
mod load {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    const NUM_KEYS: usize = 1000;

    #[divan::bench(name = "MemoryStore")]
    fn memory_store(bencher: divan::Bencher) {
        let rt = build_rt();
        let store = MemoryStore::<Simple>::new();
        let data = Simple::sample();
        let ttl = ttl_sample();

        let keys = (1..=NUM_KEYS.try_into().unwrap())
            .map(SessionKey::try_from_u128)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rt.block_on(async {
            for key in &keys {
                store.update(key, &data, ttl).await.unwrap();
            }
        });
        let index = AtomicUsize::new(0);

        bencher
            .with_inputs(|| {
                keys.get(index.fetch_add(1, Ordering::SeqCst))
                    .cloned()
                    .expect(
                        "`NUM_KEYS` is not large enough to cover all iterations\n\
                        lower the iteration count with `sample_count` or `sample_size`, or increase `NUM_KEYS`",
                    )
            })
            .bench_values(|key| {
                rt.block_on(async {
                    let rec = store.load(&key).await.unwrap();
                    black_box(rec);
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
