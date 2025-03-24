use std::time::Duration;

use futures_util::{stream, StreamExt, TryStreamExt};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng as TestRng;
use time::UtcDateTime;
use tower_sesh_core::{store::SessionStoreRng, SessionKey, SessionStore, Ttl};

pub mod support;
use support::SessionData;

#[doc(hidden)]
pub mod __private {
    pub use paste;
    pub use tokio;
}

macro_rules! doc {
    ($test_suite:item) => {
        /// The `tower-sesh` test suite, which is run for every store implementation.
        ///
        /// This macro takes a single expression after `store: ` as an argument,
        /// which is used to initialize a separate store instance for every test
        /// function. The type of the expression must implement
        /// [`SessionStore`][session-store] and [`SessionStoreRng`].
        ///
        /// [session-store]: tower_sesh_core::store#implementing-sessionstore
        ///
        /// For example, the following macro invocation:
        ///
        /// ```no_run
        /// # use tower_sesh::store::MemoryStore;
        /// # use tower_sesh_test::test_suite;
        /// #
        /// test_suite! {
        ///     store: MemoryStore::new(),
        /// }
        /// ```
        ///
        /// Expands to something like this:
        ///
        /// ```no_run
        /// # use tower_sesh::store::MemoryStore;
        /// #
        /// #[tokio::test]
        /// async fn create_does_collision_resolution() {
        ///     tower_sesh_test::test_create_does_collision_resolution(MemoryStore::new());
        /// }
        ///
        /// #[tokio::test]
        /// async fn loading_session_after_create() {
        ///     tower_sesh_test::test_loading_session_after_create(MemoryStore::new());
        /// }
        ///
        /// // ...rest of test suite...
        /// ```
        ///
        /// # Note on test determinism
        ///
        /// Though each test runs with its own separate store instance, each store
        /// instance may in fact perform operations concurrently on the same database.
        /// For example, in [`tower-sesh-store-redis`]'s test suite, each `RedisStore`
        /// connects to the same Redis server. This won't result in flakiness, since
        /// each test generates unique session keys deterministically.
        ///
        /// [`tower-sesh-store-redis`]: https://docs.rs/tower-sesh-store-redis
        ///
        /// # Examples
        ///
        /// ```no_run
        /// mod memory_store {
        ///     use tower_sesh::store::MemoryStore;
        ///     use tower_sesh_test::test_suite;
        ///
        ///     test_suite! {
        ///         store: MemoryStore::new(),
        ///     }
        /// }
        ///
        /// mod memory_store_caching_store {
        ///     use tower_sesh::store::{CachingStore, MemoryStore};
        ///     use tower_sesh_test::test_suite;
        ///
        ///     test_suite! {
        ///         store: CachingStore::from_cache_and_store(
        ///             MemoryStore::new(),
        ///             MemoryStore::new(),
        ///         ),
        ///     }
        /// }
        /// ```
        ///
        /// A store initializer can also contain `.await`:
        ///
        /// ```no_run
        /// use serde::{de::DeserializeOwned, Serialize};
        /// use tower_sesh_core::{store::SessionStoreRng, SessionStore};
        ///
        /// async fn redis_store<T, Rng>() -> impl SessionStore<T> + SessionStoreRng<Rng>
        /// where
        ///     T: Serialize + DeserializeOwned + Send + Sync + 'static,
        ///     Rng: rand::CryptoRng + Send + 'static,
        /// {
        ///     // ...
        ///     # unimplemented!() as tower_sesh_store_redis::RedisStore<T>
        /// }
        ///
        /// mod normal {
        ///     use tower_sesh_test::test_suite;
        ///
        ///     test_suite! {
        ///         store: redis_store().await,
        ///     }
        /// }
        ///
        /// mod with_caching_store {
        ///     use tower_sesh::store::{CachingStore, MemoryStore};
        ///     use tower_sesh_test::test_suite;
        ///
        ///     test_suite! {
        ///         store: CachingStore::from_cache_and_store(
        ///             MemoryStore::new(),
        ///             redis_store().await,
        ///         ),
        ///     }
        /// }
        /// ```
        #[macro_export]
        $test_suite
    };
}

