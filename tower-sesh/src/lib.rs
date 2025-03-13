#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! A Tower middleware for strongly typed, efficient sessions.
//!
//! **🚧 UNDER CONSTRUCTION 🚧**
//!
//! This crate is being actively developed. Its public API is open to change at
//! any time.
//!
//! To track development of this crate, visit its [GitHub repository].
//!
//! [GitHub repository]: https://github.com/loqusion/tower-sesh

// TODO: Include this in `tower-sesh` docs
#[cfg_attr(not(tower_sesh_docs_local), doc(hidden))]
pub mod _draft {
    //! Top-level documentation draft.
    //!
    //! ## Comparison of session stores
    //!
    //! |                 | Persistent | Horizontally scalable |
    //! |-----------------|------------|-----------------------|
    //! | [`MemoryStore`] | no         | no                    |
    //! | [`RedisStore`]  | yes\*      | yes                   |
    //! | [`SqlxStore`]   | yes        | yes†                  |
    //!
    //! \* Only if [Redis persistence] is enabled.<br>
    //! † Depends on the specific database: SQLite is not horizontally scalable.
    //!
    //! [`MemoryStore`]: crate::store::MemoryStore
    //! [`RedisStore`]: https://docs.rs/tower-sesh-store-redis
    //! [`SqlxStore`]: https://docs.rs/tower-sesh-store-sqlx
    //! [Redis persistence]: https://redis.io/docs/latest/operate/oss_and_stack/management/persistence/
    //!
    //! # Feature flags
    //!
    //! The following crate [feature flags] are available:
    //!
    //! - `axum` *(enabled by default)*: Enables the [`Session`] [extractor]
    //!   (for use with [`axum`]).
    //! - `log`: Causes trace instrumentation points to emit [`log`] records
    //!   (for compatibility with the `log` crate).
    //! - `memory-store` *(enabled by default)*: Enables [`MemoryStore`].
    //! - `tracing` *(enabled by default)*: Enables [`tracing`] output. In order
    //!   to record trace events, you must use a [`Subscriber`] implementation,
    //!   such as one provided by the [`tracing-subscriber`] crate.
    //!   Alternatively, you can enable this crate's `log` feature and use a
    //!   logger compatible with the `log` crate.
    //!
    //! [feature flags]: https://doc.rust-lang.org/cargo/reference/features.html#the-features-section
    //! [`Session`]: crate::Session
    //! [extractor]: https://docs.rs/axum/latest/axum/extract/index.html
    //! [`axum`]: https://docs.rs/axum
    //! [`MemoryStore`]: crate::store::MemoryStore
    //! [`tracing`]: https://docs.rs/tracing
    //! [`Subscriber`]: https://docs.rs/tracing-core/latest/tracing_core/subscriber/trait.Subscriber.html
    //! [`tracing-subscriber`]: https://docs.rs/tracing-subscriber
    //! [`log`]: https://docs.rs/log
}

#[doc(inline)]
pub use middleware::SessionLayer;
#[doc(inline)]
pub use session::Session;

#[macro_use]
mod macros;

pub mod middleware;
pub mod session;
pub mod store;

// Not public API. Items in this module do not follow semantic versioning.
#[doc(hidden)]
pub mod config;

mod util;
