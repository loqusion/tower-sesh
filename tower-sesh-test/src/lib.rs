use std::{collections::HashMap, future::Future, hash::Hash, time::Duration};

use futures::FutureExt;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng as TestRng;
use tower_sesh_core::{store::SessionStoreRng, time::now, SessionKey, SessionStore, Ttl};

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
            update_creates_missing_entry
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
    async fn get_or_insert_async<K, V, F>(map: &mut HashMap<K, V>, key: K, future: F) -> &mut V
    where
        K: Eq + Hash,
        F: Future<Output = V>,
    {
        use std::collections::hash_map::Entry;
        match map.entry(key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(future.await),
        }
    }

    async fn assert_unique_keys_and_data(keys: &[SessionKey], store: &impl SessionStore<String>) {
        let load = |key| store.load(key).map(|result| result.unwrap().unwrap().data);
        let mut map: HashMap<&SessionKey, String> = HashMap::new();
        for (i, key1) in keys.iter().enumerate() {
            let data1 = get_or_insert_async(&mut map, key1, load(key1))
                .await
                .to_owned();
            for key2 in keys.iter().skip(i + 1) {
                assert_ne!(key1, key2);
                let data2 = get_or_insert_async(&mut map, key2, load(key2))
                    .await
                    .to_owned();
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

pub async fn test_update_creates_missing_entry(
    store: impl SessionStore<String> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(56474);
    let session_key = rng.random::<SessionKey>();

    store
        .update(&session_key, &"hello world".to_owned(), ttl())
        .await
        .unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert_eq!(
        record.as_ref().map(|rec| rec.data.as_str()),
        Some("hello world")
    );
}

fn ttl() -> Ttl {
    now() + Duration::from_secs(10)
}