#[cfg(doc)]
doc! {macro_rules! test_suite {
    (store: $store:expr $(,)?) => { unimplemented!() }
}}

#[cfg(not(doc))]
doc! {macro_rules! test_suite {
    (store : $store:expr $(,)?) => {
        $crate::test_suite! {
            @impl $store => {
                smoke
                create_does_collision_resolution
                loading_session_after_create
                loading_session_after_update_nonexisting
                loading_session_after_update_existing
                loading_session_after_update_ttl
                loading_a_missing_session_returns_none
                loading_an_expired_session_returns_none_create
                loading_an_expired_session_returns_none_update_nonexisting
                loading_an_expired_session_returns_none_update_existing
                loading_an_expired_session_returns_none_update_ttl
                delete_after_create
                delete_after_update
                delete_does_not_error_for_missing_entry
                ttl_with_999_999_999_nanoseconds_create
                ttl_with_999_999_999_nanoseconds_update_nonexisting
                ttl_with_999_999_999_nanoseconds_update_existing
                ttl_with_999_999_999_nanoseconds_update_ttl
                update_ttl_extends_session_that_would_otherwise_expire
                // FIXME: Remove this `ignore` when `MemoryStore` is fixed
                #[ignore = "this test fails with `MemoryStore`"]
                update_ttl_does_not_revive_expired_session
            }
        }
    };

    (
        @impl $store:expr =>
        {
            $(
                $(#[$m:meta])*
                $test:ident
            )+
        }
    ) => {
        $(
            $(#[$m])*
            #[$crate::__private::tokio::test]
            async fn $test() {
                $crate::__private::paste::paste! {
                    $crate::[<test_ $test>]($store).await;
                }
            }
        )+
    };
}}

fn ttl() -> Ttl {
    let now = Ttl::now_local().unwrap();
    ttl_of(now)
}

fn ttl_of(ttl: Ttl) -> Ttl {
    ttl + Duration::from_secs(10 * 60)
}

fn ttl_strict() -> Ttl {
    let now = Ttl::now_local().unwrap();
    ttl_strict_of(now)
}

fn ttl_strict_of(ttl: Ttl) -> Ttl {
    // miri requires a more lenient TTL due to its slower execution speed
    const STRICT_OFFSET: Duration = if cfg!(miri) {
        Duration::from_secs(20)
    } else {
        // NOTE: This threshold may cause spurious test failures on some
        // systems. If that is the case, try increasing this value.
        Duration::from_millis(1500)
    };
    ttl + STRICT_OFFSET
}

trait TtlExt {
    type Normalized;

    fn normalize(self) -> Self::Normalized;
}

impl TtlExt for Ttl {
    type Normalized = UtcDateTime;

    fn normalize(self) -> Self::Normalized {
        self.replace_nanosecond(0).unwrap().to_utc()
    }
}

pub async fn test_smoke(_store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>) {}

pub async fn test_create_does_collision_resolution(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    async fn assert_unique_keys_and_data(
        keys: &[SessionKey],
        store: &impl SessionStore<SessionData>,
    ) {
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

    let test_cases = [1, 2, 3]
        .into_iter()
        .map(SessionData::sample_with)
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

pub async fn test_loading_session_after_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(3005911574);
    store.rng(rng);

    let data = SessionData::sample();
    let ttl = ttl_strict();
    let session_key = store.create(&data, ttl).await.unwrap();

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_loading_session_after_update_nonexisting(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(2848227658);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());

    let data = SessionData::sample();
    let ttl = ttl_strict();
    store.update(&session_key, &data, ttl).await.unwrap();

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_loading_session_after_update_existing(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2280217217);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let data = SessionData::sample();
    let ttl = ttl_strict();
    store.update(&session_key, &data, ttl).await.unwrap();

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_loading_session_after_update_ttl(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(122915542);
    store.rng(rng);
    let data = SessionData::sample();
    let session_key = store.create(&data, ttl()).await.unwrap();

    let ttl = ttl_strict();
    store.update_ttl(&session_key, ttl).await.unwrap();

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_loading_a_missing_session_returns_none(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(999412874);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(31348441);
    store.rng(rng);

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    let session_key = store
        .create(&SessionData::sample(), five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_nonexisting(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(880523847);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    store
        .update(
            &session_key,
            &SessionData::sample(),
            five_microseconds_from_now,
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_existing(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(92143371);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    store
        .update(
            &session_key,
            &SessionData::sample(),
            five_microseconds_from_now,
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_loading_an_expired_session_returns_none_update_ttl(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2587831351);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    store
        .update_ttl(&session_key, five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_after_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(306111374);
    store.rng(rng);

    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();
    store.delete(&session_key).await.unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_after_update(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(200708635);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    store
        .update(&session_key, &SessionData::sample(), ttl())
        .await
        .unwrap();
    store.delete(&session_key).await.unwrap();

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}

pub async fn test_delete_does_not_error_for_missing_entry(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(136113526);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());

    store.delete(&session_key).await.unwrap();
}

fn ttl_edge_case() -> Ttl {
    (Ttl::now_local().unwrap() + Duration::from_secs(10 * 60))
        .replace_nanosecond(1_000_000_000 - 1)
        .unwrap()
}

pub async fn test_ttl_with_999_999_999_nanoseconds_create(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(747720501);
    store.rng(rng);

    let ttl = ttl_edge_case();
    let session_key = store.create(&SessionData::sample(), ttl).await.unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_ttl_with_999_999_999_nanoseconds_update_nonexisting(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let mut rng = TestRng::seed_from_u64(1551031452);
    let session_key = rng.random::<SessionKey>();
    store.rng(rng);

    let ttl = ttl_edge_case();
    store
        .update(&session_key, &SessionData::sample(), ttl)
        .await
        .unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_ttl_with_999_999_999_nanoseconds_update_existing(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2177610229);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let ttl = ttl_edge_case();
    store
        .update(&session_key, &SessionData::sample(), ttl)
        .await
        .unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_ttl_with_999_999_999_nanoseconds_update_ttl(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(337520113);
    store.rng(rng);
    let session_key = store.create(&SessionData::sample(), ttl()).await.unwrap();

    let ttl = ttl_edge_case();
    store.update_ttl(&session_key, ttl).await.unwrap();
    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.ttl.normalize(), ttl.normalize());
}

pub async fn test_update_ttl_extends_session_that_would_otherwise_expire(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(1171023902);
    store.rng(rng);

    let before = Ttl::now_local().unwrap();
    let strict_ttl = ttl_strict_of(before);
    let data = SessionData::sample_with(1171023902);
    let session_key = store.create(&data, strict_ttl).await.unwrap();

    let updated_ttl = ttl();
    store.update_ttl(&session_key, updated_ttl).await.unwrap();

    let sleep_until_duration = strict_ttl - Ttl::now_local().unwrap();
    if sleep_until_duration.is_positive() {
        let sleep_until_duration = sleep_until_duration.unsigned_abs();
        tokio::time::sleep(sleep_until_duration + Duration::from_millis(10)).await;
    }

    let record = store.load(&session_key).await.unwrap().unwrap();
    assert_eq!(record.data, data);
    assert_eq!(record.ttl.normalize(), updated_ttl.normalize());
}

pub async fn test_update_ttl_does_not_revive_expired_session(
    mut store: impl SessionStore<SessionData> + SessionStoreRng<TestRng>,
) {
    let rng = TestRng::seed_from_u64(2495922455);
    store.rng(rng);

    let five_microseconds_from_now = Ttl::now_local().unwrap() + Duration::from_millis(50);
    let session_key = store
        .create(&SessionData::sample(), five_microseconds_from_now)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(90)).await;

    store.update_ttl(&session_key, ttl()).await.unwrap();
    let record = store.load(&session_key).await.unwrap();
    assert!(record.is_none());
}
