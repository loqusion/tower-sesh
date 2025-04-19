//! Test suite and utilities for `tower-sesh`.
//!
//! # Usage
//!
//! First, add `tower-sesh-test` to your `dev-dependencies`:
//!
//! ```toml
//! [dev-dependencies]
//! tower-sesh-test = "0.1"
//! ```
//!
//! ## Using the test suite macro
//!
//! The [`test_suite!`] macro expands into multiple test items, and is used to
//! run the test suite for a given store implementation. The expression
//! following `store:` is used to initialize the store in the body of each test,
//! and may include `.await`. The type of the expression must implement
//! [`SessionStore`][session-store] and [`SessionStoreRng`][session-store-rng].
//!
//! [session-store]: tower_sesh_core::store#implementing-sessionstore
//! [session-store-rng]: tower_sesh_core::store::SessionStoreRng
//!
//! For example, the following macro invocation:
//!
//! ```no_run
//! use tower_sesh_test::test_suite;
//!
//! test_suite! {
//!     store: MemoryStore::new(),
//! }
//! ```
//!
//! Expands to something like this:
//!
//! ```no_run
//! #[tokio::test]
//! async fn create_does_collision_resolution() {
//!     let store = MemoryStore::new();
//!     tower_sesh_test::test_create_does_collision_resolution(store);
//! }
//!
//! #[tokio::test]
//! async fn loading_session_after_create() {
//!     let store = MemoryStore::new();
//!     tower_sesh_test::test_loading_session_after_create(store);
//! }
//!
//! // ...rest of test suite...
//! ```
//!
//! It is recommended to define two test suites per store: one using the store
//! alone, and one with [`CachingStore`]:
//!
//! [`CachingStore`]: tower_sesh::store::CachingStore
//!
//! ```ignore
//! mod my_store {
//!     use tower_sesh_test::test_suite;
//!
//!     test_suite! {
//!         store: MyStore::new(),
//!     }
//! }
//!
//! mod my_caching_store {
//!     use tower_sesh::store::{CachingStore, MemoryStore};
//!     use tower_sesh_test::test_suite;
//!
//!     test_suite! {
//!         store: CachingStore::from_cache_and_store(
//!             MemoryStore::new(),
//!             MyStore::new(),
//!         ),
//!     }
//! }
//! ```
//!
//! ### Scope-based resource management
//!
//! Using `guard: <expr>`, you can define a resource tied to a test's lifetime.
//! At the beginning of each test, `<expr>` is evaluated before the expression
//! following `store:`, and its [`Drop`] implementation is run on the test's
//! completion (regardless of success or failure). You can bind this expression
//! to a variable with the syntax `guard: <guard_ident> = <expr>`; this variable
//! can be used in the `store:` expression.
//!
//! For example:
//!
//! ```ignore
//! test_suite! {
//!     guard: file = tempfile().unwrap(),
//!     store: MyStore::new(file),
//! }
//! ```
//!
//! For a more practical example, see [`tower-sesh-store-redis`'s test suite].
//!
//! ### Note on test determinism
//!
//! Ideally, each test should be isolated from every other test so that the
//! success or failure of a test does not depend on execution order.
//! For instance, [`tower-sesh-store-redis`'s test suite] uses
//! [`guard:`](#scope-based-resource-management) to run a unique Redis process
//! for every test.
//!
//! [`tower-sesh-store-redis`'s test suite]:
//!     https://github.com/loqusion/tower-sesh/blob/main/tower-sesh-store-redis/tests/suite.rs
//!
//! In some cases, implementing perfect test isolation may be prohibitively
//! expensive. To account for this possibility, `tower-sesh-test` implements a
//! fallback layer of isolation: _session key uniqueness_. Each test
//! deterministically generates unique session keys, ensuring that no two tests
//! may access the same session concurrently.
//!
//! That being said, if you define two test suites like in the example above
//! with `CachingStore`, they must never be run simultaneously if they run on
//! the same database. Either run one database instance for each test suite, or
//! use [conditional compilation] to run the test suites separately. You can
//! find an example using conditional compilation
//! [here][conditional-compilation-example]
//! (also the [command][conditional-compilation-example-ci]).
//!
//! [conditional compilation]:
//!     https://doc.rust-lang.org/rustc/command-line-arguments.html#--cfg-configure-the-compilation-environment
//! [conditional-compilation-example]:
//!     https://github.com/loqusion/tower-sesh/blob/69e9e1f477a9ae1312d168ddabf8c3932917e43e/tower-sesh-store-redis/tests/suite.rs
//! [conditional-compilation-example-ci]:
//!     https://github.com/loqusion/tower-sesh/blob/69e9e1f477a9ae1312d168ddabf8c3932917e43e/.github/workflows/CI.yml#L168

