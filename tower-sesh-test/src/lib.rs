use std::time::Duration;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use tower_sesh_core::{time::now, SessionKey, SessionStore};

#[doc(hidden)]
pub use paste;

#[macro_export]
macro_rules! test_suite {
    ($store:expr) => {
        $crate::test_suite! {
            @impl $store =>
            smoke loading_a_missing_session_returns_none update_creates_missing_entry
        }
    };

    (@impl $store:expr => $($test:ident)+) => {
        $(
            #[tokio::test]
            async fn $test() {
                $crate::paste::paste! {
                    $crate::[<test_ $test>]($store).await;
                }
            }
        )+
    };
}

pub async fn test_smoke(_store: impl SessionStore<()>) {}

pub async fn test_loading_a_missing_session_returns_none(store: impl SessionStore<()>) {
    let mut rng = ChaCha20Rng::seed_from_u64(999412874);
    let session_key = rng.random::<SessionKey>();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_update_creates_missing_entry(store: impl SessionStore<String>) {
    let mut rng = ChaCha20Rng::seed_from_u64(56474);
    let session_key = rng.random::<SessionKey>();
    let ttl = now() + Duration::from_secs(10);

    store
        .update(&session_key, &"hello world".to_owned(), ttl)
        .await
        .unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert_eq!(
        record.as_ref().map(|rec| rec.data.as_str()),
        Some("hello world")
    );
}
