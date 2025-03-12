use std::time::Duration;

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
    let rng = TestRng::seed_from_u64(4787236816789423423);
    let session_key = rng.clone().random::<SessionKey>();
    assert_eq!(session_key, rng.clone().random()); // sanity check

    store
        .update(&session_key, &"hello, world!".to_owned(), ttl())
        .await
        .unwrap();

    store.rng(rng.clone());
    let created_key = store
        .create(&"not hello, world!".to_owned(), ttl())
        .await
        .unwrap();

    assert_ne!(session_key, created_key);
    let original_data = store.load(&session_key).await.unwrap().unwrap().data;
    let created_data = store.load(&created_key).await.unwrap().unwrap().data;
    assert_ne!(original_data, created_data);

    store.rng(rng.clone());
    let another_created_key = store
        .create(&"another not hello, world!".to_owned(), ttl())
        .await
        .unwrap();

    assert_ne!(session_key, another_created_key);
    assert_ne!(created_key, another_created_key);
    let original_data = store.load(&session_key).await.unwrap().unwrap().data;
    let created_data = store.load(&created_key).await.unwrap().unwrap().data;
    let another_created_data = store
        .load(&another_created_key)
        .await
        .unwrap()
        .unwrap()
        .data;
    assert_ne!(original_data, another_created_data);
    assert_ne!(created_data, another_created_data);
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
