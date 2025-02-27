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
#[cfg_attr(not(localdocs), doc(hidden))]
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
}

#[doc(inline)]
pub use middleware::SessionLayer;
#[doc(inline)]
pub use session::Session;
#[cfg(feature = "value")]
#[doc(inline)]
pub use value::Value;

#[macro_use]
mod macros;

pub mod middleware;
pub mod session;
pub mod store;
#[cfg(feature = "value")]
pub mod value;

// Not public API. Items in this module do not follow semantic versioning.
#[doc(hidden)]
pub mod config;

mod util;
