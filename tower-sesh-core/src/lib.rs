//! Core of `tower-sesh`.
//!
//! # Warning
//!
//! The API of this crate is not meant for general use and does _not_ follow
//! Semantic Versioning. If you are building a custom session store, you should
//! pin an exact version of `tower-sesh-core` to avoid breakages:
//!
//! ```toml
//! tower-sesh-core = { version = "=X.Y.Z" }
//! ```
//!
//! And then keep releases in sync with `tower-sesh-core`.

#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unexpected_cfgs, unused_assignments, unused_variables)
    )
))]

#[doc(inline)]
pub use crate::key::SessionKey;
#[doc(inline)]
pub use crate::store::{Record, SessionStore};
#[doc(inline)]
pub use crate::time::Ttl;

#[macro_use]
mod macros;

#[doc(hidden)]
pub mod __private {
    #[cfg(feature = "tracing")]
    pub use ::tracing;
}

pub mod key;
pub mod store;
pub mod time;