#![warn(missing_debug_implementations)]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unused_assignments, unused_variables)
    )
))]

pub use suite::*;
pub use support::TestRng;

pub mod suite;
pub mod support;

#[doc(hidden)]
pub mod __private {
    pub use paste;
    pub use tokio;
}

macro_rules! doc {
    ($test_suite:item) => {
        /// The `tower-sesh` test suite, which is run for every store implementation.
        ///
        /// See [the top-level documentation][lib] for more details.
        ///
        /// [lib]: crate#using-the-test-suite-macro
        #[macro_export]
        $test_suite
    };
}

#[cfg(doc)]
doc! {macro_rules! test_suite {
    (guard: $guard_ident:ident = $guard:expr, store: $store:expr $(,)?) => {
        unimplemented!()
    };
    (guard: $guard:expr, store: $store:expr $(,)?) => { unimplemented!() };
    (store: $store:expr $(,)?) => { unimplemented!() };
}}

// To add a test, write a test function in one of `suite`'s submodules meeting
// all of the following requirements:
//
// - The test function's name must begin with `test_`.
//
// - The test function must take a single argument `store` which implements
//   `SessionStore<T>` (for some specific `T`) and `SessionStoreRng<TestRng>`.
//   `T` must satisfy the type constraints for all store implementations —
//   in general, it should implement `Clone`, `Serialize`, and `Deserialize`.
//   Usually, you should use `SessionData` defined in `support`.
//
// - The test function must return a type which implements `Future`, e.g. by
//   using the `async` keyword in front of `fn`. The value returned from the
//   function is `.await`ed then discarded; note that an `Err` returned from a
//   function will cause the test to falsely indicate success.
//
// - The test function must pass a `TestRng` to `store` with the
//   `SessionStoreRng::rng()` method before calling any other methods
//   on `store`. `TestRng` should be instantiated with a unique, fixed seed
//   using the `SeedableRng::seed_from_u64()` method. To acquire a seed, Bash
//   and Zsh users can run `echo $RANDOM`; Fish users can run `random` (run the
//   command more than once and append the two numbers to reduce the risk of
//   collision).
//
// Then, add the name component following `test_` to the list of tests under
// the `// Test Suite` comment.
//
// Tests are grouped by the module they're defined in and sorted in the order
// they appear in that module. For example, if you added a test function named
// `test_does_a_thing` to the `store` module, then `does_a_thing` should be
// added under the `// store` comment.
#[cfg(not(doc))]
doc! {macro_rules! test_suite {
    (guard: $guard_ident:ident = $guard:expr, store: $store:expr $(,)?) => {
        $crate::test_suite! {
            @(guard: $guard_ident = $guard, store: $store) => {
                // Test Suite

                smoke

                // store
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
    (guard: $guard:expr, store: $store:expr $(,)?) => {
        $crate::test_suite! {
            guard: __guard = $guard,
            store: $store,
        }
    };
    (store: $store:expr $(,)?) => {
        $crate::test_suite! {
            guard: (),
            store: $store,
        }
    };

    (
        @(
            guard: $guard_ident:ident = $guard:expr,
            store: $store:expr
        ) => {
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
                let $guard_ident = $guard;
                let __store = $store;
                $crate::__private::paste::paste! {
                    $crate::[<test_ $test>](__store).await;
                }
            }
        )+
    };
}}
