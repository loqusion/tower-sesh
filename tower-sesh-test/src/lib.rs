use std::time::Duration;

use futures::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng as TestRng;
use tower_sesh_core::{store::SessionStoreRng, SessionKey, SessionStore, Ttl};

#[doc(hidden)]
pub mod __private {
    pub use paste;
    pub use tokio;
}

#[macro_export]
macro_rules! test_suite {
    ($store:expr) => {
        $crate::test_suite! {
            @impl $store =>
            smoke create_does_collision_resolution loading_a_missing_session_returns_none
            loading_an_expired_session_returns_none_create
            loading_an_expired_session_returns_none_update
            loading_an_expired_session_returns_none_update_ttl update
            delete_after_create delete_after_update delete_does_not_error_for_missing_entry
        }
    };

    (@impl $store:expr => $($test:ident)+) => {
        $(
            #[$crate::__private::tokio::test]
            async fn $test() {
                $crate::__private::paste::paste! {
                    $crate::[<test_ $test>]($store).await;
                }
            }
        )+
    };
}

pub async fn test_smoke(_store: impl SessionStore<()> + SessionStoreRng<TestRng>) {}

pub async fn test_create_does_collision_resolution(
    mut store: impl SessionStore<String> + SessionStoreRng<TestRng>,
) {
    async fn assert_unique_keys_and_data(keys: &[SessionKey], store: &impl SessionStore<String>) {
        let data = stream::iter(keys.iter().map(|key| store.load(key)))
            .buffered(keys.len())
            .map_ok(|record| record.unwrap().data)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        for (i, (key1, data1)) in keys.iter().zip(data.iter()).enumerate() {
            for (key2, data2) in keys.iter().zip(data.iter()).skip(i + 1) {
                assert_ne!(key1, key2);
                assert_ne!(data1, data2);
            }
        }
    }

    let test_cases = [
        "hello, world!",
        "not hello, world!",
        "another not hello, world!",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    let mut keys: Vec<SessionKey> = Vec::new();

    let rng = TestRng::seed_from_u64(4787236816789423423);
    assert_eq!(rng.clone().random::<SessionKey>(), rng.clone().random()); // sanity check

    for data in test_cases.iter() {
        store.rng(rng.clone());
        let created_key = store.create(data, ttl()).await.unwrap();
        keys.push(created_key);

        assert_unique_keys_and_data(&keys, &store).await;
    }
}

pub async fn test_loading_a_missing_session_returns_none(
    store: impl SessionStore<()> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(999412874);
    let session_key = rng.random::<SessionKey>();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_create(
    mut store: impl SessionStore<()> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(31348441);
    store.rng(rng);

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_micros(5);
    let session_key = store.create(&(), five_microseconds_from_now).await.unwrap();

    tokio::time::sleep(Duration::from_micros(10)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update(
    store: impl SessionStore<()> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(880523847);
    let session_key = rng.random::<SessionKey>();

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_micros(5);
    store
        .update(&session_key, &(), five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_micros(10)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_ttl(
    mut store: impl SessionStore<()> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2587831351);
    store.rng(rng);

    let session_key = store.create(&(), ttl()).await.unwrap();
    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_micros(5);
    store
        .update_ttl(&session_key, five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_micros(10)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_update(store: impl SessionStore<String> + SessionStoreRng<TestRng>) {
    let mut rng = TestRng::seed_from_u64(25593);
    let session_key = rng.random::<SessionKey>();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());

    // creates missing entry
    let before = Ttl::now_local().unwrap();
    store
        .update(&session_key, &"hello world".to_owned(), ttl())
        .await
        .unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, "hello world");
    assert!(record.ttl > before);

    // updates existing entry
    let before = Ttl::now_local().unwrap();
    store
        .update(&session_key, &"another hello world".to_owned(), ttl())
        .await
        .unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, "another hello world");
    assert!(record.ttl > before);
}

pub async fn test_delete_after_create(mut store: impl SessionStore<()> + SessionStoreRng<TestRng>) {
    let rng = TestRng::seed_from_u64(306111374);
    store.rng(rng);

    let session_key = store.create(&(), ttl()).await.unwrap();
    store.delete(&session_key).await.unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_after_update(store: impl SessionStore<()> + SessionStoreRng<TestRng>) {
    let mut rng = TestRng::seed_from_u64(200708635);
    let session_key = rng.random::<SessionKey>();

    store.update(&session_key, &(), ttl()).await.unwrap();
    store.delete(&session_key).await.unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_does_not_error_for_missing_entry(
    store: impl SessionStore<()> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(136113526);
    let session_key = rng.random::<SessionKey>();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());

    store.delete(&session_key).await.unwrap();
}

fn ttl() -> Ttl {
    Ttl::now_local().unwrap() + Duration::from_secs(10)
}
