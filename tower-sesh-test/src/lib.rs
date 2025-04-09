//! Test suite and utilities for `tower-sesh`.
//!
//! See [`test_suite`] for more details.

pub mod suite;
pub use suite::*;
pub mod support;
pub use support::TestRng;

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
        /// [`SessionStore`][session-store] and [`SessionStoreRng`][session-store-rng].
        ///
        /// [session-store]: tower_sesh_core::store#implementing-sessionstore
        /// [session-store-rng]: tower_sesh_core::store::SessionStoreRng
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
                loading_session_after_create_with_ttl_in_past
                loading_session_after_update_nonexisting_with_ttl_in_past
                loading_session_after_update_existing_with_ttl_in_past
                loading_session_after_update_ttl_with_ttl_in_past
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
